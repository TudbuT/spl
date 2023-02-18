use core::panic;
use std::{
    io::{stdout, Write},
    mem,
    sync::Arc,
};

use crate::{mutex::Mut, runtime::*};

pub fn print(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        print!("{s}");
        stdout().lock().flush().unwrap();
    } else {
        panic!("incorrect usage of print");
    }
}

pub fn clone(stack: &mut Stack) {
    let o = stack.pop();
    stack.push(Arc::new(Mut::new(o.lock_ro().clone())));
}

pub fn dup(stack: &mut Stack) {
    let o = stack.peek();
    stack.push(o);
}

pub fn pop(stack: &mut Stack) {
    stack.pop();
}

pub fn swap(stack: &mut Stack) {
    let a = stack.pop();
    let b = stack.pop();
    stack.push(a);
    stack.push(b);
}

pub fn settype(stack: &mut Stack) {
    if let Value::Str(s) = stack.pop().lock_ro().native.clone() {
        let o = stack.pop();
        let kind = runtime(|rt| rt.get_type_by_name(s).unwrap());
        let mut obj = o.lock();
        for property in &kind.lock_ro().properties {
            obj.property_map.insert(property.clone(), Value::Null.spl());
        }
        obj.kind = kind;
        mem::drop(obj);
        stack.push(o);
    } else {
        panic!("incorrect usage of settype");
    }
}

pub fn gettype(stack: &mut Stack) {
    let o = stack.pop();
    stack.push(Value::Str(o.lock_ro().kind.lock_ro().get_name()).spl());
}

pub fn array_new(stack: &mut Stack) {
    if let Value::Mega(i) = stack.pop().lock_ro().native.clone() {
        stack.push(Value::Array(vec![Value::Null.spl(); i as usize]).spl());
    } else {
        panic!("incorrect usage of anew");
    }
}

pub fn array_len(stack: &mut Stack) {
    if let Value::Array(ref a) = stack.pop().lock_ro().native {
        stack.push(Value::Mega(a.len() as i128).spl());
    } else {
        panic!("incorrect usage of array-len");
    }
}

pub fn array_get(stack: &mut Stack) {
    if let Value::Array(ref a) = stack.pop().lock_ro().native {
        if let Value::Mega(i) = stack.pop().lock_ro().native.clone() {
            stack.push(a[i as usize].clone());
        } else {
            panic!("incorrect usage of array-get");
        }
    } else {
        panic!("incorrect usage of array-get");
    }
}

pub fn array_set(stack: &mut Stack) {
    if let Value::Array(ref mut a) = stack.pop().lock().native {
        if let Value::Mega(i) = stack.pop().lock_ro().native.clone() {
            let o = stack.pop();
            a[i as usize] = o;
        } else {
            panic!("incorrect usage of array-set");
        }
    } else {
        panic!("incorrect usage of array-set");
    }
}

pub fn eq(stack: &mut Stack) {
    let b = stack.pop();
    let a = stack.pop();
    stack.push(Value::Int(if a == b { 1 } else { 0 }).spl())
}

pub fn lt(stack: &mut Stack) {
    let b = stack.pop();
    let a = stack.pop();
    stack.push(Value::Int(if a < b { 1 } else { 0 }).spl())
}

pub fn gt(stack: &mut Stack) {
    let b = stack.pop();
    let a = stack.pop();
    stack.push(Value::Int(if a > b { 1 } else { 0 }).spl())
}

pub fn not(stack: &mut Stack) {
    let o = stack.pop();
    stack.push(Value::Int(if o.lock_ro().is_truthy() { 0 } else { 1 }).spl())
}

pub fn plus(stack: &mut Stack) {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a + b),
            _ => panic!(),
        }
        .spl(),
    );
}

pub fn minus(stack: &mut Stack) {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a - b),
            _ => panic!(),
        }
        .spl(),
    );
}

pub fn slash(stack: &mut Stack) {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a / b),
            _ => panic!(),
        }
        .spl(),
    );
}

pub fn star(stack: &mut Stack) {
    let a = stack.pop().lock_ro().native.clone();
    let b = stack.pop().lock_ro().native.clone();
    stack.push(
        match (a, b) {
            (Value::Mega(a), Value::Mega(b)) => Value::Mega(a * b),
            _ => panic!(),
        }
        .spl(),
    );
}

pub fn to_int(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Int(match o {
        Value::Null => panic!("incompatible: null - int"),
        Value::Int(x) => x,
        Value::Long(x) => x as i32,
        Value::Mega(x) => x as i32,
        Value::Float(x) => x as i32,
        Value::Double(x) => x as i32,
        Value::Func(_) => panic!("incompatible: func - int"),
        Value::Array(_) => panic!("incompatible: array - int"),
        Value::Str(x) => x.parse().expect("invalid int"),
    }).spl())
}

pub fn to_long(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Long(match o {
        Value::Null => panic!("incompatible: null - long"),
        Value::Int(x) => x as i64,
        Value::Long(x) => x as i64,
        Value::Mega(x) => x as i64,
        Value::Float(x) => x as i64,
        Value::Double(x) => x as i64,
        Value::Func(_) => panic!("incompatible: func - long"),
        Value::Array(_) => panic!("incompatible: array - long"),
        Value::Str(x) => x.parse().expect("invalid long"),
    }).spl())
}

pub fn to_mega(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Mega(match o {
        Value::Null => panic!("incompatible: null - mega"),
        Value::Int(x) => x as i128,
        Value::Long(x) => x as i128,
        Value::Mega(x) => x as i128,
        Value::Float(x) => x as i128,
        Value::Double(x) => x as i128,
        Value::Func(_) => panic!("incompatible: func - mega"),
        Value::Array(_) => panic!("incompatible: array - mega"),
        Value::Str(x) => x.parse().expect("invalid mega"),
    }).spl())
}

pub fn to_float(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Float(match o {
        Value::Null => panic!("incompatible: null - float"),
        Value::Int(x) => x as f32,
        Value::Long(x) => x as f32,
        Value::Mega(x) => x as f32,
        Value::Float(x) => x as f32,
        Value::Double(x) => x as f32,
        Value::Func(_) => panic!("incompatible: func - float"),
        Value::Array(_) => panic!("incompatible: array - float"),
        Value::Str(x) => x.parse().expect("invalid float"),
    }).spl())
}

pub fn to_double(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Double(match o {
        Value::Null => panic!("incompatible: null - double"),
        Value::Int(x) => x as f64,
        Value::Long(x) => x as f64,
        Value::Mega(x) => x as f64,
        Value::Float(x) => x as f64,
        Value::Double(x) => x as f64,
        Value::Func(_) => panic!("incompatible: func - double"),
        Value::Array(_) => panic!("incompatible: array - double"),
        Value::Str(x) => x.parse().expect("invalid double"),
    }).spl())
}

pub fn to_array(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Array(match o {
        Value::Null => panic!("incompatible: null - array"),
        Value::Int(_) => panic!("incompatible: int - array"),
        Value::Long(_) => panic!("incompatible: long - array"),
        Value::Mega(_) => panic!("incompatible: mega - array"),
        Value::Float(_) => panic!("incompatible: float - array"),
        Value::Double(_) => panic!("incompatible: double - array"),
        Value::Func(_) => panic!("incompatible: func - array"),
        Value::Array(x) => x,
        Value::Str(x) => x.chars().map(|x| Value::Int(x as u32 as i32).spl()).collect(),
    }).spl())
}

pub fn to_str(stack: &mut Stack) {
    let o = stack.pop().lock_ro().native.clone();
    stack.push(Value::Str(match o {
        Value::Null => panic!("incompatible: null - str"),
        Value::Int(x) => x.to_string(),
        Value::Long(x) => x.to_string(),
        Value::Mega(x) => x.to_string(),
        Value::Float(x) => x.to_string(),
        Value::Double(x) => x.to_string(),
        Value::Func(_) => panic!("incompatible: func - str"),
        Value::Array(x) => String::from_iter(x.into_iter().map(|x| match &x.lock_ro().native {
            Value::Int(x) => char::from_u32(*x as u32).expect("invalid Unicode Char: {x}"),
            _ => panic!("incompatible: !int - __str_element")
        })),
        Value::Str(x) => x,
    }).spl())
}

pub fn call(stack: &mut Stack) {
    if let Value::Func(ref a) = stack.pop().lock_ro().native {
        stack.call(a);
    } else {
        panic!("incorrect usage of call");
    }
}

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    r.define_func(
        "pop".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(pop),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "dup".to_owned(),
        AFunc::new(Func {
            ret_count: 2,
            to_call: FuncImpl::Native(dup),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "clone".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(clone),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "swap".to_owned(),
        AFunc::new(Func {
            ret_count: 2,
            to_call: FuncImpl::Native(swap),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "print".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(print),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "call".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(call),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "gettype".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(gettype),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "settype".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(settype),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "anew".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(array_new),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "array-len".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(array_len),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "array-get".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(array_get),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "array-set".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(array_set),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "eq".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(eq),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "lt".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(lt),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "gt".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(gt),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "not".to_owned(),
        AFunc::new(Func {
            ret_count: 0,
            to_call: FuncImpl::Native(not),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "+".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(plus),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "-".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(minus),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "/".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(slash),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "*".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(star),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_int".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_int),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_long".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_long),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_mega".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_mega),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_float".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_float),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_double".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_double),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_array".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_array),
            origin: o.clone(),
        }),
    );
    r.define_func(
        "_str".to_owned(),
        AFunc::new(Func {
            ret_count: 1,
            to_call: FuncImpl::Native(to_str),
            origin: o.clone(),
        }),
    );
}
