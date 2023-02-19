use spl::{lexer::lex, runtime::*};

use std::{env::args, fs};

fn main() -> OError {
    let rt = Runtime::new();
    rt.set();
    let mut stack = Stack::new_in(FrameInfo {
        file: "std.spl".to_owned(),
        function: "root".to_owned(),
    });
    fn argv(stack: &mut Stack) -> OError {
        stack.push(Value::Array(args().into_iter().map(|x| Value::Str(x).spl()).collect()).spl());
        Ok(())
    }
    fn read_file(stack: &mut Stack) -> OError {
        let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
            return stack.err(ErrorKind::InvalidCall("read_file".to_owned()))
        };
        stack.push(
            Value::Str(
                fs::read_to_string(s).map_err(|x| stack.error(ErrorKind::IO(format!("{x:?}"))))?,
            )
            .spl(),
        );
        Ok(())
    }
    stack.define_func(
        "argv".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(argv),
            origin: stack.get_frame(),
            fname: None,
            name: "argv".to_owned(),
        }),
    );
    stack.define_func(
        "read-file".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(read_file),
            origin: stack.get_frame(),
            fname: None,
            name: "read-file".to_owned(),
        }),
    );
    let words = lex(fs::read_to_string("std.spl").unwrap()).map_err(|x| Error {
        kind: ErrorKind::LexError(format!("{x:?}")),
        stack: Vec::new(),
    })?;
    words.exec(&mut stack)?;
    Runtime::reset();
    Ok(())
}
