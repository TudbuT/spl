use std::{
    io::{stdout, Write},
    mem, process,
    sync::Arc,
};

use crate::{mutex::Mut, runtime::*};

macro_rules! type_err {
    ($stack:expr, $a:expr, $b:expr) => {
        $stack.err(ErrorKind::InvalidType($a.to_owned(), $b.to_owned()))?
    };
}

pub fn print(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("print".to_owned()))
    };
    print!("{s}");
    stdout().lock().flush().unwrap();
    Ok(())
}

pub fn clone(stack: &mut Stack) -> OError {
    let o = stack.pop();
    stack.push(Arc::new(Mut::new(o.lock_ro().clone())));
    Ok(())
}

pub fn dup(stack: &mut Stack) -> OError {
    let o = stack.peek();
    stack.push(o);
    Ok(())
}

pub fn pop(stack: &mut Stack) -> OError {
    stack.pop();
    Ok(())
}

pub fn swap(stack: &mut Stack) -> OError {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a);
    stack.push(b);
    Ok(())
}

pub fn settype(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("settype".to_owned()))
    };
    let o = stack.pop();
    let kind = runtime(|rt| rt.get_type_by_name(s.clone()))
        .ok_or_else(|| stack.error(ErrorKind::TypeNotFound(s)))?;
    let mut obj = o.lock();
    for property in &kind.lock_ro().properties {
        obj.property_map.insert(property.clone(), Value::Null.spl());
    }
    obj.kind = kind;
    mem::drop(obj);
    stack.push(o);
    Ok(())
}

pub fn gettype(stack: &mut Stack) -> OError {
    let o = stack.pop();
    stack.push(Value::Str(o.lock_ro().kind.lock_ro().get_name()).spl());
    Ok(())
}

pub fn array_new(stack: &mut Stack) -> OError {
    let Value::Mega(i) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("anew".to_owned()))
    };
    stack.push(Value::Array(vec![Value::Null.spl(); i as usize]).spl());
    Ok(())
}

pub fn array_len(stack: &mut Stack) -> OError {
    let binding = stack.pop();
    let Value::Array(ref a) = binding.lock_ro().native else {
        return stack.err(ErrorKind::InvalidCall("array-len".to_owned()))
    };
    stack.push(Value::Mega(a.len() as i128).spl());
    Ok(())
}

pub fn array_get(stack: &mut Stack) -> OError {
    let binding = stack.pop();
    let Value::Array(ref a) = binding.lock_ro().native else {
        return stack.err(ErrorKind::InvalidCall("array-get".to_owned()))
    };
    let Value::Mega(i) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("array-get".to_owned()))
    };
    stack.push(a[i as usize].clone());
    Ok(())
}

pub fn array_set(stack: &mut Stack) -> OError {
    let binding = stack.pop();
    let Value::Array(ref mut a) = binding.lock().native else {
        return stack.err(ErrorKind::InvalidCall("array-set".to_owned()))
    };
    let Value::Mega(i) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("array-set".to_owned()))
    };
    let o = stack.pop();
    stack.push(a[i as usize].clone());
    a[i as usize] = o;
    Ok(())
}

pub fn eq(stack: &mut Stack) -> OError {
    let b = stack.pop();
    let a = stack.pop();
    stack.push(Value::Int(if a == b { 1 } else { 0 }).spl());
    Ok(())
}

pub fn lt(stack: &mut Stack) -> OError {
    let b = stack.pop();
    let a = stack.pop();
    stack.push(Value::Int(if a < b { 1 } else { 0 }).spl());
    Ok(())
}

pub fn gt(stack: &mut Stack) -> OError {
    let b = stack.pop();
    let a = stack.pop();
    stack.push(Value::Int(if a > b { 1 } else { 0 }).spl());
    Ok(())
}

pub fn not(stack: &mut Stack) -> OError {
    let o = stack.pop();
    stack.push(Value::Int(if o.lock_ro().is_truthy() { 0 } else { 1 }).spl());
    Ok(())
}

pub fn plus(stack: &mut Stack) -> OError {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a + b),
            _ => todo!(),
        }
        .spl(),
    );
    Ok(())
}

pub fn minus(stack: &mut Stack) -> OError {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a - b),
            _ => todo!(),
        }
        .spl(),
    );
    Ok(())
}

pub fn slash(stack: &mut Stack) -> OError {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a / b),
            _ => todo!(),
        }
        .spl(),
    );
    Ok(())
}

pub fn star(stack: &mut Stack) -> OError {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a * b),
            _ => todo!(),
        }
        .spl(),
    );
    Ok(())
}

pub fn to_int(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Int(match o {
            Value::Null => type_err!(stack, "null", "int"),
            Value::Int(x) => x,
            Value::Long(x) => x as i32,
            Value::Mega(x) => x as i32,
            Value::Float(x) => x as i32,
            Value::Double(x) => x as i32,
            Value::Func(_) => type_err!(stack, "func", "int"),
            Value::Array(_) => type_err!(stack, "array", "int"),
            Value::Str(x) => x
                .parse()
                .map_err(|_| stack.error(ErrorKind::Parse(x, "int".to_owned())))?,
        })
        .spl(),
    );
    Ok(())
}

pub fn to_long(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Long(match o {
            Value::Null => type_err!(stack, "null", "long"),
            Value::Int(x) => x as i64,
            Value::Long(x) => x as i64,
            Value::Mega(x) => x as i64,
            Value::Float(x) => x as i64,
            Value::Double(x) => x as i64,
            Value::Func(_) => type_err!(stack, "func", "long"),
            Value::Array(_) => type_err!(stack, "array", "long"),
            Value::Str(x) => x
                .parse()
                .map_err(|_| stack.error(ErrorKind::Parse(x, "long".to_owned())))?,
        })
        .spl(),
    );
    Ok(())
}

pub fn to_mega(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Mega(match o {
            Value::Null => type_err!(stack, "null", "mega"),
            Value::Int(x) => x as i128,
            Value::Long(x) => x as i128,
            Value::Mega(x) => x as i128,
            Value::Float(x) => x as i128,
            Value::Double(x) => x as i128,
            Value::Func(_) => type_err!(stack, "func", "mega"),
            Value::Array(_) => type_err!(stack, "array", "mega"),
            Value::Str(x) => x
                .parse()
                .map_err(|_| stack.error(ErrorKind::Parse(x, "mega".to_owned())))?,
        })
        .spl(),
    );
    Ok(())
}

pub fn to_float(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Float(match o {
            Value::Null => type_err!(stack, "null", "float"),
            Value::Int(x) => x as f32,
            Value::Long(x) => x as f32,
            Value::Mega(x) => x as f32,
            Value::Float(x) => x as f32,
            Value::Double(x) => x as f32,
            Value::Func(_) => type_err!(stack, "func", "float"),
            Value::Array(_) => type_err!(stack, "array", "float"),
            Value::Str(x) => x
                .parse()
                .map_err(|_| stack.error(ErrorKind::Parse(x, "float".to_owned())))?,
        })
        .spl(),
    );
    Ok(())
}

pub fn to_double(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Double(match o {
            Value::Null => type_err!(stack, "null", "double"),
            Value::Int(x) => x as f64,
            Value::Long(x) => x as f64,
            Value::Mega(x) => x as f64,
            Value::Float(x) => x as f64,
            Value::Double(x) => x as f64,
            Value::Func(_) => type_err!(stack, "func", "double"),
            Value::Array(_) => type_err!(stack, "array", "double"),
            Value::Str(x) => x
                .parse()
                .map_err(|_| stack.error(ErrorKind::Parse(x, "double".to_owned())))?,
        })
        .spl(),
    );
    Ok(())
}

pub fn to_array(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Array(match o {
            Value::Null => type_err!(stack, "null", "array"),
            Value::Int(_) => type_err!(stack, "int", "array"),
            Value::Long(_) => type_err!(stack, "long", "array"),
            Value::Mega(_) => type_err!(stack, "mega", "array"),
            Value::Float(_) => type_err!(stack, "float", "array"),
            Value::Double(_) => type_err!(stack, "double", "array"),
            Value::Func(_) => type_err!(stack, "func", "array"),
            Value::Array(x) => x,
            Value::Str(x) => x
                .chars()
                .map(|x| Value::Int(x as u32 as i32).spl())
                .collect(),
        })
        .spl(),
    );
    Ok(())
}

pub fn to_str(stack: &mut Stack) -> OError {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(
        Value::Str(match o {
            Value::Null => type_err!(stack, "null", "str"),
            Value::Int(x) => x.to_string(),
            Value::Long(x) => x.to_string(),
            Value::Mega(x) => x.to_string(),
            Value::Float(x) => x.to_string(),
            Value::Double(x) => x.to_string(),
            Value::Func(_) => type_err!(stack, "func", "str"),
            Value::Array(x) => {
                let iter: Vec<_> = x
                    .into_iter()
                    .map(|x| match &x.lock_ro().native {
                        Value::Int(x) => char::from_u32(*x as u32).ok_or_else(|| {
                            stack.error(ErrorKind::InvalidType(
                                format!("int-{x}"),
                                "__str-element".to_owned(),
                            ))
                        }),
                        _ => stack.err(ErrorKind::InvalidType(
                            "int".to_owned(),
                            "__str-element".to_owned(),
                        )),
                    })
                    .collect();
                let mut fixed = String::with_capacity(iter.len());
                for item in iter {
                    fixed.push(item?);
                }
                fixed
            }
            Value::Str(x) => x,
        })
        .spl(),
    );
    Ok(())
}

pub fn call(stack: &mut Stack) -> OError {
    let Value::Func(a) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("call".to_owned()))
    };
    stack.call(&a)
}

pub fn exit(stack: &mut Stack) -> OError {
    let Value::Int(a) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("exit".to_owned()))
    };
    process::exit(a)
}

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    let fns: [(&str, fn(&mut Stack) -> OError, u32); 28] = [
        ("pop", pop, 0),
        ("dup", dup, 2),
        ("clone", clone, 1),
        ("swap", swap, 2),
        ("print", print, 0),
        ("call", call, 0),
        ("gettype", gettype, 1),
        ("settype", settype, 1),
        ("anew", array_new, 1),
        ("array-len", array_len, 1),
        ("array-get", array_get, 1),
        ("array-set", array_set, 1),
        ("eq", eq, 1),
        ("lt", lt, 1),
        ("gt", gt, 1),
        ("not", not, 1),
        ("+", plus, 1),
        ("-", minus, 1),
        ("/", slash, 1),
        ("*", star, 1),
        ("_int", to_int, 1),
        ("_long", to_long, 1),
        ("_mega", to_mega, 1),
        ("_float", to_float, 1),
        ("_double", to_double, 1),
        ("_array", to_array, 1),
        ("_str", to_str, 1),
        ("exit", exit, 0),
    ];
    for f in fns {
        r.define_func(
            f.0.to_owned(),
            AFunc::new(Func {
                ret_count: f.2,
                to_call: FuncImpl::Native(f.1),
                origin: o.clone(),
                cname: None,
            }),
        );
    }
}
