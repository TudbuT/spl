use spl::{lexer::lex, runtime::*};

use std::{
    io::{stdout, Write},
    sync::Arc,
    vec,
};

fn main() {
    let rt = Runtime::new();
    let mut stack = Stack::new();
    fn print(stack: &mut Stack) {
        let s = stack.pop();
        let s = s.lock();
        if let Constant::Str(ref s) = s.native {
            print!("{s}");
            stdout().lock().flush().unwrap();
        }
    }
    stack.define_func(
        "print".to_owned(),
        Arc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(print),
            origin: FrameInfo {
                file: "RUNTIME".to_owned(),
            },
        }),
    );
    rt.set();
    Words {
        words: vec![
            Word::Key(Keyword::Func(
                "println".to_owned(),
                0,
                Words {
                    words: vec![
                        Word::Call("print".to_owned(), true, 0),
                        Word::Const(Constant::Str("\n".to_owned())),
                        Word::Call("print".to_owned(), true, 0),
                    ],
                },
            )),
            Word::Key(Keyword::Def("helloworld".to_owned())),
            Word::Const(Constant::Str("Hello, World".to_owned())),
            Word::Call("=helloworld".to_owned(), false, 0),
            Word::Call("helloworld".to_owned(), false, 0),
            Word::Call("println".to_owned(), true, 0),
        ],
    }
    .exec(&mut stack);
    lex("func println { | print \"\\n\" print } def helloworld \"Hello, World\" =helloworld helloworld println".to_owned(), "TEST".to_owned()).exec(&mut stack);
    Runtime::reset();
}
