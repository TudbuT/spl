use std::sync::Arc;

use crate::{lexer, runtime::*};

pub fn dyn_dump(stack: &mut Stack) {
    Words {
        words: vec![Word::Key(Keyword::Dump)],
    }
    .exec(stack);
}

pub fn dyn_def(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        Words {
            words: vec![Word::Key(Keyword::Def(s))],
        }
        .exec(stack);
    } else {
        panic!("incorrect usage of dyn-def");
    }
}

pub fn dyn_func(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        if let Value::Func(f) = stack.pop().lock_ro().native.clone() {
            stack.define_func(s, f);
        } else {
            panic!("incorrect usage of dyn-func");
        }
    } else {
        panic!("incorrect usage of dyn-func");
    }
}

pub fn dyn_construct(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        Words {
            words: vec![Word::Key(Keyword::Construct(s, Vec::new(), Vec::new()))],
        }
        .exec(stack);
    } else {
        panic!("incorrect usage of dyn-construct");
    }
}

pub fn dyn_def_field(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        if let Value::Str(name) = stack.pop().lock_ro().native.clone() {
            runtime(|rt| {
                rt.get_type_by_name(s)
                    .unwrap()
                    .lock()
                    .add_property(name, stack.get_frame());
            });
        } else {
            panic!("incorrect usage of dyn-def-field");
        }
    } else {
        panic!("incorrect usage of dyn-def-field");
    }
}

pub fn dyn_def_method(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        if let Value::Str(name) = stack.pop().lock_ro().native.clone() {
            if let Value::Func(f) = stack.pop().lock_ro().native.clone() {
                runtime(|rt| {
                    rt.get_type_by_name(s)
                        .unwrap()
                        .lock()
                        .functions
                        .insert(name, f);
                });
            } else {
                panic!("incorrect usage of dyn-def-method");
            }
        } else {
            panic!("incorrect usage of dyn-def-method");
        }
    } else {
        panic!("incorrect usage of dyn-def-method");
    }
}

pub fn dyn_include(stack: &mut Stack) {
    if let Value::Str(b) = stack.pop().lock_ro().native.clone() {
        if let Value::Str(a) = stack.pop().lock_ro().native.clone() {
            Words {
                words: vec![Word::Key(Keyword::Include(a, b))],
            }
            .exec(stack);
        } else {
            panic!("incorrect usage of dyn-include");
        }
    } else {
        panic!("incorrect usage of dyn-include");
    }
}

pub fn dyn_while(stack: &mut Stack) {
    if let Value::Func(blk) = stack.pop().lock_ro().native.clone() {
        if let Value::Func(cond) = stack.pop().lock_ro().native.clone() {
            loop {
                cond.to_call.call(stack);
                if !stack.pop().lock_ro().is_truthy() {
                    break;
                }
                blk.to_call.call(stack);
            }
        } else {
            panic!("incorrect usage of dyn-while");
        }
    } else {
        panic!("incorrect usage of dyn-while");
    }
}

pub fn dyn_if(stack: &mut Stack) {
    if let Value::Func(blk) = stack.pop().lock_ro().native.clone() {
        if stack.pop().lock_ro().is_truthy() {
            blk.to_call.call(stack);
        }
    } else {
        panic!("incorrect usage of dyn-if");
    }
}

pub fn dyn_call(stack: &mut Stack) {
    if let Value::Str(mut s) = stack.pop().lock_ro().native.clone() {
        let mut words = Vec::new();
        let mut ra = 0;
        while s.starts_with("&") {
            ra += 1;
            s = s[1..].to_owned();
        }
        if s.ends_with(";") {
            words.push(Word::Call(s[..s.len() - 1].to_owned(), true, ra));
        } else {
            words.push(Word::Call(s.to_owned(), false, ra));
        }
        Words { words }.exec(stack);
    } else {
        panic!("incorrect usage of dyn-call");
    }
}

pub fn dyn_objcall(stack: &mut Stack) {
    if let Value::Str(mut s) = stack.pop().lock_ro().native.clone() {
        let mut words = Vec::new();
        let mut ra = 0;
        while s.starts_with("&") {
            ra += 1;
            s = s[1..].to_owned();
        }
        if s.ends_with(";") {
            words.push(Word::ObjCall(s[..s.len() - 1].to_owned(), true, ra));
        } else {
            words.push(Word::ObjCall(s.to_owned(), false, ra));
        }
        Words { words }.exec(stack);
    } else {
        panic!("incorrect usage of dyn-objcall");
    }
}

pub fn dyn_all_types(stack: &mut Stack) {
    runtime(|rt| {
        stack.push(
            Value::Array(
                rt.get_types()
                    .into_iter()
                    .map(|x| Value::Str(x.lock_ro().get_name()).spl())
                    .collect(),
            )
            .spl(),
        );
    });
}

pub fn dyn_read(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        stack.push(
            Value::Func(AFunc::new(Func {
                ret_count: 0,
                to_call: FuncImpl::SPL(lexer::lex(
                    s,
                    "dyn-read@".to_owned() + &stack.get_origin().file,
                    stack.get_frame(),
                )),
                origin: stack.get_frame(),
                cname: None,
            }))
            .spl(),
        );
    } else {
        panic!("incorrect usage of dyn-call");
    }
}

pub fn dyn_readf(stack: &mut Stack) {
    if let Value::Str(n) = stack.pop().lock_ro().native.clone() {
        if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
            stack.push(
                Value::Func(AFunc::new(Func {
                    ret_count: 0,
                    to_call: FuncImpl::SPL(lexer::lex(s, n.clone(), stack.get_frame())),
                    origin: stack.get_frame(),
                    cname: Some(n),
                }))
                .spl(),
            );
        } else {
            panic!("incorrect usage of dyn-call");
        }
    } else {
        panic!("incorrect usage of dyn-call");
    }
}

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    let fns: [(&str, fn(&mut Stack), u32); 14] = [
        ("dyn-__dump", dyn_dump, 0),
        ("dyn-def", dyn_def, 0),
        ("dyn-func", dyn_func, 0),
        ("dyn-construct", dyn_construct, 0),
        ("dyn-def-field", dyn_def_field, 0),
        ("dyn-def-method", dyn_def_method, 0),
        ("dyn-include", dyn_include, 0),
        ("dyn-while", dyn_while, 0),
        ("dyn-if", dyn_if, 0),
        ("dyn-call", dyn_call, 0),
        ("dyn-objcall", dyn_objcall, 0),
        ("dyn-all-types", dyn_all_types, 1),
        ("dyn-read", dyn_read, 1),
        ("dyn-readf", dyn_readf, 1),
    ];
    for f in fns {
        r.define_func(f.0.to_owned(), AFunc::new(Func {
            ret_count: f.2,
            to_call: FuncImpl::Native(f.1),
            origin: o.clone(),
            cname: None,
        }));
    }
}
