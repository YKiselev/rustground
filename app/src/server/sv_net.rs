use super::{sv_error::ServerError, sv_poll::ServerPollThread};



struct ServerNet {
    poll_thread: ServerPollThread
}

impl ServerNet {

    pub(crate) fn new() -> Result<ServerNet, ServerError> {
        unimplemented!()
    }
    

    fn update(&mut self) {
        
    }
}