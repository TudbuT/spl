use spl::{find_in_splpath, lex, oxidizer::RustAppBuilder, start_file};

use std::{env::args, fs};

fn main() {
    let mut args = args().skip(1);
    let arg = &args
        .next()
        .unwrap_or_else(|| find_in_splpath("repl.spl").expect("no file to be run"));
    if arg == "--build" {
        let file = args.next().unwrap();
        let data = fs::read_to_string(file.clone()).expect("unable to read specified file");
        println!("Building SPL with specified natives file...");
        let mut builder = RustAppBuilder::new();
        println!("Embedding source...");
        builder.add_source(file, data.to_owned());
        println!("Preparing rust code...");
        builder.prepare(lex(data.to_owned()).expect("invalid SPL in natives file."));
        println!("Building...");
        println!("Built! Binary is {}", builder.build().unwrap().get_binary());
        return;
    }
    if let Err(x) = start_file(arg) {
        println!("{x:?}");
    }
}
