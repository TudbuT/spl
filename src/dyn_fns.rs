use std::sync::Arc;

use crate::{lexer, runtime::*};
use crate::*;

pub fn dyn_dump(stack: &mut Stack) -> OError {
    Words {
        words: vec![Word::Key(Keyword::Dump)],
    }
    .exec(stack)
}

pub fn dyn_def(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-def".to_owned()))
    };
    Words {
        words: vec![Word::Key(Keyword::Def(s))],
    }
    .exec(stack)?;
    Ok(())
}

pub fn dyn_func(stack: &mut Stack) -> OError {
    let (
        Value::Str(s),
        Value::Func(f),
    ) = (
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
    ) else {
        return stack.err(ErrorKind::InvalidCall("dyn-func".to_owned()))
    };
    stack.define_func(s, f);
    Ok(())
}

pub fn dyn_construct(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-construct".to_owned()))
    };
    Words {
        words: vec![Word::Key(Keyword::Construct(
            s,
            Vec::new(),
            Vec::new(),
            false,
        ))],
    }
    .exec(stack)?;
    Ok(())
}

pub fn dyn_namespace(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-construct".to_owned()))
    };
    Words {
        words: vec![Word::Key(Keyword::Construct(
            s,
            Vec::new(),
            Vec::new(),
            true,
        ))],
    }
    .exec(stack)?;
    Ok(())
}

pub fn dyn_def_field(stack: &mut Stack) -> OError {
    let (
        Value::Str(s),
        Value::Str(name),
    ) = (
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
    ) else {
        return stack.err(ErrorKind::InvalidCall("dyn-def-field".to_owned()))
    };
    runtime(|rt| rt.get_type_by_name(&s))
        .ok_or_else(|| stack.error(ErrorKind::TypeNotFound(s)))?
        .lock()
        .add_property(name, stack.get_frame())?;
    Ok(())
}

pub fn dyn_def_method(stack: &mut Stack) -> OError {
    let (
        Value::Str(s),
        Value::Str(name),
        Value::Func(f),
    ) = (
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
    ) else {
        return stack.err(ErrorKind::InvalidCall("dyn-def-method".to_owned()))
    };
    runtime(|rt| rt.get_type_by_name(&s))
        .ok_or_else(|| stack.error(ErrorKind::TypeNotFound(s)))?
        .lock()
        .functions
        .insert(name, f);
    Ok(())
}

pub fn dyn_include(stack: &mut Stack) -> OError {
    let (
        Value::Str(b),
        Value::Str(a),
    ) = (
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
    ) else {
        return stack.err(ErrorKind::InvalidCall("dyn-include".to_owned()))
    };
    Words {
        words: vec![Word::Key(Keyword::Include(a, b))],
    }
    .exec(stack)?;
    Ok(())
}

pub fn dyn_while(stack: &mut Stack) -> OError {
    let (
        Value::Func(blk),
        Value::Func(cond),
    ) = (
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
    ) else {
        return stack.err(ErrorKind::InvalidCall("dyn-while".to_owned()))
    };
    loop {
        cond.to_call.call(stack)?;
        if !stack.pop().lock_ro().is_truthy() {
            break;
        }
        blk.to_call.call(stack)?;
    }
    Ok(())
}

pub fn dyn_if(stack: &mut Stack) -> OError {
    let Value::Func(blk) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-if".to_owned()))
    };
    if stack.pop().lock_ro().is_truthy() {
        blk.to_call.call(stack)?;
    }
    Ok(())
}

pub fn dyn_call(stack: &mut Stack) -> OError {
    let Value::Str(mut s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-call".to_owned()))
    };
    let mut words = Vec::new();
    let mut ra = 0;
    while s.starts_with('&') {
        ra += 1;
        s = s[1..].to_owned();
    }
    if s.ends_with(';') {
        words.push(Word::Call(s[..s.len() - 1].to_owned(), true, ra));
    } else {
        words.push(Word::Call(s, false, ra));
    }
    Words { words }.exec(stack)?;
    Ok(())
}

pub fn dyn_objcall(stack: &mut Stack) -> OError {
    let Value::Str(mut s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-objcall".to_owned()))
    };
    let mut words = Vec::new();
    let mut ra = 0;
    while s.starts_with('&') {
        ra += 1;
        s = s[1..].to_owned();
    }
    if s.ends_with(';') {
        words.push(Word::ObjCall(s[..s.len() - 1].to_owned(), true, ra));
    } else {
        words.push(Word::ObjCall(s, false, ra));
    }
    Words { words }.exec(stack)?;
    Ok(())
}

pub fn dyn_all_types(stack: &mut Stack) -> OError {
    stack.push(
        Value::Array(
            runtime(|rt| rt.get_types())
                .into_iter()
                .map(|x| Value::Str(x.lock_ro().get_name()).spl())
                .collect(),
        )
        .spl(),
    );
    Ok(())
}

pub fn dyn_read(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("dyn-read".to_owned()))
    };
    stack.push(
        Value::Func(AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::SPL(
                lexer::lex(s).map_err(|x| stack.error(ErrorKind::LexError(format!("{x:?}"))))?,
            ),
            run_as_base: false,
            origin: stack.get_frame(),
            fname: None,
            name: "(dyn-read)".to_owned(),
        }))
        .spl(),
    );
    Ok(())
}

pub fn dyn_readf(stack: &mut Stack) -> OError {
    let (
        Value::Str(s),
        Value::Str(n),
    ) = (
        stack.pop().lock_ro().native.clone(),
        stack.pop().lock_ro().native.clone(),
    ) else {
        return stack.err(ErrorKind::InvalidCall("dyn-readf".to_owned()))
    };
    stack.push(
        Value::Func(AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::SPL(
                lexer::lex(s).map_err(|x| stack.error(ErrorKind::LexError(format!("{x:?}"))))?,
            ),
            run_as_base: true,
            origin: stack.get_frame(),
            fname: Some(n),
            name: "root".to_owned(),
        }))
        .spl(),
    );
    Ok(())
}

pub fn dyn_catch(stack: &mut Stack) -> OError {
    require_on_stack!(ctch, Func, stack, "dyn-catch");
    require_on_stack!(blk, Func, stack, "dyn-catch");
    require_on_stack!(types, Array, stack, "dyn-catch");
    if let Err(e) = blk.to_call.call(stack) {
        if types.is_empty() || types.contains(&e.kind.to_string().spl()) {
            stack.push(e.spl());
            ctch.to_call.call(stack)
        } else {
            Err(e)
        }
    } else {
        Ok(())
    }
}

pub fn dyn_use(stack: &mut Stack) -> OError {
    require_on_stack!(item, Str, stack, "dyn-use");
    Words::new(vec![Word::Key(Keyword::Use(item))]).exec(stack)
}

pub(crate) fn wrap(f: fn(&mut Stack) -> OError) -> impl Fn(&mut Stack) -> OError {
    move |stack| unsafe {
        let frame = stack.pop_frame(0);
        let r = f(stack);
        stack.push_frame(frame);
        r
    }
}

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    type Fn = fn(&mut Stack) -> OError;
    let fns: [(&str, Fn, u32); 17] = [
        ("dyn-__dump", dyn_dump, 0),
        ("dyn-def", dyn_def, 0),
        ("dyn-func", dyn_func, 0),
        ("dyn-construct", dyn_construct, 0),
        ("dyn-namespace", dyn_namespace, 0),
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
        ("dyn-catch", dyn_catch, 0),
        ("dyn-use", dyn_use, 0),
    ];
    for f in fns {
        r.define_func(
            f.0.to_owned(),
            AFunc::new(Func {
                ret_count: f.2,
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(wrap(f.1)))),
                run_as_base: false,
                origin: o.clone(),
                fname: None,
                name: f.0.to_owned(),
            }),
        );
    }
}
