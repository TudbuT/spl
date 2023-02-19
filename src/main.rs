use spl::{lexer::lex, runtime::*};

use std::fs;

fn main() -> OError {
    let rt = Runtime::new();
    rt.set();
    let mut stack = Stack::new_in(FrameInfo {
        file: "std.spl".to_owned(),
        function: "root".to_owned(),
    });
    let words = lex(fs::read_to_string("std.spl").unwrap()).map_err(|x| Error {
        kind: ErrorKind::LexError(format!("{x:?}")),
        stack: Vec::new(),
    })?;
    words.exec(&mut stack)?;
    Runtime::reset();
    Ok(())
}
