//! LAContext objects stay on one native actor thread. Replies carry no context
//! or credential across the actor boundary, only a private approval receipt.

use block2::RcBlock;
use objc2::{rc::Retained, runtime::Bool};
use objc2_foundation::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAPolicy};
use std::{collections::HashMap, sync::mpsc};
use tauri::async_runtime::{Receiver, Sender, channel};
use uuid::Uuid;

use crate::domain::{AppError, AppResult};

use crate::application::ports::NativeApproval;

enum Job {
    Authenticate {
        id: Uuid,
        reason: String,
        reply: Sender<AppResult<NativeApproval>>,
    },
    Completed {
        id: Uuid,
        accepted: bool,
    },
    Cancel,
}

#[derive(Clone)]
pub struct NativeAuthenticator {
    sender: mpsc::Sender<Job>,
}

impl NativeAuthenticator {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let callback_sender = sender.clone();
        std::thread::Builder::new()
            .name("topotp-native-auth".into())
            .spawn(move || {
                let mut pending: HashMap<
                    Uuid,
                    (Retained<LAContext>, Sender<AppResult<NativeApproval>>),
                > = HashMap::new();
                for job in receiver {
                    match job {
                        Job::Authenticate { id, reason, reply } => {
                            // SAFETY: All contexts are created, used and invalidated
                            // on this thread; callback captures only Send Rust data.
                            let context = unsafe { LAContext::new() };
                            let policy = LAPolicy::DeviceOwnerAuthentication;
                            if unsafe { context.canEvaluatePolicy_error(policy) }.is_err() {
                                let _ = reply.blocking_send(Err(AppError::Cancelled));
                                continue;
                            }
                            unsafe {
                                context.setTouchIDAuthenticationAllowableReuseDuration(0.0);
                                context
                                    .setLocalizedCancelTitle(Some(&NSString::from_str("Cancelar")));
                                context.setLocalizedFallbackTitle(Some(&NSString::from_str(
                                    "Usar senha do macOS",
                                )));
                            }
                            let out = callback_sender.clone();
                            let block =
                                RcBlock::new(move |accepted: Bool, _error: *mut NSError| {
                                    let _ = out.send(Job::Completed {
                                        id,
                                        accepted: accepted.as_bool(),
                                    });
                                });
                            unsafe {
                                context.evaluatePolicy_localizedReason_reply(
                                    policy,
                                    &NSString::from_str(&reason),
                                    &block,
                                );
                            }
                            pending.insert(id, (context, reply));
                        }
                        Job::Completed { id, accepted } => {
                            if let Some((context, reply)) = pending.remove(&id) {
                                unsafe {
                                    context.invalidate();
                                }
                                let result = if accepted {
                                    Ok(NativeApproval::from_native_success())
                                } else {
                                    Err(AppError::Cancelled)
                                };
                                let _ = reply.blocking_send(result);
                            }
                        }
                        Job::Cancel => {
                            for (_, (context, reply)) in pending.drain() {
                                unsafe {
                                    context.invalidate();
                                }
                                let _ = reply.blocking_send(Err(AppError::Cancelled));
                            }
                        }
                    }
                }
            })
            .expect("native authentication actor could not start");
        Self { sender }
    }

    pub async fn authenticate(&self, reason: &str) -> AppResult<NativeApproval> {
        let (reply, mut receiver): (Sender<_>, Receiver<_>) = channel(1);
        self.sender
            .send(Job::Authenticate {
                id: Uuid::new_v4(),
                reason: reason.into(),
                reply,
            })
            .map_err(|_| AppError::Cancelled)?;
        receiver.recv().await.ok_or(AppError::Cancelled)?
    }

    pub fn cancel(&self) {
        let _ = self.sender.send(Job::Cancel);
    }
}

impl Default for NativeAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::application::ports::Authentication for NativeAuthenticator {
    fn authenticate<'a>(
        &'a self,
        reason: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AppResult<NativeApproval>> + Send + 'a>>
    {
        Box::pin(NativeAuthenticator::authenticate(self, reason))
    }
    fn cancel(&self) {
        NativeAuthenticator::cancel(self);
    }
}
