//! # Using SPL as a crate
//!
//! SPL has a complete API for use in applications and libraries.
//! To start a file, use `start_file` with the path, which is relatively straightforward, just like
//! `start_file_in_runtime`, which is the same as `start_file` but doesn't create and set a
//! runtime.
//!
//! To start code more customizably, you will have to create a stack and a runtime yourself, then
//! call `add_std` to include the standard library.
//!
//! Example:
//! ```
//! use spl::*;
//! fn main() -> OError {
//!     Runtime::new().set();
//!     let mut stack = Stack::new();
//!     add_std(&mut stack)?;
//!     Words::new(vec![
//!         Word::Const(Value::Str("Hello, World!".to_owned())),
//!         Word::Call("println".to_owned(), /*pop result:*/ false, /*reference:*/ 0)
//!     ]).exec(&mut stack);
//!     Ok(())
//! }
//! ```

#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]

pub mod dyn_fns;
pub mod lexer;
pub mod mutex;
pub mod oxidizer;
pub mod runtime;
pub mod sasm;
pub mod std_fns;
pub mod stdlib;
pub mod stream;

pub use lexer::*;
pub use runtime::*;

use std::fs;

/// Creates a runtime, lexes and executes some SPL code from a file, returning the stack that was
/// used for the operations, which should be empty in most cases.
pub fn start_file(path: &str) -> Result<Stack, Error> {
    Runtime::new().set();
    (start_file_in_runtime(path), Runtime::reset()).0
}

/// TO START A STANDALONE PIECE OF CODE, USE start_file!!
/// Lexes and starts some SPL code from a file, returning the stack.
pub fn start_file_in_runtime(path: &str) -> Result<Stack, Error> {
    let mut stack = Stack::new();
    // import stdlib
    add_std(&mut stack)?;

    // run file
    Words::new(vec![
        Word::Const(Value::Str(path.to_owned())),
        Word::Call("call-main-on-file".to_owned(), false, 0),
    ])
    .exec(&mut stack)?;

    Ok(stack)
}

/// Include the standard library in a runtime-stack-pair, where the runtime has been .set().
pub fn add_std(stack: &mut Stack) -> OError {
    let f = find_in_splpath("std.spl");
    let words = lex(if let Ok(f) = f {
        fs::read_to_string(f).unwrap()
    } else {
        f.unwrap_err()
    })
    .map_err(|x| stack.error(ErrorKind::LexError(format!("{x:?}"))))?;
    words.exec(stack)
}

macro_rules! nofmt {
    {$($code:tt)*} => {
        $($code)*
    };
}

// rustfmt adds infinite indentation to this, incrementing every time it is run.
nofmt! {
    #[macro_export]
    macro_rules! require_on_stack {
        ($name:tt, $type:tt, $stack:expr, $fn:literal) => {
            let Value::$type($name) = $stack.pop().lock_ro().native.clone() else {
                return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
            };
        };
    }

    #[macro_export]
    macro_rules! require_int_on_stack {
        ($name:tt, $stack:expr, $fn:literal) => {
            let Value::Int($name) = $stack.pop().lock_ro().native.clone().try_mega_to_int() else {
                return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
            };
        };
    }
    #[macro_export]
    macro_rules! require_array {
        ($name:tt, $array:expr, $stack:expr, $fn:literal) => {
            let Value::Array(ref $name) = $array.lock_ro().native else {
                return $stack.err(ErrorKind::InvalidCall($fn.to_owned()))
            };
        };
    }
    #[macro_export]
    macro_rules! require_mut_array {
        ($name:tt, $array:expr, $stack:expr, $fn:literal) => {
            let Value::Array(ref mut $name) = $array.lock().native else {
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
}
