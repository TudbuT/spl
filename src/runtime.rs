use crate::{
    dyn_fns,
    mutex::*,
    std_fns,
    stream::{self, *},
};

use core::panic;
use std::mem;
use std::sync::RwLockWriteGuard;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    vec,
};
use std::{collections::VecDeque, thread::panicking};

pub type AMObject = Arc<Mut<Object>>;
pub type AMType = Arc<Mut<Type>>;
pub type AFunc = Arc<Func>;
pub type OError = Result<(), Error>;

thread_local! {
    static RUNTIME: RefCell<Option<Arc<Mut<Runtime>>>> = RefCell::new(None);
}

pub fn runtime<T>(f: impl FnOnce(RwLockWriteGuard<Runtime>) -> T) -> T {
    RUNTIME.with(|rt| {
        f(rt.borrow_mut()
            .as_mut()
            .expect("no runtime (use .set())")
            .lock())
    })
}

#[derive(Clone)]
pub struct Runtime {
    next_type_id: u32,
    types_by_name: HashMap<String, AMType>,
    types_by_id: HashMap<u32, AMType>,
    next_stream_id: u128,
    streams: HashMap<u128, Arc<Mut<Stream>>>,
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("next_type_id", &self.next_type_id)
            .field("types_by_name", &self.types_by_name)
            .field("types_by_id", &self.types_by_id)
            .field("next_stream_id", &self.next_stream_id)
            .finish()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let mut rt = Runtime {
            next_type_id: 0,
            types_by_name: HashMap::new(),
            types_by_id: HashMap::new(),
            next_stream_id: 0,
            streams: HashMap::new(),
        };
        let _ = rt.make_type("null".to_owned(), Ok); // infallible
        let _ = rt.make_type("int".to_owned(), Ok); // infallible
        let _ = rt.make_type("long".to_owned(), Ok); // infallible
        let _ = rt.make_type("mega".to_owned(), Ok); // infallible
        let _ = rt.make_type("float".to_owned(), Ok); // infallible
        let _ = rt.make_type("double".to_owned(), Ok); // infallible
        let _ = rt.make_type("func".to_owned(), Ok); // infallible
        let _ = rt.make_type("array".to_owned(), Ok); // infallible
        let _ = rt.make_type("str".to_owned(), Ok); // infallible
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

    pub fn make_type(
        &mut self,
        name: String,
        op: impl FnOnce(Type) -> Result<Type, Error>,
    ) -> Result<AMType, Error> {
        let t = Arc::new(Mut::new(op(Type {
            name: name.clone(),
            id: (self.next_type_id, self.next_type_id += 1).0,
            parents: Vec::new(),
            functions: HashMap::new(),
            properties: Vec::new(),
        })?));
        self.types_by_id.insert(self.next_type_id - 1, t.clone());
        self.types_by_name.insert(name, t.clone());
        Ok(t)
    }

    pub fn register_stream(&mut self, stream: Stream) -> (u128, Arc<Mut<Stream>>) {
        let id = (self.next_stream_id, self.next_stream_id += 1).0;
        self.streams.insert(id, Arc::new(Mut::new(stream)));
        (id, self.streams.get(&id).unwrap().clone())
    }

    pub fn get_stream(&self, id: u128) -> Option<Arc<Mut<Stream>>> {
        self.streams.get(&id).cloned()
    }

    pub fn destroy_stream(&mut self, id: u128) {
        self.streams.remove(&id);
    }

    pub fn reset() {
        RUNTIME.with(|x| *x.borrow_mut() = None);
    }
}

pub trait SetRuntime {
    fn set(self);
}

impl SetRuntime for Runtime {
    fn set(self) {
        Arc::new(Mut::new(self)).set()
    }
}

impl SetRuntime for Arc<Mut<Runtime>> {
    fn set(self) {
        RUNTIME.with(move |x| *x.borrow_mut() = Some(self));
    }
}

#[derive(Clone, Debug)]
pub struct FrameInfo {
    pub file: String,
    pub function: String,
}

#[derive(Clone, Debug)]
pub struct Frame {
    parent: Option<Arc<Frame>>,
    pub variables: Mut<HashMap<String, AMObject>>,
    pub functions: Mut<HashMap<String, AFunc>>,
    pub origin: FrameInfo,
    pub redirect_to_base: bool,
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("\nVars: \n")?;
        for (name, object) in self.variables.lock_ro().iter() {
            f.write_str("  ")?;
            f.write_str(name)?;
            f.write_str(": ")?;
            std::fmt::Display::fmt(&object.lock_ro(), f)?;
            f.write_str("\n")?;
        }
        f.write_str("\n")?;
        Ok(())
    }
}

impl Frame {
    pub fn dummy() -> Self {
        Frame {
            parent: None,
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: FrameInfo {
                file: "\0".to_owned(),
                function: "\0".to_owned(),
            },
            redirect_to_base: false,
        }
    }

    pub fn root() -> Self {
        Frame {
            parent: None,
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: FrameInfo {
                file: "RUNTIME".to_owned(),
                function: "root".to_owned(),
            },
            redirect_to_base: false,
        }
    }

    pub fn root_in(info: FrameInfo) -> Self {
        Frame {
            parent: None,
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: info,
            redirect_to_base: false,
        }
    }

    pub fn new(parent: Arc<Frame>, function: String) -> Self {
        Frame {
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: FrameInfo {
                function,
                ..parent.origin.clone()
            },
            parent: Some(parent),
            redirect_to_base: false,
        }
    }

    pub fn new_in(
        parent: Arc<Frame>,
        origin: String,
        function: String,
        redirect_to_parent: bool,
    ) -> Self {
        Frame {
            parent: Some(parent),
            variables: Mut::new(HashMap::new()),
            functions: Mut::new(HashMap::new()),
            origin: FrameInfo {
                file: origin,
                function,
            },
            redirect_to_base: redirect_to_parent,
        }
    }

    pub fn set_var(&self, name: String, obj: AMObject, stack: &Stack) -> OError {
        let mut frame = self;
        loop {
            if let Some(x) = frame.variables.lock().get_mut(&name) {
                *x = obj;
                return Ok(());
            }
            if let Some(ref x) = frame.parent {
                frame = x;
            } else {
                return Err(Error {
                    kind: ErrorKind::VariableNotFound(name),
                    stack: stack.trace(),
                });
            }
        }
    }

    pub fn get_var(&self, name: String, stack: &Stack) -> Result<AMObject, Error> {
        let mut frame = self;
        loop {
            if let Some(x) = frame.variables.lock_ro().get(&name) {
                return Ok(x.clone());
            }
            if let Some(ref x) = frame.parent {
                frame = x;
            } else {
                return Err(Error {
                    kind: ErrorKind::VariableNotFound(name),
                    stack: stack.trace(),
                });
            }
        }
    }

    pub fn path(&self) -> Vec<FrameInfo> {
        let mut r = Vec::new();
        let mut frame = self;
        loop {
            r.insert(0, frame.origin.clone());

            if let Some(ref parent) = frame.parent {
                frame = parent;
            } else {
                break;
            }
        }
        r
    }

    pub fn readable_path(&self) -> String {
        let mut item = String::new();
        let path = self.path();
        let mut file = "\0".to_owned();
        for element in path {
            if element.file != file {
                item += " | in ";
                item += &element.file;
                item += ":";
                file = element.file;
            }
            item += " ";
            item += &element.function;
        }
        item
    }

    pub fn is_dummy(&self) -> bool {
        self.parent.is_none() && self.origin.file == "\0" && self.origin.function == "\0"
    }
}

#[derive(Clone, Debug)]
pub struct Stack {
    frames: Vec<Arc<Frame>>,
    object_stack: Vec<AMObject>,
    pub return_accumultor: u32,
}

impl Display for Stack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for frame in &self.frames {
            f.write_str("Frame:")?;
            f.write_str(&frame.readable_path())?;
            f.write_str("\n")?;
            std::fmt::Display::fmt(&frame.as_ref(), f)?;
            f.write_str("\n\n")?;
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

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Self {
        let o = Arc::new(Frame::root());
        let mut r = Stack {
            frames: vec![o.clone()],
            object_stack: Vec::new(),
            return_accumultor: 0,
        };

        dyn_fns::register(&mut r, o.clone());
        std_fns::register(&mut r, o.clone());
        stream::register(&mut r, o);

        r
    }

    pub fn new_in(frame_info: FrameInfo) -> Self {
        let o = Arc::new(Frame::root_in(frame_info));
        let mut r = Stack {
            frames: vec![o.clone()],
            object_stack: Vec::new(),
            return_accumultor: 0,
        };

        dyn_fns::register(&mut r, o.clone());
        std_fns::register(&mut r, o.clone());
        stream::register(&mut r, o);

        r
    }

    pub fn define_func(&mut self, name: String, func: AFunc) {
        let mut frame = self.frames.last().unwrap().clone();
        if frame.redirect_to_base {
            frame = self.frames.first().unwrap().clone();
        }
        frame.functions.lock().insert(name, func);
    }

    pub fn call(&mut self, func: &AFunc) -> OError {
        let f = if let Some(ref cname) = func.fname {
            Frame::new_in(
                func.origin.clone(),
                cname.clone(),
                func.name.clone(),
                func.run_at_base,
            )
        } else {
            Frame::new(func.origin.clone(), func.name.clone())
        };
        self.frames.push(Arc::new(f));
        func.to_call.call(self)?;
        self.frames.pop().unwrap();
        Ok(())
    }

    pub fn get_func(&self, name: String) -> Result<AFunc, Error> {
        let mut frame = self.frames.last().unwrap();
        loop {
            let functions = &frame.functions;
            if let Some(x) = functions.lock_ro().get(&name) {
                return Ok(x.clone());
            }
            if let Some(ref x) = frame.parent {
                frame = x;
            } else {
                return Err(Error {
                    kind: ErrorKind::FuncNotFound(name),
                    stack: self.trace(),
                });
            }
        }
    }

    pub fn define_var(&mut self, name: String) {
        let mut frame = self.frames.last().unwrap().clone();
        if frame.redirect_to_base {
            frame = self.frames.first().unwrap().clone();
        }
        let tmpname = name.clone();
        let tmpframe = frame.clone();
        frame.functions.lock().insert(
            name.clone(),
            Arc::new(Func {
                ret_count: 1,
                origin: frame.clone(),
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    stack.push(tmpframe.get_var(tmpname.clone(), stack)?);
                    Ok(())
                }))),
                run_at_base: false,
                fname: Some("RUNTIME".to_owned()),
                name: name.clone(),
            }),
        );
        let tmpname = name.clone();
        let tmpframe = frame.clone();
        frame.functions.lock().insert(
            "=".to_owned() + &name,
            Arc::new(Func {
                ret_count: 0,
                origin: frame.clone(),
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    let v = stack.pop();
                    tmpframe.set_var(tmpname.clone(), v, stack)
                }))),
                run_at_base: false,
                fname: Some("RUNTIME".to_owned()),
                name: "=".to_owned() + &name,
            }),
        );
        frame.variables.lock().insert(name, Value::Null.spl());
    }

    pub fn pop_until(&mut self, obj: AMObject) -> Vec<AMObject> {
        let Some((idx, ..)) = self.object_stack.iter().enumerate().rfind(|o| *o.1.lock_ro() == *obj.lock_ro()) else {
            return Vec::new()
        };
        let items = self.object_stack[idx + 1..].to_vec();
        self.object_stack = self.object_stack[0..idx].to_vec();
        items
    }

    pub fn len(&self) -> usize {
        self.object_stack.len()
    }

    pub fn set_var(&self, name: String, obj: AMObject) -> OError {
        self.get_frame().set_var(name, obj, self)
    }

    pub fn get_var(&self, name: String) -> Result<AMObject, Error> {
        self.get_frame().get_var(name, self)
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

    pub fn err<T>(&self, kind: ErrorKind) -> Result<T, Error> {
        Err(Error {
            kind,
            stack: self.trace(),
        })
    }

    pub fn error(&self, kind: ErrorKind) -> Error {
        Error {
            kind,
            stack: self.trace(),
        }
    }

    pub fn mr_trace(&self) -> Vec<Vec<FrameInfo>> {
        self.frames.iter().map(|frame| frame.path()).collect()
    }

    pub fn trace(&self) -> Vec<String> {
        self.frames
            .iter()
            .map(|frame| frame.readable_path())
            .collect()
    }

    pub fn peek_frame(&self, index: usize) -> Arc<Frame> {
        self.frames
            .get(self.frames.len() - index - 1)
            .unwrap()
            .clone()
    }

    /// # Safety
    /// This function is not technically unsafe. It is marked this way to indicate that it
    /// can cause unstable runtime states and panics. However, memory safety is still guaranteed.
    pub unsafe fn pop_frame(&mut self, index: usize) -> Arc<Frame> {
        self.frames.remove(self.frames.len() - index - 1)
    }

    /// # Safety
    /// This function is not technically unsafe. It is marked this way to indicate that it
    /// can cause unstable runtime states and panics. However, memory safety is still guaranteed.
    pub unsafe fn push_frame(&mut self, frame: Arc<Frame>) {
        self.frames.push(frame);
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

impl Value {
    fn ensure_init(self, stack: &Stack) -> Self {
        match self {
            Value::Func(x) if x.origin.is_dummy() => Value::Func(AFunc::new(Func {
                origin: stack.get_frame(),
                ..x.as_ref().clone()
            })),
            x => x,
        }
    }

    pub fn try_mega_to_int(self) -> Value {
        if let Value::Mega(x) = self {
            Value::Int(x as i32)
        } else {
            self
        }
    }
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
    Native(fn(&mut Stack) -> OError),
    NativeDyn(Arc<Box<dyn Fn(&mut Stack) -> OError>>),
    SPL(Words),
}

impl FuncImpl {
    pub fn call(&self, stack: &mut Stack) -> OError {
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
    pub run_at_base: bool,
    pub origin: Arc<Frame>,
    pub fname: Option<String>,
    pub name: String,
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

    pub fn write_into(&self, object: &mut Object) {
        let mut to_apply = self.properties.clone();
        let mut q = VecDeque::from(self.parents.clone());
        while let Some(t) = q.pop_front() {
            to_apply.append(&mut t.lock_ro().properties.clone());
            q.append(&mut VecDeque::from(t.lock_ro().parents.clone()));
        }
        for property in to_apply.into_iter().rev() {
            object.property_map.insert(property, Value::Null.spl());
        }
    }

    pub fn add_property(&mut self, name: String, origin: Arc<Frame>) -> OError {
        let tmpname = name.clone();
        self.functions.insert(
            name.clone(),
            Arc::new(Func {
                ret_count: 1,
                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                    let o = stack.pop();
                    let o = o.lock_ro();
                    stack.push(
                        o.property_map
                            .get(&tmpname)
                            .ok_or_else(|| {
                                stack.error(ErrorKind::PropertyNotFound(
                                    o.kind.lock_ro().name.clone(),
                                    tmpname.clone(),
                                ))
                            })?
                            .clone(),
                    );
                    Ok(())
                }))),
                run_at_base: false,
                origin: origin.clone(),
                fname: Some("RUNTIME".to_owned()),
                name: name.clone(),
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
                    Ok(())
                }))),
                run_at_base: false,
                origin,
                fname: Some("RUNTIME".to_owned()),
                name: "=".to_owned() + &name,
            }),
        );
        self.properties.push(name);
        Ok(())
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

impl Eq for Object {}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.kind.lock_ro().name)?;
        f.write_str("(")?;
        self.native.fmt(f)?;
        f.write_str(") {")?;
        for (k, v) in &self.property_map {
            f.write_str(" ")?;
            f.write_str(k)?;
            f.write_str(": ")?;
            std::fmt::Display::fmt(&v.lock_ro(), f)?;
            f.write_str(",")?;
        }
        f.write_str(" }")?;
        Ok(())
    }
}

impl Object {
    pub fn new(kind: AMType, native: Value) -> Object {
        let mut r = Object {
            property_map: HashMap::new(),
            kind: kind.clone(),
            native,
        };
        kind.lock_ro().write_into(&mut r);
        r
    }

    pub fn is_truthy(&self) -> bool {
        match &self.native {
            Value::Null => self.kind.lock_ro().id != 0,
            Value::Int(x) => x > &0,
            Value::Long(x) => x > &0,
            Value::Mega(x) => x > &0,
            Value::Float(_) => true,
            Value::Double(_) => true,
            Value::Func(_) => true,
            Value::Array(_) => true,
            Value::Str(x) => !x.is_empty(),
        }
    }
}

impl From<Value> for Object {
    fn from(value: Value) -> Self {
        Object::new(
            runtime(|x| {
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
    pub fn exec(&self, stack: &mut Stack) -> OError {
        for word in self.words.clone() {
            match word {
                Word::Key(x) => match x {
                    Keyword::Dump => println!("{stack}"),
                    Keyword::Def(x) => stack.define_var(x),
                    Keyword::Func(name, rem, words) => stack.define_func(
                        name.clone(),
                        Arc::new(Func {
                            ret_count: rem,
                            to_call: FuncImpl::SPL(words),
                            run_at_base: false,
                            origin: stack.get_frame(),
                            fname: None,
                            name,
                        }),
                    ),
                    Keyword::Construct(name, fields, methods) => {
                        let origin = stack.get_frame();
                        stack.define_var(name.clone());
                        stack.set_var(
                            name.clone(),
                            Value::Str(
                                runtime(move |mut rt| {
                                    rt.make_type(name.clone(), move |mut t| {
                                        for field in fields {
                                            t.add_property(field, origin.clone())?;
                                        }
                                        t.functions.extend(methods.into_iter().map(|(k, v)| {
                                            (
                                                k.clone(),
                                                Arc::new(Func {
                                                    ret_count: v.0,
                                                    to_call: FuncImpl::SPL(v.1),
                                                    run_at_base: false,
                                                    origin: origin.clone(),
                                                    fname: None,
                                                    name: name.clone() + ":" + &k,
                                                }),
                                            )
                                        }));
                                        Ok(t)
                                    })
                                })?
                                .lock_ro()
                                .get_name(),
                            )
                            .spl(),
                        )?;
                    }
                    Keyword::Include(ta, tb) => {
                        let rstack = &stack;
                        runtime(move |rt| {
                            rt.get_type_by_name(tb.clone())
                                .ok_or_else(|| rstack.error(ErrorKind::TypeNotFound(tb)))?
                                .lock()
                                .parents
                                .push(
                                    rt.get_type_by_name(ta.clone())
                                        .ok_or_else(|| rstack.error(ErrorKind::TypeNotFound(ta)))?,
                                );
                            Ok(())
                        })?;
                    }
                    Keyword::While(cond, blk) => loop {
                        cond.exec(stack)?;
                        if !stack.pop().lock_ro().is_truthy() {
                            break;
                        }
                        blk.exec(stack)?;
                        if stack.return_accumultor > 0 {
                            break;
                        }
                    },
                    Keyword::If(blk) => {
                        if stack.pop().lock_ro().is_truthy() {
                            blk.exec(stack)?;
                        }
                    }
                    Keyword::With(vars) => {
                        for var in vars.into_iter().rev() {
                            stack.define_var(var.clone());
                            let obj = stack.pop();
                            stack.set_var(var, obj)?;
                        }
                    }
                },
                Word::Const(x) => {
                    if option_env!("SPLDEBUG").is_some() {
                        println!("CNST({}) {x:?}", stack.len());
                    }
                    stack.push(x.clone().ensure_init(stack).spl())
                }
                Word::Call(x, rem, ra) => {
                    if option_env!("SPLDEBUG").is_some() {
                        println!("CALL({}) {x}", stack.len());
                    }
                    let f = stack.get_func(x.clone())?;
                    if ra != 0 {
                        let mut f = Value::Func(f);
                        for n in 1..ra {
                            let mut s = String::new();
                            for _ in 0..n {
                                s += "&";
                            }
                            let ftmp = f;
                            f = Value::Func(AFunc::new(Func {
                                ret_count: 1,
                                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                                    stack.push(ftmp.clone().spl());
                                    Ok(())
                                }))),
                                run_at_base: false,
                                origin: stack.get_frame(),
                                fname: None,
                                name: s + &x,
                            }));
                        }
                        stack.push(f.spl());
                    } else {
                        stack.call(&f)?;
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
                    let f0 = o.kind.lock_ro();
                    let f = f0
                        .get_fn(x.clone())
                        .ok_or_else(|| Error {
                            kind: ErrorKind::MethodNotFound(f0.name.clone(), x.clone()),
                            stack: stack.trace(),
                        })?
                        .clone();
                    if option_env!("SPLDEBUG").is_some() {
                        println!("CALL({}) {}:{x}", stack.len(), &o.kind.lock_ro().name);
                    }
                    mem::drop(f0);
                    mem::drop(o);
                    if ra != 0 {
                        let mut f = Value::Func(f.clone());
                        for n in 1..ra {
                            let mut s = String::new();
                            for _ in 0..n {
                                s += "&";
                            }
                            let ftmp = f;
                            f = Value::Func(AFunc::new(Func {
                                ret_count: 1,
                                to_call: FuncImpl::NativeDyn(Arc::new(Box::new(move |stack| {
                                    stack.push(ftmp.clone().spl());
                                    Ok(())
                                }))),
                                run_at_base: false,
                                origin: stack.get_frame(),
                                fname: None,
                                name: s + &x,
                            }));
                        }
                        stack.pop();
                        stack.push(f.spl())
                    } else {
                        stack.call(&f)?;
                        if rem {
                            for _ in 0..f.ret_count {
                                stack.pop();
                            }
                        }
                    }
                }
            }
            if stack.return_accumultor > 0 {
                stack.return_accumultor -= 1;
                return Ok(());
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Parse(String, String),
    InvalidCall(String),
    InvalidType(String, String),
    VariableNotFound(String),
    FuncNotFound(String),
    MethodNotFound(String, String),
    PropertyNotFound(String, String),
    TypeNotFound(String),
    LexError(String),
    IO(String),
    Custom(String),
    CustomObject(AMObject),
}

#[derive(PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub stack: Vec<String>,
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if panicking() {
            f.write_str("\n\nSPL PANIC DUE TO UNCAUGHT ERROR:\n")?;
            f.write_str(format!("Error: {:?}", self.kind).as_str())?;
            f.write_str("\n")?;
            f.write_str(self.stack.join("\n").as_str())?;
            f.write_str("\n\n")?;
            Ok(())
        } else {
            f.debug_struct("Error")
                .field("kind", &self.kind)
                .field("stack", &self.stack)
                .finish()
        }
    }
}
