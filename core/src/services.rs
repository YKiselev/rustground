use crate::arguments::Arguments;
use crate::files::Files;

pub struct Services {
    files: Files,
    //console: Console,
}

impl Services {
    pub fn new(args: &Arguments) -> Self {
        Services {
            files: Files::new(args)
        }
    }
}

struct Console {}