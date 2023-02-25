#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]

pub mod dyn_fns;
pub mod lexer;
pub mod mutex;
pub mod runtime;
pub mod std_fns;
pub mod stream;

use std::fs;

use lexer::lex;
use runtime::*;

pub fn start_file(path: &str) -> Result<Stack, Error> {
    Runtime::new().set();
    (start_file_in_runtime(path), Runtime::reset()).0
}

pub fn start_file_in_runtime(path: &str) -> Result<Stack, Error> {
    let mut stack = Stack::new_in(FrameInfo {
        file: "std.spl".to_owned(),
        function: "root".to_owned(),
    });
    // import stdlib
    let words =
        lex(fs::read_to_string(find_in_splpath("std.spl")).unwrap()).map_err(|x| Error {
            kind: ErrorKind::LexError(format!("{x:?}")),
            stack: Vec::new(),
        })?;
    words.exec(&mut stack)?;

    // run file
    Words {
        words: vec![
            Word::Const(Value::Str(path.to_owned())),
            Word::Call("call-main-on-file".to_owned(), false, 0),
        ],
    }
    .exec(&mut stack)?;

    Ok(stack)
}

#[macro_export]
macro_rules! require_on_stack {
    ($name:tt, $type:tt, $stack:expr, $fn:literal) => {
        let Value::$type($name)
            = $stack.pop().lock_ro().native.clone() else {
            return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
        };
    };
}
#[macro_export]
macro_rules! require_int_on_stack {
    ($name:tt, $stack:expr, $fn:literal) => {
        let Value::Int($name)
            = $stack.pop().lock_ro().native.clone().try_mega_to_int() else {
            return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
        };
    };
}
#[macro_export]
macro_rules! require_array {
    ($name:tt, $array:expr, $stack:expr, $fn:literal) => {
        let Value::Array(ref $name)
            = $array.lock_ro().native else {
            return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
        };
    };
}
#[macro_export]
macro_rules! require_mut_array {
    ($name:tt, $array:expr, $stack:expr, $fn:literal) => {
        let Value::Array(ref mut $name)
            = $array.lock().native else {
            return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
        };
    };
}
#[macro_export]
macro_rules! require_array_on_stack {
    ($name:tt, $stack:expr, $fn:literal) => {
        let binding = $stack.pop();
        require_array!($name, binding, $stack, $fn)
    };
}
#[macro_export]
macro_rules! require_mut_array_on_stack {
    ($name:tt, $stack:expr, $fn:literal) => {
        let binding = $stack.pop();
        require_mut_array!($name, binding, $stack, $fn)
    };
}
