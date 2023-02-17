use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter, Pointer, Debug},
    sync::{Arc, Mutex},
    vec,
};

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

#[derive(Clone)]
struct Runtime {
    next_type_id: u32,
    types_by_name: HashMap<String, Arc<Type>>,
    types_by_id: HashMap<u32, Arc<Type>>,
}

impl Runtime {
    pub fn new() -> Self {
        let mut rt = Runtime {
            next_type_id: 0,
            types_by_name: HashMap::new(),
            types_by_id: HashMap::new(),
        };
        rt.make_type("null".to_owned(), |t|t);
        rt.make_type("int".to_owned(), |t|t);
        rt.make_type("long".to_owned(), |t|t);
        rt.make_type("mega".to_owned(), |t|t);
        rt.make_type("func".to_owned(), |t|t);
        rt.make_type("array".to_owned(), |t|t);
        rt.make_type("str".to_owned(), |t|t);
        rt
    }

    pub fn get_type_by_name(&self, name: String) -> Option<Arc<Type>> {
        self.types_by_name.get(&name).cloned()
    }

    pub fn get_type_by_id(&self, id: u32) -> Option<Arc<Type>> {
        self.types_by_id.get(&id).cloned()
    }

    pub fn make_type(&mut self, name: String, op: impl FnOnce(Type) -> Type) -> Arc<Type> {
        let t = Arc::new(op(Type {
            name: name.clone(),
            id: (self.next_type_id, self.next_type_id += 1).0,
            functions: HashMap::new(),
            properties: Vec::new(),
        }));
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
struct FrameInfo {
    file: String,
}

#[derive(Clone)]
struct Frame {
    parent: Option<Arc<Frame>>,
    object_stack: Vec<Arc<Mutex<Object>>>,
    variables: HashMap<String, Arc<Mutex<Object>>>,
    origin: FrameInfo,
}

impl Display for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Stack: \n")?;
        for object in &self.object_stack {
            f.write_str("  ")?;
            object.lock().unwrap().fmt(f)?;
            f.write_str("\n")?;
        }
        f.write_str("\nVars: \n")?;
        for (name, object) in &self.variables {
            f.write_str("  ")?;
            f.write_str(&name)?;
            f.write_str(": ")?;
            object.lock().unwrap().fmt(f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl Frame {
    fn root() -> Self {
        Frame {
            parent: None,
            object_stack: Vec::new(),
            variables: HashMap::new(),
            origin: FrameInfo {
                file: "RUNTIME".to_owned(),
            },
        }
    }

    pub fn new(parent: Arc<Frame>, origin: FrameInfo) -> Self {
        Frame {
            parent: Some(parent),
            object_stack: Vec::new(),
            variables: HashMap::new(),
            origin,
        }
    }
}

#[derive(Clone)]
struct Stack {
    frames: Vec<Frame>,
}

impl Display for Stack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for frame in &self.frames {
            f.write_str("Frame: ")?;
            f.write_str(&frame.origin.file)?;
            f.write_str("\n\n")?;
            frame.fmt(f)?;
        }
        Ok(())
    }
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            frames: vec![Frame::root()],
        }
    }

    pub fn push(&mut self, obj: Arc<Mutex<Object>>) {
        self.frames
            .last_mut()
            .expect("program end reached but stack still being used")
            .object_stack
            .push(obj)
    }
}

#[derive(Clone)]
enum Keyword {
    DUMP,
}

#[derive(Clone, Debug)]
enum Constant {
    Int(i32),
    Long(i64),
    Mega(i128),
    Str(String),
    Func(Func),
    Null,
}

#[derive(Clone)]
enum Word {
    Key(Keyword),
    Const(Constant),
    GetPointer(String),
    ObjGetPointer(String),
    Call(String),
    ObjCall(String),
}

#[derive(Clone)]
struct Words {
    words: Vec<Word>,
}

#[derive(Clone)]
enum FuncImpl {
    NATIVE(fn(&mut Stack)),
    SPL(Words),
}

#[derive(Clone)]
struct Func {
    arg_count: u32,
    to_call: FuncImpl,
}

impl Debug for Func {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.arg_count.to_string())?;
        Ok(())
    }
}

#[derive(Clone)]
struct Type {
    name: String,
    id: u32,
    functions: HashMap<String, Func>,
    properties: Vec<String>,
}

#[derive(Clone)]
struct Object {
    kind: Arc<Type>,
    property_map: HashMap<String, Arc<Mutex<Object>>>,
    native: Constant,
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.kind.name)?;
        f.write_str("(")?;
        self.native.fmt(f)?;
        f.write_str(") { ")?;
        for (k, v) in &self.property_map {
            f.write_str(&k)?;
            f.write_str(": ")?;
            v.fmt(f)?;
        }
        f.write_str(" }")?;
        Ok(())
    }
}

impl Object {
    pub fn new(kind: Arc<Type>, native: Constant) -> Object {
        Object {
            property_map: {
                let mut map = HashMap::new();
                for property in &kind.properties {
                    map.insert(property.clone(), Constant::Null.spl());
                }
                map
            },
            kind,
            native,
        }
    }
}

impl From<Constant> for Object {
    fn from(value: Constant) -> Self {
        Object::new(
            RUNTIME.with(|x| {
                let x = x.borrow();
                let x = x.as_ref().unwrap();
                match value {
                    Constant::Null => x.get_type_by_id(0),
                    Constant::Int(_) => x.get_type_by_id(1),
                    Constant::Long(_) => x.get_type_by_id(2),
                    Constant::Mega(_) => x.get_type_by_id(3),
                    Constant::Func(_) => x.get_type_by_id(4),
                    // array is 5
                    Constant::Str(_) => x.get_type_by_id(6),
                }
                .expect("runtime uninitialized: default types not set.")
            }),
            value,
        )
    }
}

trait SPL {
    fn spl(self) -> Arc<Mutex<Object>>;
}

impl<T> SPL for T
where
    T: Into<Object>,
{
    fn spl(self) -> Arc<Mutex<Object>> {
        Arc::new(Mutex::new(self.into()))
    }
}

impl Words {
    pub fn exec(&self, stack: &mut Stack) {
        for word in &self.words {
            match word {
                Word::Key(x) => match x {
                    Keyword::DUMP => println!("{}", stack),
                },
                Word::Const(x) => stack.push(x.clone().spl()),
                Word::GetPointer(_) => todo!(),
                Word::ObjGetPointer(_) => todo!(),
                Word::Call(_) => todo!(),
                Word::ObjCall(_) => todo!(),
            }
        }
    }
}

fn main() {
    let rt = Runtime::new();
    rt.set();
    Words {
        words: vec![
            Word::Const(Constant::Str("Hello, World".to_owned())),
            Word::Key(Keyword::DUMP),
        ],
    }
    .exec(&mut Stack::new());
    Runtime::reset();
}
