use spl::{lexer::lex, runtime::*};

use std::{fs, vec};

fn main() -> OError {
    let rt = Runtime::new();
    let mut stack = Stack::new();
    rt.set();
    Words {
        words: vec![
            Word::Key(Keyword::Func(
                "println".to_owned(),
                0,
                Words {
                    words: vec![
                        Word::Call("print".to_owned(), true, 0),
                        Word::Const(Value::Str("\n".to_owned())),
                        Word::Call("print".to_owned(), true, 0),
                    ],
                },
            )),
            Word::Key(Keyword::Def("helloworld".to_owned())),
            Word::Const(Value::Str("Hello, World".to_owned())),
            Word::Call("=helloworld".to_owned(), false, 0),
            Word::Call("helloworld".to_owned(), false, 0),
            Word::Call("println".to_owned(), true, 0),
        ],
    }
    .exec(&mut stack)?;
    let words = lex(
        fs::read_to_string("test.spl").unwrap(),
        "test.spl".to_owned(),
        stack.get_frame(),
    ).map_err(|x| Error {
        kind: ErrorKind::LexError(format!("{x:?}")),
        stack: Vec::new(),
    })?;
    println!("{words:#?}");
    words.exec(&mut stack)?;
    Runtime::reset();
    Ok(())
}
