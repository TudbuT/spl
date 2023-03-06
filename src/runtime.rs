use crate::{
    dyn_fns,
    mutex::*,
    std_fns, stdlib,
    stream::{self, *},
};

use core::panic;
use std::collections::VecDeque;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    vec,
};
use std::{env::var, mem, path::Path};

pub type AMObject = Arc<Mut<Object>>;
pub type AMType = Arc<Mut<Type>>;
pub type AFunc = Arc<Func>;
pub type OError = Result<(), Error>;

thread_local! {
    static RUNTIME: RefCell<Option<Arc<Mut<Runtime>>>> = RefCell::new(None);
}

/// Obtains a reference to the runtime.
pub fn runtime<T>(f: impl FnOnce(RwLockReadGuard<Runtime>) -> T) -> T {
    RUNTIME.with(|rt| {
        f(rt.borrow_mut()
            .as_mut()
            .expect("no runtime (use .set())")
            .lock_ro())
    })
}

/// Obtains a mutable reference to the runtime.
pub fn runtime_mut<T>(f: impl FnOnce(RwLockWriteGuard<Runtime>) -> T) -> T {
    RUNTIME.with(|rt| {
        f(rt.borrow_mut()
            .as_mut()
            .expect("no runtime (use .set())")
            .lock())
    })
}

pub fn get_type(name: &str) -> Option<AMType> {
    runtime(|rt| rt.get_type_by_name(name))
}

/// An SPL runtime.
///
/// This holds:
/// - types
/// - type refs
/// - streams
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

    pub fn get_type_by_name(&self, name: &str) -> Option<AMType> {
        self.types_by_name.get(name).cloned()
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

/// Anything that can be .set() and result in the runtime being set.
/// Implemented for Arc<Mut<Runtime>> and Runtime.
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

/// A frame's location in SPL code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameInfo {
    pub file: String,
    pub function: String,
}

/// An SPL stack frame.
///
/// This holds:
/// - its parent
/// - variables
/// - functions
/// - its origin ([FrameInfo])
/// - whether all functions in it should be made global.
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
        f.write_str("\nFuncs: \n")?;
        for (name, ..) in self.functions.lock_ro().iter() {
            f.write_str("  ")?;
            f.write_str(name)?;
            f.write_str("\n")?;
        }
        f.write_str("\n\n")?;
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
                file: "std.spl".to_owned(),
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
                return Err(stack.error(ErrorKind::VariableNotFound(name)));
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
                return Err(stack.error(ErrorKind::VariableNotFound(name)));
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

/// An SPL stack.
///
/// This holds:
/// - a stack of frames
/// - the main stack of objects
/// - a return accumultor: how many blocks to return directly from
#[derive(Clone, Debug)]
pub struct Stack {
    frames: Vec<Arc<Frame>>,
    object_stack: Vec<AMObject>,
    files: Vec<String>,
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
            files: Vec::new(),
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
            files: Vec::new(),
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
                func.run_as_base,
            )
        } else {
            Frame::new(func.origin.clone(), func.name.clone())
        };
        self.frames.push(Arc::new(f));
        let r = func.to_call.call(self);
        self.frames.pop().unwrap();
        r
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
                return Err(self.error(ErrorKind::FuncNotFound(name)));
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
                run_as_base: false,
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
                run_as_base: false,
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
            mr_stack: self.mr_trace(),
        })
    }

    pub fn error(&self, kind: ErrorKind) -> Error {
        Error {
            kind,
            stack: self.trace(),
            mr_stack: self.mr_trace(),
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

    pub(crate) fn include_file(&mut self, s: &String) -> bool {
        if self.files.contains(s) {
            false
        } else {
            self.files.push(s.to_owned());
            true
        }
    }
}

/// An SPL keyword. Used to deviate from normal linear code structure.
///
/// This is different from a [Word], which are any SPL code.
#[derive(Clone, Debug)]
pub enum Keyword {
    /// <none>
    ///
    /// Dumps stack. Not available as actual keyword, therefore only obtainable through AST
    /// manipulation, a dyn call, or modding.
    /// equivalent to dyn-__dump
    /// example: func main { int | "Hello, world!" dyn-__dump pop 0 }
    Dump,
    /// def <name>
    ///
    /// Defines a variable.
    /// equivalent to "<name>" dyn-def
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
    Construct(String, Vec<String>, Vec<(String, (u32, Words))>, bool),
    /// include <typeA> in <typeB>
    ///
    /// Adds <typeA> as a parent type of <typeB>.
    /// equivalent to "<typeA>" "<typeB>" dyn-include
    Include(String, String),
    /// use <path>:<item>
    ///
    /// equivalent to "<path>:<item>" dyn-use
    Use(String),
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
    /// catch [<type> <...>] { <code> } with { <wordsOnCatch> }
    ///
    /// Catches errors that happen within <code>, running <wordsOnCatch> when an error is
    /// encountered and the error is of <type> (or, if no type is specified, any error).
    /// equivalent to \[ ["<type>" <...>] \] { | <code> } { | <wordsOnCatch> } dyn-catch
    Catch(Vec<String>, Words, Words),
}

/// Any SPL value that is not a construct.
///
/// Holds its rust representation.
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

/// The smallest fragment of SPL code.
#[derive(Clone, Debug)]
pub enum Word {
    /// A keyword, used to deviate from normal code structure.
    Key(Keyword),
    /// A constant to push to the stack when encountered.
    Const(Value),
    /// A function call.
    Call(String, bool, u32),
    /// A method call.
    ObjCall(String, bool, u32),
}

/// A collection of executable words.
#[derive(Clone, Debug)]
pub struct Words {
    pub words: Vec<Word>,
}

impl Words {
    pub fn new(words: Vec<Word>) -> Self {
        Words { words }
    }
}

/// Any kind of SPL-executable code.
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

/// Any kind of SPL-executable code with metadata surrounding it.
///
/// This holds:
/// - the amount of values returned when called
/// - the actual executable code ([FuncImpl])
/// - the frame that defined it
/// - the name of the file it was defined in, if this is different form the definition frame
/// - the name of the function.
/// - wether it should be run as the root layer (= wether functions it defines should be made
///   global)
#[derive(Clone)]
pub struct Func {
    pub ret_count: u32,
    pub to_call: FuncImpl,
    pub origin: Arc<Frame>,
    pub fname: Option<String>,
    pub name: String,
    pub run_as_base: bool,
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

/// Any SPL type.
///
/// This holds:
/// - the name
/// - the numeric ID
/// - its parent types
/// - its methods
/// - its fields
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
            if !object.property_map.contains_key(&property) {
                object.property_map.insert(property, Value::Null.spl());
            }
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
                origin: origin.clone(),
                run_as_base: false,
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
                origin,
                run_as_base: false,
                fname: Some("RUNTIME".to_owned()),
                name: "=".to_owned() + &name,
            }),
        );
        self.properties.push(name);
        Ok(())
    }
}

/// Any kind of SPL object, no matter if it is a construct or not.
///
/// This holds:
/// - the type of the object
/// - the fields mandated by the type
/// - the native value ([Value]), null for constructs unless set manually.
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
            return None;
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

    pub fn field(&self, name: &str, stack: &mut Stack) -> Result<AMObject, Error> {
        Ok(self
            .property_map
            .get(name)
            .ok_or_else(|| {
                stack.error(ErrorKind::PropertyNotFound(
                    self.kind.lock_ro().name.to_owned(),
                    name.to_owned(),
                ))
            })?
            .clone())
    }
}

impl From<String> for Object {
    fn from(value: String) -> Self {
        Value::Str(value).into()
    }
}

impl From<FrameInfo> for Object {
    fn from(value: FrameInfo) -> Self {
        let mut obj = Object::new(
            get_type("FrameInfo").expect("FrameInfo type must exist"),
            Value::Null,
        );
        obj.property_map.insert("file".to_owned(), value.file.spl());
        obj.property_map
            .insert("function".to_owned(), value.function.spl());
        obj
    }
}

impl<T> From<Vec<T>> for Object
where
    T: Into<Object>,
{
    fn from(value: Vec<T>) -> Self {
        Value::Array(value.into_iter().map(|x| x.spl()).collect()).into()
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

/// Trait for converting things to SPL Objects.
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

/// Finds a file in the SPL_PATH, or returns the internal [stdlib] version of it.
pub fn find_in_splpath(path: &str) -> Result<String, String> {
    if Path::new(path).exists() {
        return Ok(path.to_owned());
    }
    let s = var("SPL_PATH").unwrap_or("/usr/lib/spl".to_owned()) + "/" + path;
    if Path::new(&s).exists() {
        Ok(s)
    } else {
        match path {
            "std.spl" => Err(stdlib::STD.to_owned()),
            "iter.spl" => Err(stdlib::ITER.to_owned()),
            "stream.spl" => Err(stdlib::STREAM.to_owned()),
            _ => Ok(path.to_owned()),
        }
    }
}

impl Words {
    /// Executes the words. This does *not* create a new frame on the stack. Use [Stack::call] to
    /// call and create a new frame.
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
                            origin: stack.get_frame(),
                            run_as_base: false,
                            fname: None,
                            name,
                        }),
                    ),
                    Keyword::Construct(name, fields, methods, is_namespace) => {
                        let origin = stack.get_frame();
                        if !name.contains(':') {
                            stack.define_var(name.clone());
                        }
                        let t = runtime_mut(|mut rt| {
                            rt.make_type(name.clone(), |mut t| {
                                for field in fields {
                                    t.add_property(field, origin.clone())?;
                                }
                                t.functions.extend(methods.into_iter().map(|(k, v)| {
                                    (
                                        k.clone(),
                                        Arc::new(Func {
                                            ret_count: v.0,
                                            to_call: FuncImpl::SPL(v.1),
                                            origin: origin.clone(),
                                            run_as_base: false,
                                            fname: None,
                                            name: name.clone() + ":" + &k,
                                        }),
                                    )
                                }));
                                Ok(t)
                            })
                        })?;

                        let to_set: Object = if is_namespace {
                            let mut obj: Object = Value::Null.into();
                            obj.kind = t.clone();
                            t.lock_ro().write_into(&mut obj);
                            obj
                        } else {
                            Value::Str(t.lock_ro().get_name()).into()
                        };
                        if name.contains(':') {
                            let Some((a, mut name)) = name.split_once(':') else { unreachable!() };
                            let mut f = stack.get_var(a.to_owned())?;
                            while let Some((a, b)) = name.split_once(':') {
                                name = b;
                                let o = f.lock_ro();
                                let nf = o.field(a, stack)?;
                                mem::drop(o);
                                f = nf;
                            }
                            *f.lock_ro().field(name, stack)?.lock() = to_set;
                        } else {
                            stack.set_var(name.clone(), to_set.spl())?;
                        }
                    }
                    Keyword::Include(ta, tb) => {
                        let rstack = &stack;
                        runtime(move |rt| {
                            rt.get_type_by_name(&tb)
                                .ok_or_else(|| rstack.error(ErrorKind::TypeNotFound(tb)))?
                                .lock()
                                .parents
                                .push(
                                    rt.get_type_by_name(&ta)
                                        .ok_or_else(|| rstack.error(ErrorKind::TypeNotFound(ta)))?,
                                );
                            Ok(())
                        })?;
                    }
                    Keyword::Use(item) => {
                        if let Some((a, mut name)) = item.split_once(':') {
                            let mut f = stack.get_var(a.to_owned())?;
                            while let Some((a, b)) = name.split_once(':') {
                                name = b;
                                let o = f.lock_ro();
                                let nf = o.field(a, stack)?;
                                mem::drop(o);
                                f = nf;
                            }
                            stack.define_var(name.to_owned());
                            let o = f.lock_ro().field(name, stack)?.clone();
                            stack.set_var(name.to_owned(), o)?;
                        }
                    }
                    Keyword::While(cond, blk) => loop {
                        cond.exec(stack)?;
                        if !stack.pop().lock_ro().is_truthy() {
                            break;
                        }
                        blk.exec(stack)?;
                        if stack.return_accumultor > 0 {
                            stack.return_accumultor -= 1;
                            break;
                        }
                    },
                    Keyword::If(blk) => {
                        if stack.pop().lock_ro().is_truthy() {
                            blk.exec(stack)?;
                        }
                    }
                    Keyword::Catch(types, blk, ctch) => {
                        if let Err(e) = blk.exec(stack) {
                            if types.is_empty() || types.contains(&e.kind.to_string()) {
                                stack.push(e.spl());
                                ctch.exec(stack)?;
                            } else {
                                return Err(e);
                            }
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
                                origin: stack.get_frame(),
                                run_as_base: false,
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
                        .ok_or_else(|| {
                            stack.error(ErrorKind::MethodNotFound(f0.name.clone(), x.clone()))
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
                                origin: stack.get_frame(),
                                run_as_base: false,
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

/// Any error SPL can handle and throw.
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

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Parse(_, _) => f.write_str("Parse"),
            ErrorKind::InvalidCall(_) => f.write_str("InvalidCall"),
            ErrorKind::InvalidType(_, _) => f.write_str("InvalidType"),
            ErrorKind::VariableNotFound(_) => f.write_str("VariableNotFound"),
            ErrorKind::FuncNotFound(_) => f.write_str("FuncNotFound"),
            ErrorKind::MethodNotFound(_, _) => f.write_str("MethodNotFound"),
            ErrorKind::PropertyNotFound(_, _) => f.write_str("PropertyNotFound"),
            ErrorKind::TypeNotFound(_) => f.write_str("TypeNotFound"),
            ErrorKind::LexError(_) => f.write_str("LexError"),
            ErrorKind::IO(_) => f.write_str("IO"),
            ErrorKind::Custom(_) => f.write_str("Custom"),
            ErrorKind::CustomObject(_) => f.write_str("CustomObject"),
        }
    }
}

/// Wrapper for ErrorKind with the stack trace.
#[derive(PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub stack: Vec<String>,
    pub mr_stack: Vec<Vec<FrameInfo>>,
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("\n\nSPL PANIC DUE TO UNCAUGHT ERROR:\n")?;
        f.write_str(format!("Error: {:?}", self.kind).as_str())?;
        f.write_str("\n")?;
        f.write_str(self.stack.join("\n").as_str())?;
        f.write_str("\n\n")?;
        Ok(())
    }
}

impl From<Error> for Object {
    fn from(value: Error) -> Self {
        let mut obj = Object::new(
            get_type("error").expect("error type must exist"),
            Value::Null,
        );
        obj.property_map
            .insert("kind".to_owned(), value.kind.to_string().spl());
        obj.property_map
            .insert("message".to_owned(), format!("{:?}", value.kind).spl());
        if let ErrorKind::CustomObject(ref o) = value.kind {
            obj.property_map.insert("object".to_owned(), o.clone());
        }
        if let ErrorKind::Custom(ref s) = value.kind {
            obj.property_map
                .insert("message".to_owned(), s.clone().spl());
        }
        obj.property_map
            .insert("trace".to_owned(), value.stack.spl());
        obj.property_map
            .insert("mr-trace".to_owned(), value.mr_stack.spl());
        obj
    }
}
