use spl::{runtime::*, start_file};

use std::env::args;

fn main() {
    if let Err(x) = start_file(&args().nth(1).expect("no file provided to be started.")) {
        println!("{x:?}");
    }
}
