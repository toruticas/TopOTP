//! One frame, decoded locally. Native callback objects never reach the webview.
use super::selection::{MAX_PIXELS, Region};
use crate::{
    application::scan::{MAX_QR_CODES, MAX_QR_PAYLOAD},
    domain::{AppError, AppResult},
};
use block2::RcBlock;
use objc2::{
    AnyThread, DefinedClass, define_class, msg_send, rc::Retained, runtime::ProtocolObject,
};
use objc2_core_graphics::{CGImage, CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use objc2_core_media::CMSampleBuffer;
use objc2_foundation::{
    NSArray, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSProcessInfo, NSString,
};
use objc2_screen_capture_kit::*;
use objc2_vision::*;
use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
type Payloads = Vec<Zeroizing<String>>;

pub fn request_permission() -> AppResult<()> {
    permission(
        || CGPreflightScreenCaptureAccess(),
        || CGRequestScreenCaptureAccess(),
    )
}
fn permission(preflight: impl FnOnce() -> bool, request: impl FnOnce() -> bool) -> AppResult<()> {
    if preflight() || request() {
        Ok(())
    } else {
        Err(AppError::ScreenCaptureDenied)
    }
}
struct Capture {
    result: Mutex<Option<mpsc::Sender<AppResult<Payloads>>>>,
    cancelled: Arc<AtomicBool>,
    finished: AtomicBool,
    stream: Mutex<Option<StreamHandles>>,
}
struct StreamHandles {
    stream: Retained<SCStream>,
    _output: Retained<Output>,
}
// SAFETY: This wrapper exposes no native handles. SCStream's asynchronous
// start/stop API is not main-thread-bound. Capture serializes ownership through
// its mutex; Output is AnyThread and its ivars contain only Weak/AtomicBool.
// No AppKit or LocalAuthentication object is stored or transferred here.
unsafe impl Send for StreamHandles {}
impl Capture {
    fn finish(&self, result: AppResult<Payloads>) {
        self.finished.store(true, Ordering::SeqCst);
        let sender = self.result.lock().unwrap().take();
        if let Some(sender) = sender {
            let _ = sender.send(result);
        }
        let stream = self.stream.lock().unwrap().take();
        // SAFETY: ScreenCaptureKit supports asynchronous start/stop; the retained
        // handles are serialized by these mutexes and no AppKit objects occur here.
        if let Some(handles) = stream {
            unsafe {
                handles.stream.stopCaptureWithCompletionHandler(None);
            }
        }
    }
}
struct OutputState {
    capture: Weak<Capture>,
    received: AtomicBool,
}
define_class!(
    // SAFETY: Only thread-safe Rust state is read directly in frame callbacks;
    // retained ScreenCaptureKit handles are protected by Capture's mutexes.
    #[unsafe(super=NSObject)]
    #[thread_kind=AnyThread]
    #[ivars=OutputState]
    struct Output;
    unsafe impl NSObjectProtocol for Output {}
    unsafe impl SCStreamOutput for Output {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn frame(&self, _: &SCStream, sample: &CMSampleBuffer, kind: SCStreamOutputType) {
            if kind != SCStreamOutputType::Screen {
                return;
            }
            let Some(capture) = self.ivars().capture.upgrade() else {
                return;
            };
            if capture.cancelled.load(Ordering::SeqCst) {
                capture.finish(Err(AppError::Cancelled));
                return;
            }
            // SAFETY: The sample buffer is valid for this callback; Vision finishes
            // synchronously before it is released. The view/core receive no image.
            let Some(buffer) = (unsafe { sample.image_buffer() }) else {
                return;
            };
            let Some(attachments) = (unsafe { sample.sample_attachments_array(false) }) else {
                return;
            };
            // SAFETY: The documented sample attachment array/dictionaries are
            // toll-free bridged Foundation containers, borrowed for this frame.
            let attachments = unsafe {
                objc2_core_foundation::CFRetained::as_ptr(&attachments)
                    .cast::<NSArray<NSDictionary<NSString, NSObject>>>()
                    .as_ref()
            };
            let complete = attachments
                .firstObject()
                .and_then(|info| info.objectForKey(unsafe { SCStreamFrameInfoStatus }))
                .and_then(|status| status.downcast::<NSNumber>().ok())
                .is_some_and(|status| status.integerValue() == SCFrameStatus::Complete.0);
            if !complete {
                return;
            }
            if self.ivars().received.swap(true, Ordering::SeqCst) {
                return;
            }
            let _keep = unsafe { Retained::retain(self as *const Self as *mut Self) }.unwrap();
            let result = objc2::rc::autoreleasepool(|_| unsafe {
                let handler = VNImageRequestHandler::initWithCVPixelBuffer_options(
                    VNImageRequestHandler::alloc(),
                    &buffer,
                    &NSDictionary::new(),
                );
                decode(&handler, &capture.cancelled)
            });
            capture.finish(result);
        }
    }
);

unsafe fn decode(handler: &VNImageRequestHandler, cancelled: &AtomicBool) -> AppResult<Payloads> {
    if cancelled.load(Ordering::SeqCst) {
        return Err(AppError::Cancelled);
    }
    // SAFETY: Objects are confined to this synchronous Vision request. The QR
    // symbol is an immutable framework constant available on supported macOS.
    let request = unsafe { VNDetectBarcodesRequest::new() };
    let qr = unsafe { VNBarcodeSymbologyQR }.ok_or(AppError::CaptureUnavailable)?;
    unsafe {
        request.setSymbologies(&NSArray::from_slice(&[qr]));
    }
    let requests: Retained<NSArray<VNRequest>> = NSArray::from_slice(&[&*request]);
    handler
        .performRequests_error(&requests)
        .map_err(|_| AppError::InvalidQrCode)?;
    if cancelled.load(Ordering::SeqCst) {
        return Err(AppError::Cancelled);
    }
    let results = unsafe { request.results() }.ok_or(AppError::NoQrCode)?;
    if results.is_empty() {
        return Err(AppError::NoQrCode);
    }
    if results.len() > MAX_QR_CODES {
        return Err(AppError::LimitExceeded);
    }
    let mut payloads = Vec::new();
    for observation in results {
        let payload = unsafe { observation.payloadStringValue() };
        if let Some(payload) = payload {
            if payload.length() > MAX_QR_PAYLOAD {
                return Err(AppError::LimitExceeded);
            }
            let payload = Zeroizing::new(payload.to_string());
            if payload.len() > MAX_QR_PAYLOAD {
                return Err(AppError::LimitExceeded);
            }
            payloads.push(payload);
        } else {
            payloads.push(Zeroizing::new(String::new()));
        }
    }
    Ok(payloads)
}
pub fn image_payloads(image: &CGImage, cancelled: &AtomicBool) -> AppResult<Payloads> {
    if CGImage::width(Some(image))
        .checked_mul(CGImage::height(Some(image)))
        .is_none_or(|pixels| pixels > MAX_PIXELS)
    {
        return Err(AppError::LimitExceeded);
    }
    objc2::rc::autoreleasepool(|_| unsafe {
        let handler = VNImageRequestHandler::initWithCGImage_options(
            VNImageRequestHandler::alloc(),
            image,
            &NSDictionary::new(),
        );
        decode(&handler, cancelled)
    })
}
pub fn capture(
    region: Region,
    cancelled: Arc<AtomicBool>,
    authorized: impl Fn() -> bool,
) -> AppResult<Payloads> {
    if !authorized() {
        return Err(AppError::Locked);
    }
    if region
        .width
        .checked_mul(region.height)
        .is_none_or(|pixels| pixels == 0 || pixels > MAX_PIXELS)
    {
        return Err(AppError::LimitExceeded);
    }
    let (sender, receiver) = mpsc::channel();
    let capture = Arc::new(Capture {
        result: Mutex::new(Some(sender)),
        cancelled: cancelled.clone(),
        finished: AtomicBool::new(false),
        stream: Mutex::new(None),
    });
    let shared = capture.clone();
    let block = RcBlock::new(
        move |content: *mut SCShareableContent, error: *mut NSError| {
            if shared.cancelled.load(Ordering::SeqCst) {
                return;
            }
            // SAFETY: Framework callback pointers are borrowed only for this callback.
            // Only Capture's synchronized state is shared between callbacks.
            let Some(content) = (unsafe { content.as_ref() }) else {
                shared.finish(Err(AppError::CaptureUnavailable));
                return;
            };
            if !error.is_null() {
                shared.finish(Err(AppError::CaptureUnavailable));
                return;
            }
            objc2::rc::autoreleasepool(|_| unsafe {
                let Some(display) = content
                    .displays()
                    .into_iter()
                    .find(|display| display.displayID() == region.display_id)
                else {
                    shared.finish(Err(AppError::CaptureUnavailable));
                    return;
                };
                let filter = SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &NSArray::new(),
                );
                let config = SCStreamConfiguration::new();
                config.setWidth(region.width);
                config.setHeight(region.height);
                config.setSourceRect(region.source_rect);
                config.setShowsCursor(false);
                config.setCapturesAudio(false);
                config.setQueueDepth(3);
                if NSProcessInfo::processInfo()
                    .operatingSystemVersion()
                    .majorVersion
                    >= 14
                {
                    let done = shared.clone();
                    let reply = RcBlock::new(move |image: *mut CGImage, error: *mut NSError| {
                        let result = if done.cancelled.load(Ordering::SeqCst) {
                            Err(AppError::Cancelled)
                        } else if !error.is_null() {
                            Err(AppError::CaptureUnavailable)
                        } else if let Some(image) = image.as_ref() {
                            image_payloads(image, &done.cancelled)
                        } else {
                            Err(AppError::CaptureUnavailable)
                        };
                        done.finish(result);
                    });
                    SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
                        &filter,
                        &config,
                        Some(&reply),
                    );
                } else {
                    let output = Output::alloc().set_ivars(OutputState {
                        capture: Arc::downgrade(&shared),
                        received: AtomicBool::new(false),
                    });
                    let output: Retained<Output> = msg_send![super(output), init];
                    let stream = SCStream::initWithFilter_configuration_delegate(
                        SCStream::alloc(),
                        &filter,
                        &config,
                        None,
                    );
                    let queue =
                        dispatch2::DispatchQueue::new("com.rafaelmariano.topotp.scan.frame", None);
                    if stream
                        .addStreamOutput_type_sampleHandlerQueue_error(
                            ProtocolObject::from_ref(&*output),
                            SCStreamOutputType::Screen,
                            Some(&queue),
                        )
                        .is_err()
                    {
                        shared.finish(Err(AppError::CaptureUnavailable));
                        return;
                    }
                    *shared.stream.lock().unwrap() = Some(StreamHandles {
                        stream: stream.clone(),
                        _output: output,
                    });
                    let done = shared.clone();
                    let reply = RcBlock::new(move |error: *mut NSError| {
                        if !error.is_null() {
                            done.finish(Err(AppError::CaptureUnavailable));
                        }
                    });
                    stream.startCaptureWithCompletionHandler(Some(&reply));
                    // Cancellation may race with asynchronous content discovery.
                    // Stop again if setup completed after the waiter already left.
                    if shared.finished.load(Ordering::SeqCst)
                        || shared.cancelled.load(Ordering::SeqCst)
                    {
                        shared.finish(Err(AppError::Cancelled));
                        stream.stopCaptureWithCompletionHandler(None);
                    }
                }
            });
        },
    );
    // SAFETY: The framework retains this block. Callback payloads are temporary;
    // shared Rust state is synchronized and all native handles are retained.
    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(false,true,&block);
    }
    wait(&capture, &receiver, authorized, Duration::from_secs(15))
}
fn wait(
    capture: &Capture,
    receiver: &mpsc::Receiver<AppResult<Payloads>>,
    authorized: impl Fn() -> bool,
    timeout: Duration,
) -> AppResult<Payloads> {
    let started = Instant::now();
    loop {
        if capture.cancelled.load(Ordering::SeqCst) || !authorized() {
            capture.cancelled.store(true, Ordering::SeqCst);
            capture.finish(Err(AppError::Cancelled));
            return Err(AppError::Cancelled);
        }
        if started.elapsed() >= timeout {
            capture.cancelled.store(true, Ordering::SeqCst);
            capture.finish(Err(AppError::CaptureTimedOut));
            return Err(AppError::CaptureTimedOut);
        }
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => {
                if !authorized() || capture.cancelled.load(Ordering::SeqCst) {
                    return Err(AppError::Locked);
                }
                return result;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => return Err(AppError::CaptureUnavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_core_foundation::{CFData, CFRetained};
    use objc2_core_graphics::{CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider};
    use qrcodegen::{QrCode, QrCodeEcc};

    fn image(payloads: &[&str]) -> CFRetained<CGImage> {
        let codes: Vec<_> = payloads
            .iter()
            .map(|payload| QrCode::encode_text(payload, QrCodeEcc::Medium).unwrap())
            .collect();
        let tile = (codes.iter().map(|code| code.size()).max().unwrap_or(21) as usize + 8) * 6;
        let width = tile * codes.len().max(1);
        let height = tile;
        let mut pixels = vec![255u8; width * height];
        for (index, code) in codes.iter().enumerate() {
            for y in 0..code.size() {
                for x in 0..code.size() {
                    if code.get_module(x, y) {
                        for dy in 0..6 {
                            for dx in 0..6 {
                                pixels[((y as usize + 4) * 6 + dy) * width
                                    + index * tile
                                    + (x as usize + 4) * 6
                                    + dx] = 0;
                            }
                        }
                    }
                }
            }
        }
        let data = CFData::from_bytes(&pixels);
        let provider = CGDataProvider::with_cf_data(Some(&data)).unwrap();
        let space = CGColorSpace::new_device_gray().unwrap();
        // SAFETY: The immutable provider owns exactly width*height grayscale bytes.
        unsafe {
            CGImage::new(
                width,
                height,
                8,
                8,
                width,
                Some(&space),
                CGBitmapInfo::empty(),
                Some(&provider),
                std::ptr::null(),
                false,
                CGColorRenderingIntent::RenderingIntentDefault,
            )
        }
        .unwrap()
    }
    #[test]
    fn native_vision_reads_synthetic_totp_hotp_and_unicode_without_capture_permission() {
        let totp =
            "otpauth://totp/Sint%C3%A9tico:alice?secret=JBSWY3DPEHPK3PXP&issuer=Sint%C3%A9tico";
        let hotp = "otpauth://hotp/test?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=7";
        let result = image_payloads(&image(&[totp, hotp]), &AtomicBool::new(false)).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|value| value.as_str() == totp));
        assert!(result.iter().any(|value| value.as_str() == hotp));
    }
    #[test]
    fn native_vision_distinguishes_no_qr_and_cancellation() {
        let blank = image(&[]);
        assert!(matches!(
            image_payloads(&blank, &AtomicBool::new(false)),
            Err(AppError::NoQrCode)
        ));
        assert!(matches!(
            image_payloads(&blank, &AtomicBool::new(true)),
            Err(AppError::Cancelled)
        ));
    }
    #[test]
    fn unauthorized_capture_never_requests_content_or_permission() {
        let region = Region {
            display_id: 0,
            source_rect: objc2_foundation::NSRect::ZERO,
            width: 10,
            height: 10,
        };
        assert!(matches!(
            capture(region, Arc::new(AtomicBool::new(false)), || false),
            Err(AppError::Locked)
        ));
    }
    #[test]
    fn permission_denial_is_one_request_and_an_existing_grant_skips_the_prompt() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        assert!(matches!(
            permission(
                || false,
                || {
                    calls.set(calls.get() + 1);
                    false
                }
            ),
            Err(AppError::ScreenCaptureDenied)
        ));
        assert_eq!(calls.get(), 1);
        assert!(permission(|| true, || panic!("must not prompt")).is_ok());
    }
    #[test]
    fn capture_waiter_discards_timeout_cancelled_and_revoked_results() {
        fn state() -> (Capture, mpsc::Receiver<AppResult<Payloads>>) {
            let (sender, receiver) = mpsc::channel();
            (
                Capture {
                    result: Mutex::new(Some(sender)),
                    cancelled: Arc::new(AtomicBool::new(false)),
                    finished: AtomicBool::new(false),
                    stream: Mutex::new(None),
                },
                receiver,
            )
        }
        let (capture, receiver) = state();
        assert!(matches!(
            wait(&capture, &receiver, || true, Duration::ZERO),
            Err(AppError::CaptureTimedOut)
        ));
        assert!(capture.finished.load(Ordering::SeqCst));
        let (capture, receiver) = state();
        capture.cancelled.store(true, Ordering::SeqCst);
        assert!(matches!(
            wait(&capture, &receiver, || true, Duration::from_secs(1)),
            Err(AppError::Cancelled)
        ));
        let (capture, receiver) = state();
        capture.finish(Ok(vec![Zeroizing::new("synthetic payload".into())]));
        let calls = AtomicBool::new(false);
        assert!(matches!(
            wait(
                &capture,
                &receiver,
                || !calls.swap(true, Ordering::SeqCst),
                Duration::from_secs(1)
            ),
            Err(AppError::Locked)
        ));
    }
}
