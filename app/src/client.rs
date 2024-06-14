use log::info;
use core::arguments::Arguments;

pub(crate) struct Client {
}

impl Client {

    pub(crate) fn update(&self) {

    }

    pub(crate) fn new(args: &Arguments) -> Self {
        info!("Starting client...");
        Client{

        }
    }
}