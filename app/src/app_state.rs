use crate::app::App;

pub(crate) trait AppState {
    fn try_advance(&self, app:&mut App) -> anyhow::Result<Option<Box<dyn AppState>>>;
}

#[cfg(test)]
mod test {
    use crate::app_state::AppState;
    use crate::as_init::InitialState;

    #[test]
    fn test_transitions() {
        let mut state: Box<dyn AppState> = Box::new(InitialState::default());
        loop {
            match state.try_advance() {
                Ok(Some(s)) => {
                    println!("State transition");
                    state = s;
                }
                Ok(None) => {
                    // not ready yet
                }
                Err(e) => {
                    println!("Got error: {}", e);
                    break;
                }
            }
        }
    }
}