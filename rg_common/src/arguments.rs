use std::env;

#[derive(Debug, Copy, Clone)]
pub struct Arguments {
    dedicated: bool,
    windowed: bool,
}

impl Arguments {
    pub fn dedicated(&self) -> bool {
        self.dedicated
    }

    pub fn windowed(&self) -> bool {
        self.windowed
    }

    fn has_option(v: &Vec<String>, opt: &str) -> bool {
        v.iter().any(|s| *s == opt)
    }

    fn get_value<'a>(v: &'a Vec<String>, opt: &str) -> Option<&'a String> {
        v.iter().position(|v| v == opt).map(|idx| &v[idx + 1])
    }

    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let dedicated = Self::has_option(&args, "--dedicated") || Self::has_option(&args, "-D");
        let windowed = Self::has_option(&args, "--windowed") || Self::has_option(&args, "-W");
        Arguments {
            dedicated,
            windowed,
        }
    }
}
