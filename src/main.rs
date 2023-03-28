use spl::{start_file, find_in_splpath};

use std::env::args;

fn main() {
    if let Err(x) = start_file(&args().nth(1).unwrap_or_else(|| find_in_splpath("repl.spl").expect("no file to be run"))) {
        println!("{x:?}");
    }
}
