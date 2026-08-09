use std::sync::Arc;

use crate::app::App;


pub trait Plugin {
    fn frame_start(&mut self, app: &Arc<App>);
    fn update(&mut self, app: &Arc<App>);
    fn frame_end(&mut self, app: &Arc<App>);
}