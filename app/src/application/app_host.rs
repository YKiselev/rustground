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
static EVENT_PROXY: OnceLock<EventLoopProxy<ClientEvent>> = OnceLock::new();

pub(crate) fn is_exit() -> bool {
    EXIT_FLAG.load(Ordering::Relaxed)
}

pub(crate) fn trigger_exit() {
    if let Ok(_) = EXIT_FLAG.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire) {
        if let Some(proxy) = EVENT_PROXY.get() {
            let _ = proxy.send_event(ClientEvent::Exiting);
        } else {
            warn!("Event proxy is not set!");
        }
    }
}

pub(crate) fn set_event_proxy(proxy: EventLoopProxy<ClientEvent>) {
    let _ = EVENT_PROXY.set(proxy).unwrap();
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
