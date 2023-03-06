use std::{
    collections::VecDeque,
    env::{args, vars},
    fs,
    io::{stdin, stdout, Write},
    mem,
    ops::{Add, Div, Mul, Rem, Sub},
    process::{self, Stdio},
    sync::Arc,
};

use crate::{dyn_fns, mutex::Mut, runtime::*, *};

#[macro_export]
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

pub fn mswap(stack: &mut Stack) -> OError {
    let Value::Mega(i) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("nswap".to_owned()))
    };
    let mut array = VecDeque::with_capacity(i as usize);
    for _ in 0..i {
        array.push_back(stack.pop());
    }
    for _ in 0..i {
        // SAFETY: Items must exist because they are added in the previous loop
        stack.push(array.pop_front().unwrap());
    }
    Ok(())
}

pub fn settype(stack: &mut Stack) -> OError {
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("settype".to_owned()))
    };
    let o = stack.pop();
    let kind = runtime(|rt| rt.get_type_by_name(&s))
        .ok_or_else(|| stack.error(ErrorKind::TypeNotFound(s)))?;
    let mut obj = o.lock();
    kind.lock_ro().write_into(&mut obj);
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

pub fn and(stack: &mut Stack) -> OError {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(
        Value::Int(if a.lock_ro().is_truthy() && b.lock_ro().is_truthy() {
            1
        } else {
            0
        })
        .spl(),
    );
    Ok(())
}

pub fn or(stack: &mut Stack) -> OError {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(
        Value::Int(if a.lock_ro().is_truthy() || b.lock_ro().is_truthy() {
            1
        } else {
            0
        })
        .spl(),
    );
    Ok(())
}

macro_rules! impl_op {
    ($a:expr, $b:expr, $op:tt, $err:expr, $($kind:tt,)*) => {
        match ($a, $b) {
            $(
                (Value::$kind(a), Value::$kind(b)) => Value::$kind(a.$op(b)),
            )*
            _ => $err?,
        }
    };
}

pub fn plus(stack: &mut Stack) -> OError {
    let b = stack.pop().lock_ro().native.clone();
    let a = stack.pop().lock_ro().native.clone();
    stack.push(
        impl_op!(
            a,
            b,
            add,
            stack.err(ErrorKind::InvalidCall("plus".to_owned())),
            Mega,
            Long,
            Int,
        )
        .spl(),
    );
    Ok(())
}

pub fn minus(stack: &mut Stack) -> OError {
    let b = stack.pop().lock_ro().native.clone();
    let a = stack.pop().lock_ro().native.clone();
    stack.push(
        impl_op!(
            a,
            b,
            sub,
            stack.err(ErrorKind::InvalidCall("minus".to_owned())),
            Mega,
            Long,
            Int,
        )
        .spl(),
    );
    Ok(())
}

pub fn slash(stack: &mut Stack) -> OError {
    let b = stack.pop().lock_ro().native.clone();
    let a = stack.pop().lock_ro().native.clone();
    stack.push(
        impl_op!(
            a,
            b,
            div,
            stack.err(ErrorKind::InvalidCall("slash".to_owned())),
            Mega,
            Long,
            Int,
        )
        .spl(),
    );
    Ok(())
}

pub fn star(stack: &mut Stack) -> OError {
    let b = stack.pop().lock_ro().native.clone();
    let a = stack.pop().lock_ro().native.clone();
    stack.push(
        impl_op!(
            a,
            b,
            mul,
            stack.err(ErrorKind::InvalidCall("star".to_owned())),
            Mega,
            Long,
            Int,
        )
        .spl(),
    );
    Ok(())
}

pub fn percent(stack: &mut Stack) -> OError {
    let b = stack.pop().lock_ro().native.clone();
    let a = stack.pop().lock_ro().native.clone();
    stack.push(
        impl_op!(
            a,
            b,
            rem,
            stack.err(ErrorKind::InvalidCall("star".to_owned())),
            Mega,
            Long,
            Int,
        )
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
            Value::Long(x) => x,
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
            Value::Mega(x) => x,
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
            Value::Float(x) => x,
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
            Value::Double(x) => x,
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

pub fn callp(stack: &mut Stack) -> OError {
    let Value::Func(a) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("callp".to_owned()))
    };
    stack.call(&a)?;
    for _ in 0..a.ret_count {
        stack.pop();
    }
    Ok(())
}

pub fn trace(stack: &mut Stack) -> OError {
    let trace = stack.trace();
    stack.push(Value::Array(trace.into_iter().map(|x| Value::Str(x).spl()).collect()).spl());
    Ok(())
}

pub fn mr_trace(stack: &mut Stack) -> OError {
    let trace = stack.mr_trace();
    stack.push(
        Value::Array(
            trace
                .into_iter()
                .map(|x| Value::Array(x.into_iter().map(|x| x.spl()).collect()).spl())
                .collect(),
        )
        .spl(),
    );
    Ok(())
}

pub fn exit(stack: &mut Stack) -> OError {
    let Value::Int(a) = stack.pop().lock_ro().native.clone().try_mega_to_int() else {
        return stack.err(ErrorKind::InvalidCall("exit".to_owned()))
    };
    process::exit(a)
}

pub fn exec(stack: &mut Stack) -> OError {
    let Value::Func(a) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("exec".to_owned()))
    };
    unsafe {
        let f = stack.pop_frame(0);
        let f1 = stack.pop_frame(0);
        a.to_call.call(stack)?;
        stack.push_frame(f1);
        stack.push_frame(f);
    }
    Ok(())
}

pub fn exec2(stack: &mut Stack) -> OError {
    let Value::Func(a) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("exec2".to_owned()))
    };
    unsafe {
        let f = stack.pop_frame(0);
        let f1 = stack.pop_frame(0);
        let f2 = stack.pop_frame(0);
        a.to_call.call(stack)?;
        stack.push_frame(f2);
        stack.push_frame(f1);
        stack.push_frame(f);
    }
    Ok(())
}

pub fn stop(stack: &mut Stack) -> OError {
    let Value::Int(i) = stack.pop().lock_ro().native.clone().try_mega_to_int() else {
        return stack.err(ErrorKind::InvalidCall("stop".to_owned()))
    };
    stack.return_accumultor += i as u32;
    Ok(())
}

pub fn argv(stack: &mut Stack) -> OError {
    stack.push(Value::Array(args().into_iter().map(|x| Value::Str(x).spl()).collect()).spl());
    Ok(())
}

pub fn get_env(stack: &mut Stack) -> OError {
    stack.push(
        Value::Array(
            vars()
                .into_iter()
                .map(|x| Value::Array(vec![Value::Str(x.0).spl(), Value::Str(x.1).spl()]).spl())
                .collect(),
        )
        .spl(),
    );
    Ok(())
}

pub fn read_file(stack: &mut Stack) -> OError {
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

pub fn alit_end(stack: &mut Stack) -> OError {
    let s = stack.pop();
    let popped = stack.pop_until(s);
    stack.push(Value::Array(popped).spl());
    Ok(())
}

pub fn import(stack: &mut Stack) -> OError {
    let Value::Str(mut s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("import".to_owned()))
    };
    let fallback = match s
        .as_str()
        .rsplit_once(|x| x == '/' || x == '#')
        .map(|(.., x)| x)
        .unwrap_or(s.as_str())
    {
        "std.spl" => Some(stdlib::STD),
        "iter.spl" => Some(stdlib::ITER),
        "stream.spl" => Some(stdlib::STREAM),
        _ => None,
    };
    if let Some(x) = s.strip_prefix('#') {
        s = find_in_splpath(x).unwrap_or(x.to_owned());
    } else if let Some(x) = s.strip_prefix('@') {
        s = x.to_owned();
    } else {
        s = stack
            .peek_frame(1)
            .origin
            .file
            .rsplit_once('/')
            .map(|x| x.0)
            .unwrap_or(".")
            .to_owned()
            + "/"
            + &s;
    }
    if stack.include_file(
        &(*fs::canonicalize(s.clone())
            .map_err(|x| stack.error(ErrorKind::IO(x.to_string())))?
            .as_os_str()
            .to_string_lossy())
        .to_owned(),
    ) {
        stack.push(Value::Str(s).spl());
        dup(stack)?;
        read_file(stack).or_else(|x| {
            if let Some(fallback) = fallback {
                stack.push(Value::Str(fallback.to_owned()).spl());
                Ok(())
            } else {
                Err(x)
            }
        })?;
        dyn_fns::wrap(dyn_fns::dyn_readf)(stack)?;
        call(stack)?;
    }
    Ok(())
}

pub fn readln(stack: &mut Stack) -> OError {
    let mut s = String::new();
    stdin()
        .read_line(&mut s)
        .map_err(|x| stack.error(ErrorKind::IO(format!("{x:?}"))))?;
    let s = if let Some(s) = s.strip_suffix("\r\n") {
        s.to_owned()
    } else {
        s
    };
    let s = if let Some(s) = s.strip_suffix('\n') {
        s.to_owned()
    } else {
        s
    };
    stack.push(Value::Str(s).spl());
    Ok(())
}

pub fn command(stack: &mut Stack) -> OError {
    let binding = stack.pop();
    let Value::Array(ref a) = binding.lock_ro().native else {
        return stack.err(ErrorKind::InvalidCall("command".to_owned()))
    };
    let mut args = Vec::new();
    for item in a.iter() {
        if let Value::Str(ref s) = item.lock_ro().native {
            args.push(s.to_owned());
        }
    }
    if args.is_empty() {
        return stack.err(ErrorKind::InvalidCall("command".to_owned()));
    }
    process::Command::new(&args[0])
        .args(&args[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|x| stack.error(ErrorKind::IO(x.to_string())))?;
    Ok(())
}

pub fn command_wait(stack: &mut Stack) -> OError {
    let binding = stack.pop();
    let Value::Array(ref a) = binding.lock_ro().native else {
        return stack.err(ErrorKind::InvalidCall("command".to_owned()))
    };
    let mut args = Vec::new();
    for item in a.iter() {
        if let Value::Str(ref s) = item.lock_ro().native {
            args.push(s.to_owned());
        } else {
            return stack.err(ErrorKind::InvalidCall("command".to_owned()));
        }
    }
    if args.is_empty() {
        return stack.err(ErrorKind::InvalidCall("command".to_owned()));
    }
    stack.push(
        Value::Int(
            process::Command::new(&args[0])
                .args(&args[1..])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|x| stack.error(ErrorKind::IO(x.to_string())))?
                .wait()
                .map_err(|x| stack.error(ErrorKind::IO(x.to_string())))?
                .code()
                .unwrap_or(-1),
        )
        .spl(),
    );
    Ok(())
}

pub fn str_to_bytes(stack: &mut Stack) -> OError {
    require_on_stack!(s, Str, stack, "str-to-bytes");
    stack.push(
        Value::Array(
            s.bytes()
                .into_iter()
                .map(|x| Value::Int(x as i32).spl())
                .collect(),
        )
        .spl(),
    );
    Ok(())
}

pub fn bytes_to_str(stack: &mut Stack) -> OError {
    require_array_on_stack!(a, stack, "str-to-bytes");
    let mut chars = Vec::new();
    for item in a.iter() {
        if let Value::Int(x) = item.lock_ro().native.clone().try_mega_to_int() {
            chars.push(x as u8);
        } else {
            return stack.err(ErrorKind::InvalidCall("command".to_owned()));
        }
    }
    stack.push(Value::Str(String::from_utf8_lossy(&chars[..]).into_owned()).spl());
    Ok(())
}

pub fn acopy(stack: &mut Stack) -> OError {
    require_on_stack!(len, Mega, stack, "acopy");
    require_on_stack!(idx_dest, Mega, stack, "acopy");
    require_on_stack!(idx_src, Mega, stack, "acopy");
    let dest_array = stack.pop();
    {
        require_mut_array!(dest, dest_array, stack, "acopy");
        require_array_on_stack!(src, stack, "acopy");
        let offset = idx_dest - idx_src;
        if (src.len() as i128) < idx_src + len
            || idx_src < 0
            || (dest.len() as i128) < idx_dest + len
            || idx_dest < 0
        {
            stack.err(ErrorKind::InvalidCall("acopy".to_owned()))?;
        }
        for i in idx_src..idx_src + len {
            *dest.get_mut((i + offset) as usize).unwrap() = src.get(i as usize).unwrap().clone();
        }
    }
    stack.push(dest_array);
    Ok(())
}

pub fn throw(stack: &mut Stack) -> OError {
    let kind = ErrorKind::CustomObject(stack.pop());
    stack.err(kind)
}

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    type Fn = fn(&mut Stack) -> OError;
    let fns: [(&str, Fn, u32); 50] = [
        ("pop", pop, 0),
        ("dup", dup, 2),
        ("clone", clone, 1),
        ("swap", swap, 2),
        ("mswap", mswap, 2),
        ("print", print, 0),
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
        ("and", and, 1),
        ("or", or, 1),
        ("+", plus, 1),
        ("-", minus, 1),
        ("/", slash, 1),
        ("*", star, 1),
        ("%", percent, 1),
        ("_int", to_int, 1),
        ("_long", to_long, 1),
        ("_mega", to_mega, 1),
        ("_float", to_float, 1),
        ("_double", to_double, 1),
        ("_array", to_array, 1),
        ("_str", to_str, 1),
        ("call", call, 0),
        ("callp", callp, 0),
        ("trace", trace, 1),
        ("mr-trace", mr_trace, 1),
        ("exit", exit, 0),
        ("exec", exec, 0),
        ("exec2", exec2, 0),
        ("stop", stop, 0),
        ("argv", argv, 1),
        ("get-env", get_env, 1),
        ("read-file", read_file, 1),
        ("alit-end", alit_end, 1),
        ("import", import, 0),
        ("readln", readln, 1),
        ("command", command, 0),
        ("command-wait", command_wait, 1),
        ("str-to-bytes", str_to_bytes, 1),
        ("bytes-to-str", bytes_to_str, 1),
        ("acopy", acopy, 1),
        ("throw", throw, 0),
    ];
    for f in fns {
        r.define_func(
            f.0.to_owned(),
            AFunc::new(Func {
                ret_count: f.2,
                to_call: FuncImpl::Native(f.1),
                run_as_base: false,
                origin: o.clone(),
                fname: None,
                name: f.0.to_owned(),
            }),
        );
    }
}
