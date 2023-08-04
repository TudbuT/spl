use spl::{find_in_splpath, lex, oxidizer::RustAppBuilder, start_file};

use std::{env::args, fs};

fn main() {
    let mut args = args().skip(1);
    let arg = &args
        .next()
        .unwrap_or_else(|| find_in_splpath("repl.spl").expect("no file to be run"));
    if arg == "--build" || arg == "--run" {
        let file = args.next().unwrap();
        let data = fs::read_to_string(file.clone()).expect("unable to read specified file");
        let build_only = arg == "--build";
        if build_only {
            println!("Building SPL with specified natives file...");
        }
        let mut builder = RustAppBuilder::new();
        if build_only {
            if let Some(name) = args.next() {
                builder.set_name(name);
            }
            println!("Embedding source...");
        }
        builder.add_source(file.to_owned(), data.to_owned());
        if build_only {
            println!("Preparing rust code...");
        }
        builder.prepare(lex(data.to_owned()).expect("invalid SPL in natives file."));
        if build_only {
            println!("Building...");
        }
        let app = builder.build(build_only).unwrap();
        if build_only {
            println!("Built! Binary is {}", app.get_binary());
        } else {
            let mut args: Vec<String> = args.collect();
            args.insert(0, file);
            let mut command = app.execute(args).unwrap();
            app.delete();
            command.wait().unwrap();
        }

        return;
    }
    if let Err(x) = start_file(arg) {
        println!("{x:?}");
    }
}
