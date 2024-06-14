use log::info;
use core::arguments::Arguments;


pub(crate) struct Server {
}

impl Server {

    pub(crate) fn update(&self){
        // todo
    }

    pub fn new(args: &Arguments) ->Self{
        info!("Starting server...");
        Server {
        }
    }
}
