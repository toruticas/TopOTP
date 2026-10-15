use crate::domain::{AppError, AppResult};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::*;
use objc2_foundation::{
    MainThreadMarker, NSNumber, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub const MAX_PIXELS: usize = 16_000_000;
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub display_id: u32,
    pub source_rect: NSRect,
    pub width: usize,
    pub height: usize,
}
pub fn region(
    display_id: u32,
    size: NSSize,
    scale: f64,
    start: NSPoint,
    end: NSPoint,
) -> AppResult<Region> {
    if ![
        size.width,
        size.height,
        scale,
        start.x,
        start.y,
        end.x,
        end.y,
    ]
    .iter()
    .all(|v| v.is_finite())
        || size.width <= 0.
        || size.height <= 0.
        || !(1.0..=4.0).contains(&scale)
    {
        return Err(AppError::invalid("region"));
    }
    let start = NSPoint::new(
        start.x.clamp(0., size.width),
        start.y.clamp(0., size.height),
    );
    let end = NSPoint::new(end.x.clamp(0., size.width), end.y.clamp(0., size.height));
    let x = start.x.min(end.x).floor();
    let top = (size.height - start.y.max(end.y)).floor();
    let width = (start.x - end.x).abs().ceil();
    let height = (start.y - end.y).abs().ceil();
    let pixel_width = (width * scale).ceil() as usize;
    let pixel_height = (height * scale).ceil() as usize;
    if width < 4. || height < 4. {
        return Err(AppError::Cancelled);
    }
    if pixel_width
        .checked_mul(pixel_height)
        .is_none_or(|pixels| pixels > MAX_PIXELS)
    {
        return Err(AppError::LimitExceeded);
    }
    Ok(Region {
        display_id,
        source_rect: NSRect::new(NSPoint::new(x, top), NSSize::new(width, height)),
        width: pixel_width,
        height: pixel_height,
    })
}
struct Selection {
    windows: Vec<Retained<OverlayWindow>>,
    callback: Rc<dyn Fn(AppResult<Region>)>,
}
thread_local! {static SELECTION:RefCell<Option<Selection>>=const{RefCell::new(None)};}
struct ViewState {
    display_id: u32,
    scale: f64,
    start: Cell<Option<NSPoint>>,
    end: Cell<Option<NSPoint>>,
}
define_class!(
    // SAFETY: Borderless NSWindow subclass, retained until selection ends.
    #[unsafe(super=NSWindow)]
    #[thread_kind=MainThreadOnly]
    struct OverlayWindow;
    impl OverlayWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self)->bool{true}
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self)->bool{false}
    }
);
define_class!(
    // SAFETY: All view state and callbacks are confined to AppKit's main thread.
    #[unsafe(super=NSView)]
    #[thread_kind=MainThreadOnly]
    #[ivars=ViewState]
    struct OverlayView;
    unsafe impl NSObjectProtocol for OverlayView {}
    impl OverlayView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first(&self)->bool{true}
        #[unsafe(method(resetCursorRects))]
        fn cursor(&self){self.addCursorRect_cursor(self.bounds(),&NSCursor::crosshairCursor());}
        #[unsafe(method(mouseDown:))]
        fn down(&self,event:&NSEvent){let point=self.convertPoint_fromView(event.locationInWindow(),None);self.ivars().start.set(Some(point));self.ivars().end.set(Some(point));self.setNeedsDisplay(true);}
        #[unsafe(method(mouseDragged:))]
        fn drag(&self,event:&NSEvent){self.ivars().end.set(Some(self.convertPoint_fromView(event.locationInWindow(),None)));self.setNeedsDisplay(true);}
        #[unsafe(method(mouseUp:))]
        fn up(&self,event:&NSEvent){
            // SAFETY: Retain the receiver while finish removes its owning window.
            let _keep=unsafe{Retained::retain(self as *const Self as *mut Self)}.unwrap();
            let end=self.convertPoint_fromView(event.locationInWindow(),None);
            let result=self.ivars().start.get().ok_or(AppError::Cancelled).and_then(|start|region(self.ivars().display_id,self.bounds().size,self.ivars().scale,start,end));
            finish(result);
        }
        #[unsafe(method(keyDown:))]
        fn key(&self,event:&NSEvent){if event.keyCode()==53 {let _keep=unsafe{Retained::retain(self as *const Self as *mut Self)}.unwrap();finish(Err(AppError::Cancelled));}}
        #[unsafe(method(drawRect:))]
        fn draw(&self,_:NSRect){
            NSColor::colorWithSRGBRed_green_blue_alpha(0.,0.,0.,0.24).setFill();
            NSBezierPath::fillRect(self.bounds());
            if let (Some(start),Some(end))=(self.ivars().start.get(),self.ivars().end.get()){
                NSColor::whiteColor().setStroke();
                let path=NSBezierPath::bezierPathWithRect(NSRect::new(NSPoint::new(start.x.min(end.x),start.y.min(end.y)),NSSize::new((start.x-end.x).abs(),(start.y-end.y).abs())));
                path.setLineWidth(2.);path.stroke();
            }
        }
    }
);
pub fn start(callback: Rc<dyn Fn(AppResult<Region>)>) -> AppResult<()> {
    let mtm = MainThreadMarker::new().ok_or(AppError::CaptureUnavailable)?;
    if SELECTION.with(|s| s.borrow().is_some()) {
        return Err(AppError::Conflict);
    }
    let screens = NSScreen::screens(mtm)
        .into_iter()
        .map(|screen| {
            let description = screen.deviceDescription();
            let number = description
                .objectForKey(&NSString::from_str("NSScreenNumber"))
                .and_then(|object| object.downcast::<NSNumber>().ok())
                .ok_or(AppError::CaptureUnavailable)?;
            Ok((
                number.unsignedIntValue(),
                screen.frame(),
                screen.backingScaleFactor(),
            ))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let mut windows = Vec::new();
    for (display_id, frame, scale) in screens {
        let view = OverlayView::alloc(mtm).set_ivars(ViewState {
            display_id,
            scale,
            start: Cell::new(None),
            end: Cell::new(None),
        });
        let view: Retained<OverlayView> = unsafe {
            msg_send![super(view),initWithFrame:NSRect::new(NSPoint::new(0.,0.),frame.size)]
        };
        let window: Retained<OverlayWindow> = unsafe {
            msg_send![OverlayWindow::alloc(mtm),initWithContentRect:frame,styleMask:NSWindowStyleMask::Borderless,backing:NSBackingStoreType::Buffered,defer:false]
        };
        // SAFETY: Rust owns the window; close must not release an extra retain.
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setLevel(NSStatusWindowLevel + 1);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        window.setContentView(Some(&view));
        window.makeKeyAndOrderFront(None);
        window.makeFirstResponder(Some(&view));
        windows.push(window);
    }
    if windows.is_empty() {
        return Err(AppError::CaptureUnavailable);
    }
    SELECTION.with(|slot| *slot.borrow_mut() = Some(Selection { windows, callback }));
    Ok(())
}
fn finish(result: AppResult<Region>) {
    let selection = SELECTION.with(|slot| slot.borrow_mut().take());
    if let Some(selection) = selection {
        for window in &selection.windows {
            window.orderOut(None);
            window.close();
        }
        (selection.callback)(result);
    }
}
pub fn cancel() {
    finish(Err(AppError::Cancelled));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retina_region_flips_y_and_scales_output_without_double_scaling_source() {
        let r = region(
            42,
            NSSize::new(1512., 982.),
            2.,
            NSPoint::new(100., 700.),
            NSPoint::new(300., 900.),
        )
        .unwrap();
        assert_eq!(r.display_id, 42);
        assert_eq!(
            r.source_rect,
            NSRect::new(NSPoint::new(100., 82.), NSSize::new(200., 200.))
        );
        assert_eq!((r.width, r.height), (400, 400));
    }
    #[test]
    fn drag_stays_on_original_display_and_rejects_excessive_or_empty_regions() {
        let r = region(
            4,
            NSSize::new(1920., 1080.),
            1.,
            NSPoint::new(200., 900.),
            NSPoint::new(-400., 1200.),
        )
        .unwrap();
        assert_eq!(
            r.source_rect,
            NSRect::new(NSPoint::new(0., 0.), NSSize::new(200., 180.))
        );
        assert!(
            region(
                1,
                NSSize::new(8000., 8000.),
                2.,
                NSPoint::new(0., 0.),
                NSPoint::new(8000., 8000.)
            )
            .is_err()
        );
        assert!(
            region(
                1,
                NSSize::new(100., 100.),
                1.,
                NSPoint::new(20., 20.),
                NSPoint::new(21., 22.)
            )
            .is_err()
        );
    }
}
