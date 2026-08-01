use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use rg_common::{App, Arguments};
use tracing::{error, warn};
use winit::event_loop::EventLoopProxy;

use crate::client::ClientEvent;

use super::app_commands::AppCommands;

static EXIT_FLAG: AtomicBool = AtomicBool::new(false);
static EVENT_PROXY: Mutex<Option<EventLoopProxy<ClientEvent>>> = Mutex::new(None);

pub(crate) fn is_exit() -> bool {
    EXIT_FLAG.load(Ordering::Relaxed)
}

pub(crate) fn trigger_exit() {
    if let Ok(_) = EXIT_FLAG.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire) {
        if let Ok(guard) = EVENT_PROXY.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(ClientEvent::Exiting);
            }
        } else {
            warn!("Event proxy mutex is poisoned!");
        }
    }
}

pub(crate) fn set_event_proxy(proxy: EventLoopProxy<ClientEvent>) {
    let mut guard = EVENT_PROXY.lock().unwrap();
    *guard = Some(proxy);
}

pub struct AppHost {
    pub app: Arc<App>,
    _commands: AppCommands,
}

impl AppHost {
    pub fn new(args: Arguments) -> Self {
        let app = Arc::new(App::new(args));
        let commands = AppCommands::new(Arc::clone(&app))
            .inspect_err(|e| error!("Unable to register commands: {}", e))
            .unwrap();
        Self {
            app,
            _commands: commands,
        }
    }
}
