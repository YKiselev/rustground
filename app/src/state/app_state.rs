use crate::app::App;

pub(crate) trait AppState {
    fn try_advance(&self, app: &mut App) -> anyhow::Result<Option<Box<dyn AppState>>>;
}