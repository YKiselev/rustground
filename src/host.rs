use std::collections::HashMap;
use std::env;
use std::ops::Index;

#[derive(Debug)]
pub struct Arguments {
    dedicated: bool,
    windowed: bool,
    base: Vec<String>,
}

impl Arguments {
    fn has_option(v: &Vec<String>, opt: &str) -> bool {
        v.iter().any(|s| {
            *s == opt
        })
    }

    fn get_value<'a>(v: &'a Vec<String>, opt: &str) -> Option<&'a String> {
        v.iter().position(|v| {
            v == opt
        }).map(|idx| {
            &v[idx + 1]
        })
    }

    pub fn new() -> Arguments {
        let args: Vec<String> = env::args().collect();
        let dedicated = Self::has_option(&args, "-dedicated");
        let windowed = Self::has_option(&args, "-windowed");
        let base: Vec<String> = Self::get_value(&args, "-base")
            .unwrap_or(&"".to_string())
            .split(",")
            .map(String::from)
            .collect();
        Arguments {
            dedicated,
            windowed,
            base,
        }
    }
}