mod host;

use std::error::Error;
use std::env;
use crate::host::Arguments;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::new();
    println!("Arguments: {:?}", args);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        //assert_eq!(*largest(&[1, 2, 3, 5, 9, 100, 55]), 100);
    }
}