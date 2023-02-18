use crate::{dyn_fns, mutex::*, std_fns};

use core::panic;
use std::collections::VecDeque;
use std::mem;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    vec,
};

pub type AMObject = Arc<Mut<Object>>;
pub type AMType = Arc<Mut<Type>>;
pub type AFunc = Arc<Func>;

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

pub fn runtime<T>(f: impl FnOnce(&mut Runtime) -> T) -> T {
    RUNTIME.with(|rt| f(rt.borrow_mut().as_mut().unwrap()))
}

#[derive(Clone)]
pub struct Runtime {
    next_type_id: u32,
    types_by_name: HashMap<String, AMType>,
    types_by_id: HashMap<u32, AMType>,
}

impl Runtime {
    pub fn new() -> Self {
        let mut rt = Runtime {
            next_type_id: 0,
            types_by_name: HashMap::new(),
            types_by_id: HashMap::new(),
        };
        rt.make_type("null".to_owned(), |t| t);
        rt.make_type("int".to_owned(), |t| t);
        rt.make_type("long".to_owned(), |t| t);
        rt.make_type("mega".to_owned(), |t| t);
        rt.make_type("float".to_owned(), |t| t);
        rt.make_type("double".to_owned(), |t| t);
        rt.make_type("func".to_owned(), |t| t);
        rt.make_type("array".to_owned(), |t| t);
        rt.make_type("str".to_owned(), |t| t);
        rt
    }

    pub fn get_type_by_name(&self, name: String) -> Option<AMType> {
        self.types_by_name.get(&name).cloned()
    }

    pub fn get_type_by_id(&self, id: u32) -> Option<AMType> {
        self.types_by_id.get(&id).cloned()
    }

    pub fn get_types(&self) -> Vec<AMType> {
        self.types_by_id.clone().into_values().collect()
    }

    pub fn make_type(&mut self, name: String, op: impl FnOnce(Type) -> Type) -> AMType {
        let t = Arc::new(Mut::new(op(Type {
            name: name.clone(),
            id: (self.next_type_id, self.next_type_id += 1).0,
            parents: Vec::new(),
            functions: HashMap::new(),
            properties: Vec::new(),
        })));
        self.types_by_id.insert(self.next_type_id - 1, t.clone());
        self.types_by_name.insert(name, t.clone());
        t
    }

    pub fn set(self) {
        RUNTIME.with(move |x| *x.borrow_mut() = Some(self));
    }

    pub fn reset() {
        RUNTIME.with(|x| *x.borrow_mut() = None);
    }
}

#[derive(Clone)]
pub struct FrameInfo {
    pub file: String,
}

#[derive(Clone)]
pub struct Frame {
    parent: Option<Arc<Frame>>,
    pub variables: Mut<HashMap<String, AMObject>>,
    pub functions: Mut<HashMap<String, AFunc>>,
    pub origin: FrameInfo,
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("\nVars: \n")?;
        for (name, object) in self.variables.lock_ro().iter() {
            f.write_str("  ")?;
            f.write_str(&name)?;
            f.write_str(": ")?;
            std::fmt::Display::fmt(&object.lock_ro(), f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl Frame {
    fn root() -> Self {
        Frame {
            parent: None,
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: FrameInfo {
                file: "RUNTIME".to_owned(),
            },
        }
    }

    pub fn new(parent: Arc<Frame>) -> Self {
        Frame {
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: parent.origin.clone(),
            parent: Some(parent),
        }
    }

    pub fn new_in(parent: Arc<Frame>, origin: String) -> Self {
        Frame {
            parent: Some(parent),
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: FrameInfo { file: origin },
        }
    }
}

#[derive(Clone)]
pub struct Stack {
    frames: Vec<Arc<Frame>>,
    object_stack: Vec<AMObject>,
}

impl Display for Stack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for frame in &self.frames {
            f.write_str("Frame: ")?;
            f.write_str(&frame.origin.file)?;
            f.write_str("\n\n")?;
            frame.as_ref().fmt(f)?;
        }
        f.write_str("Stack: \n")?;
        for object in &self.object_stack {
            f.write_str("  ")?;
            std::fmt::Display::fmt(&object.lock_ro(), f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl Stack {
    pub fn new() -> Self {
        let o = Arc::new(Frame::root());
        let mut r = Stack {
            frames: vec![o.clone()],
            object_stack: Vec::new(),
        };

        dyn_fns::register(&mut r, o.clone());
        std_fns::register(&mut r, o.clone());

        r
    }

    pub fn define_func(&mut self, name: String, func: AFunc) {
        self.frames
            .last_mut()
            .unwrap()
            .functions
            .lock()
            .insert(name, func);
    }

    pub fn call(&mut self, func: &AFunc) {
        self.frames.push(Arc::new(Frame::new(func.origin.clone())));
        func.to_call.call(self);
        self.frames.pop().unwrap();
    }

    pub fn get_func(&self, name: String) -> AFunc {
        let mut frame = self.frames.last().unwrap();
        loop {
            let functions = &frame.functions;
            if let Some(x) = functions.lock_ro().get(&name) {
                return x.clone();
            }
            if let Some(ref x) = frame.parent {
                frame = x;
            } else {
                panic!("Function not found: {}", name)
            }
        }
    }

    // TODO actually use the frame for the variable (can cause issues when using referenced
    // functions)
    pub fn define_var(&mut self, name: String) {
        let frame = self.frames.last_mut().unwrap().clone();
        let tmpname = name.clone();
        frame.functions.lock().insert(
            name.clone(),
            Arc::new(Func {
                ret_count: 1,
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    stack.push(stack.get_var(tmpname.clone()))
                }))),
                origin: frame.clone(),
            }),
        );
        let tmpname = name.clone();
        frame.functions.lock().insert(
            "=".to_owned() + &name,
            Arc::new(Func {
                ret_count: 0,
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    let v = stack.pop();
                    stack.set_var(tmpname.clone(), v);
                }))),
                origin: frame.clone(),
            }),
        );
        frame.variables.lock().insert(name, Value::Null.spl());
    }

    pub fn set_var(&mut self, name: String, obj: AMObject) {
        let mut frame = self.frames.last().unwrap();
        loop {
            if let Some(x) = frame.variables.lock().get_mut(&name) {
                *x = obj;
                break;
            }
            if let Some(ref x) = frame.parent {
                frame = x;
            } else {
                dyn_fns::dyn_dump(self);
                panic!("undefined var: {name}");
            }
        }
    }

    pub fn get_var(&self, name: String) -> AMObject {
        let mut frame = self.frames.last().unwrap();
        loop {
            if let Some(x) = frame.variables.lock_ro().get(&name) {
                return x.clone();
            }
            if let Some(ref x) = frame.parent {
                frame = x;
            } else {
                panic!("undefined var")
            }
        }
    }

    pub fn push(&mut self, obj: AMObject) {
        self.object_stack.push(obj)
    }

    pub fn peek(&mut self) -> AMObject {
        self.object_stack
            .last()
            .cloned()
            .unwrap_or(Value::Null.spl())
    }

    pub fn pop(&mut self) -> AMObject {
        self.object_stack.pop().unwrap_or(Value::Null.spl())
    }

    pub fn get_origin(&self) -> FrameInfo {
        self.frames.last().unwrap().origin.clone()
    }

    pub fn get_frame(&self) -> Arc<Frame> {
        self.frames.last().unwrap().clone()
    }
}

#[derive(Clone, Debug)]
pub enum Keyword {
    /// <none>
    ///
    /// Dumps stack. Not available as actual keyword, therefore only obtainable through AST
    /// manipulation or modding. When using dyn variant, it must be enabled in the main function.
    /// equivalent to dyn-__dump
    /// example: func main { int | dyn-__dump-check "Hello, world!" dyn-__dump 0 }
    Dump,
    /// def <name>
    ///
    /// Defines a variable.
    /// equivalent to <name> dyn-def
    Def(String),
    /// func <name> { <rem> | <words> }
    ///
    /// Defines function <name> with <rem> return size
    /// equivalent to { <rem> | <words> } "<name>" dyn-func
    Func(String, u32, Words),
    /// construct <name> { <field> <...> ; <fn-name> { <rem> | <words> } <...> }
    ///
    /// Creates type <name>
    /// equivalent to
    /// "<name>" dyn-construct; "<field>" "<name>" dyn-def-field { <rem> | <words> } "<fn-name>"
    /// "<name>" dyn-def-method
    Construct(String, Vec<String>, Vec<(String, (u32, Words))>),
    /// include <typeA> in <typeB>
    ///
    /// Adds <typeA> as a parent type of <typeB>.
    /// equivalent to "<typeA>" "<typeB>" dyn-include
    Include(String, String),
    /// while { <wordsA> } { <wordsB> }
    ///
    /// If wordsA result in a truthy value being on the top of the stack, execute wordsB, and
    /// repeat.
    /// equivalent to { int | <wordsA> } { | <wordsB> } dyn-while
    While(Words, Words),
    /// if { <wordsB> }
    ///
    /// If wordsA result in a truthy value being on the top of the stack, execute wordsB.
    /// equivalent to { | <wordsB> } dyn-if
    If(Words),
    /// with <item> <...> ;
    ///
    /// Defines variables in reverse order.
    /// equivalent to def <...> =<...> def <item> =<item>
    /// or "<...>" dyn-def "=<...>" dyn-call "<item>" dyn-def "=<item>" dyn-call
    With(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Int(i32),
    Long(i64),
    Mega(i128),
    Float(f32),
    Double(f64),
    Func(AFunc),
    Array(Vec<AMObject>),
    Str(String),
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Mega(a), Value::Mega(b)) => a.partial_cmp(b),
            _ => panic!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Word {
    Key(Keyword),
    Const(Value),
    Call(String, bool, u32),
    ObjCall(String, bool, u32),
}

#[derive(Clone, Debug)]
pub struct Words {
    pub words: Vec<Word>,
}

#[derive(Clone)]
pub enum FuncImpl {
    Native(fn(&mut Stack)),
    NativeDyn(Arc<Box<dyn Fn(&mut Stack)>>),
    SPL(Words),
}

impl FuncImpl {
    pub fn call(&self, stack: &mut Stack) {
        match self {
            FuncImpl::Native(x) => x(stack),
            FuncImpl::NativeDyn(x) => x(stack),
            FuncImpl::SPL(x) => x.exec(stack),
        }
    }
}

#[derive(Clone)]
pub struct Func {
    pub ret_count: u32,
    pub to_call: FuncImpl,
    pub origin: Arc<Frame>,
}

impl PartialEq for Func {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.ret_count.to_string())?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Type {
    name: String,
    id: u32,
    pub parents: Vec<AMType>,
    pub functions: HashMap<String, AFunc>,
    pub properties: Vec<String>,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Type {
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_fn(&self, name: String) -> Option<AFunc> {
        if let Some(x) = self.functions.get(&name) {
            return Some(x.clone());
        }
        let mut q = VecDeque::from(self.parents.clone());
        while let Some(t) = q.pop_front() {
            if let Some(x) = t.lock_ro().functions.get(&name) {
                return Some(x.clone());
            }
            q.append(&mut VecDeque::from(t.lock_ro().parents.clone()));
        }
        None
    }

    pub fn add_property(&mut self, name: String, origin: Arc<Frame>) {
        let tmpname = name.clone();
        self.functions.insert(
            name.clone(),
            Arc::new(Func {
                ret_count: 1,
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    let o = stack.pop();
                    stack.push(o.lock_ro().property_map.get(&tmpname).unwrap().clone());
                }))),
                origin: origin.clone(),
            }),
        );
        let tmpname = name.clone();
        self.functions.insert(
            "=".to_owned() + &name,
            Arc::new(Func {
                ret_count: 0,
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    let o = stack.pop();
                    let v = stack.pop();
                    o.lock().property_map.insert(tmpname.clone(), v);
                }))),
                origin,
            }),
        );
        self.properties.push(name);
    }
}

#[derive(Clone, Debug)]
pub struct Object {
    pub kind: AMType,
    pub property_map: HashMap<String, AMObject>,
    pub native: Value,
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.property_map == other.property_map
            && self.native == other.native
    }
}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.kind != other.kind {
            panic!();
        }
        self.native.partial_cmp(&other.native)
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.kind.lock_ro().name)?;
        f.write_str("(")?;
        self.native.fmt(f)?;
        f.write_str(") { ")?;
        for (k, v) in &self.property_map {
            f.write_str(&k)?;
            f.write_str(": ")?;
            std::fmt::Display::fmt(&v.lock_ro(), f)?;
        }
        f.write_str(" }")?;
        Ok(())
    }
}

impl Object {
    pub fn new(kind: AMType, native: Value) -> Object {
        Object {
            property_map: {
                let mut map = HashMap::new();
                for property in &kind.lock_ro().properties {
                    map.insert(property.clone(), Value::Null.spl());
                }
                map
            },
            kind,
            native,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match &self.native {
            Value::Null => false,
            Value::Int(x) => x > &0,
            Value::Long(x) => x > &0,
            Value::Mega(x) => x > &0,
            Value::Float(_) => true,
            Value::Double(_) => true,
            Value::Func(_) => true,
            Value::Array(_) => true,
            Value::Str(x) => x == "",
        }
    }
}

impl From<Value> for Object {
    fn from(value: Value) -> Self {
        Object::new(
            RUNTIME.with(|x| {
                let x = x.borrow();
                let x = x.as_ref().unwrap();
                match value {
                    Value::Null => x.get_type_by_id(0),
                    Value::Int(_) => x.get_type_by_id(1),
                    Value::Long(_) => x.get_type_by_id(2),
                    Value::Mega(_) => x.get_type_by_id(3),
                    Value::Float(_) => x.get_type_by_id(4),
                    Value::Double(_) => x.get_type_by_id(5),
                    Value::Func(_) => x.get_type_by_id(6),
                    Value::Array(_) => x.get_type_by_id(7),
                    Value::Str(_) => x.get_type_by_id(8),
                }
                .expect("runtime uninitialized: default types not set.")
            }),
            value,
        )
    }
}

pub trait SPL {
    fn spl(self) -> AMObject;
}

impl<T> SPL for T
where
    T: Into<Object>,
{
    fn spl(self) -> AMObject {
        Arc::new(Mut::new(self.into()))
    }
}

impl Words {
    pub fn exec(&self, stack: &mut Stack) {
        for word in self.words.clone() {
            match word {
                Word::Key(x) => match x {
                    Keyword::Dump => println!("{}", stack),
                    Keyword::Def(x) => stack.define_var(x),
                    Keyword::Func(name, rem, words) => stack.define_func(
                        name,
                        Arc::new(Func {
                            ret_count: rem,
                            to_call: FuncImpl::SPL(words),
                            origin: stack.get_frame(),
                        }),
                    ),
                    Keyword::Construct(name, fields, methods) => {
                        let origin = stack.get_frame();
                        stack.define_var(name.clone());
                        stack.set_var(
                            name.clone(),
                            Value::Str(
                                RUNTIME
                                    .with(move |rt| {
                                        rt.borrow_mut().as_mut().unwrap().make_type(
                                            name,
                                            move |mut t| {
                                                for field in fields {
                                                    t.add_property(field, origin.clone());
                                                }
                                                t.functions.extend(methods.into_iter().map(
                                                    |(k, v)| {
                                                        (
                                                            k,
                                                            Arc::new(Func {
                                                                ret_count: v.0,
                                                                to_call: FuncImpl::SPL(v.1),
                                                                origin: origin.clone(),
                                                            }),
                                                        )
                                                    },
                                                ));
                                                t
                                            },
                                        )
                                    })
                                    .lock_ro()
                                    .get_name(),
                            )
                            .spl(),
                        );
                    }
                    Keyword::Include(ta, tb) => {
                        RUNTIME.with(move |rt| {
                            let mut rt = rt.borrow_mut();
                            let rt = rt.as_mut().unwrap();
                            // TODO: Handle properly
                            rt.get_type_by_name(tb)
                                .unwrap()
                                .lock()
                                .parents
                                .push(rt.get_type_by_name(ta).unwrap())
                        });
                    }
                    Keyword::While(cond, blk) => loop {
                        cond.exec(stack);
                        if !stack.pop().lock_ro().is_truthy() {
                            break;
                        }
                        blk.exec(stack);
                    },
                    Keyword::If(blk) => {
                        if stack.pop().lock_ro().is_truthy() {
                            blk.exec(stack);
                        }
                    }
                    Keyword::With(vars) => {
                        for var in vars.into_iter().rev() {
                            stack.define_var(var.clone());
                            let obj = stack.pop();
                            stack.set_var(var, obj);
                        }
                    }
                },
                Word::Const(x) => stack.push(x.clone().spl()),
                Word::Call(x, rem, ra) => {
                    let f = stack.get_func(x);
                    if ra != 0 {
                        let mut f = Value::Func(f);
                        for _ in 1..ra {
                            let ftmp = f;
                            f = Value::Func(AFunc::new(Func {
                                ret_count: 1,
                                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                                    stack.push(ftmp.clone().spl());
                                }))),
                                origin: stack.get_frame(),
                            }));
                        }
                    } else {
                        stack.call(&f);
                        if rem {
                            for _ in 0..f.ret_count {
                                stack.pop();
                            }
                        }
                    }
                }
                Word::ObjCall(x, rem, ra) => {
                    let o = stack.peek();
                    let o = o.lock_ro();
                    // TODO: raise error if not found
                    let f0 = o.kind.lock_ro();
                    let f = f0
                        .functions
                        .get(&x)
                        .unwrap_or_else(|| {
                            panic!(
                                "objcall not possible. {} {:?} {}",
                                o.kind.lock_ro().name,
                                o.native,
                                x
                            )
                        })
                        .clone();
                    mem::drop(f0);
                    mem::drop(o);
                    if ra != 0 {
                        let mut f = Value::Func(f.clone());
                        for _ in 1..ra {
                            let ftmp = f;
                            f = Value::Func(AFunc::new(Func {
                                ret_count: 1,
                                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                                    stack.push(ftmp.clone().spl());
                                }))),
                                origin: stack.get_frame(),
                            }));
                        }
                    } else {
                        stack.call(&f);
                        if rem {
                            for _ in 0..f.ret_count {
                                stack.pop();
                            }
                        }
                    }
                }
            }
        }
    }
}
