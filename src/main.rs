use spl::{find_in_splpath, start_file};

use std::env::args;

fn main() {
    if let Err(x) = start_file(
        &args()
            .nth(1)
            .unwrap_or_else(|| find_in_splpath("repl.spl").expect("no file to be run")),
    ) {
        println!("{x:?}");
    }
}
