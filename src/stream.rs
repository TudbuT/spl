use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Read,
    io::Write,
    mem,
    net::{Shutdown, TcpStream},
    sync::Arc,
};

use once_cell::sync::Lazy;

use crate::{mutex::Mut, runtime::*, *};

static STREAM_TYPES: Lazy<Arc<Mut<HashMap<String, StreamType>>>> =
    Lazy::new(|| Arc::new(Mut::new(HashMap::new())));
static IS_INITIALIZED: Lazy<Arc<Mut<bool>>> = Lazy::new(|| Arc::new(Mut::new(false)));

/// Registers a custom stream type.
pub fn register_stream_type(
    name: &str,
    supplier: impl Fn(&mut Stack) -> Result<Stream, Error> + Sync + Send + 'static,
) {
    STREAM_TYPES
        .lock()
        .insert(name.to_owned(), StreamType::from(supplier));
}

/// Gets a stream type by name.
pub fn get_stream_type(name: String) -> Option<StreamType> {
    STREAM_TYPES.lock_ro().get(&name).cloned()
}

/// An SPL stream type.
#[derive(Clone)]
pub struct StreamType {
    func: Arc<Box<dyn Fn(&mut Stack) -> Result<Stream, Error> + Sync + Send + 'static>>,
}

impl StreamType {
    pub fn make_stream(&self, stack: &mut Stack) -> Result<Stream, Error> {
        (self.func)(stack)
    }
}

/// An SPL stream, holding a reader and a writer, and a function to close it.
pub struct Stream {
    reader: Box<dyn Read + 'static>,
    writer: Box<dyn Write + 'static>,
    close: fn(&mut Self),
}

impl Stream {
    pub fn new<T: Read + Write + 'static>(main: T, close: fn(&mut Self)) -> Self {
        let mut rw = Box::new(main);
        Self {
            // SAFETY: Because these are both in private fields on one object, they can not be
            // written to simultaneously or read from while writing due to the guards put in place
            // by the borrow checker on the Stream.
            reader: Box::new(unsafe { mem::transmute::<&mut _, &mut T>(rw.as_mut()) }),
            writer: rw,
            close,
        }
    }
    pub fn new_split(
        reader: impl Read + 'static,
        writer: impl Write + 'static,
        close: fn(&mut Self),
    ) -> Self {
        Self {
            reader: Box::new(reader),
            writer: Box::new(writer),
            close,
        }
    }
}

impl Read for Stream {
    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.reader.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.reader.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.reader.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.reader.read_exact(buf)
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Write for Stream {
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.writer.write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(buf)
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.writer.write_fmt(fmt)
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<T> From<T> for StreamType
where
    T: Fn(&mut Stack) -> Result<Stream, Error> + Sync + Send + 'static,
{
    fn from(value: T) -> Self {
        StreamType {
            func: Arc::new(Box::new(value)),
        }
    }
}

pub fn new_stream(stack: &mut Stack) -> OError {
    require_on_stack!(s, Str, stack, "write-stream");
    let stream = get_stream_type(s.clone())
        .ok_or_else(|| stack.error(ErrorKind::VariableNotFound(format!("__stream-type-{s}"))))?
        .make_stream(stack)?;
    let stream = runtime_mut(move |mut rt| Ok(rt.register_stream(stream)))?;
    stack.push(Value::Mega(stream.0 as i128).spl());
    Ok(())
}

pub fn write_stream(stack: &mut Stack) -> OError {
    require_on_stack!(id, Mega, stack, "write-stream");
    require_array_on_stack!(a, stack, "write-stream");
    let stream = runtime(|rt| {
        rt.get_stream(id as u128)
            .ok_or_else(|| stack.error(ErrorKind::VariableNotFound(format!("__stream-{id}"))))
    })?;
    let mut fixed = Vec::with_capacity(a.len());
    for item in a.iter() {
        match item.lock_ro().native {
            Value::Int(x) => fixed.push(x as u8),
            _ => type_err!(stack, "!int", "int"),
        }
    }
    stack.push(
        Value::Mega(
            stream
                .lock()
                .write(&fixed[..])
                .map_err(|x| stack.error(ErrorKind::IO(format!("{x:?}"))))? as i128,
        )
        .spl(),
    );
    Ok(())
}

pub fn write_all_stream(stack: &mut Stack) -> OError {
    require_on_stack!(id, Mega, stack, "write-all-stream");
    require_array_on_stack!(a, stack, "write-all-stream");
    let stream = runtime(|rt| {
        rt.get_stream(id as u128)
            .ok_or_else(|| stack.error(ErrorKind::VariableNotFound(format!("__stream-{id}"))))
    })?;
    let mut fixed = Vec::with_capacity(a.len());
    for item in a.iter() {
        match item.lock_ro().native {
            Value::Int(x) => fixed.push(x as u8),
            _ => type_err!(stack, "!int", "int"),
        }
    }
    stream
        .lock()
        .write_all(&fixed[..])
        .map_err(|x| stack.error(ErrorKind::IO(format!("{x:?}"))))?;
    Ok(())
}

pub fn read_stream(stack: &mut Stack) -> OError {
    require_on_stack!(id, Mega, stack, "read-stream");
    let array = stack.pop();
    {
        require_mut_array!(a, array, stack, "read-stream");
        let stream = runtime(|rt| {
            rt.get_stream(id as u128)
                .ok_or_else(|| stack.error(ErrorKind::VariableNotFound(format!("__stream-{id}"))))
        })?;
        let mut vec = vec![0; a.len()];
        stack.push(
            Value::Mega(
                stream
                    .lock()
                    .read(&mut vec[..])
                    .map_err(|x| stack.error(ErrorKind::IO(format!("{x:?}"))))?
                    as i128,
            )
            .spl(),
        );
        a.clone_from_slice(
            &vec.into_iter()
                .map(|x| Value::Int(x as i32).spl())
                .collect::<Vec<_>>(),
        );
    }
    stack.push(array);
    Ok(())
}

pub fn read_all_stream(stack: &mut Stack) -> OError {
    require_on_stack!(id, Mega, stack, "read-all-stream");
    let array = stack.pop();
    {
        require_mut_array!(a, array, stack, "read-all-stream");
        let stream = runtime(|rt| {
            rt.get_stream(id as u128)
                .ok_or_else(|| stack.error(ErrorKind::VariableNotFound(format!("__stream-{id}"))))
        })?;
        let mut vec = vec![0; a.len()];
        stream
            .lock()
            .read_exact(&mut vec[..])
            .map_err(|x| stack.error(ErrorKind::IO(format!("{x:?}"))))?;
        a.clone_from_slice(
            &vec.into_iter()
                .map(|x| Value::Int(x as i32).spl())
                .collect::<Vec<_>>(),
        );
    }
    stack.push(array);
    Ok(())
}

pub fn close_stream(stack: &mut Stack) -> OError {
    require_on_stack!(id, Mega, stack, "close-stream");
    if let Some(stream) = runtime(|rt| rt.get_stream(id as u128)) {
        let mut stream = stream.lock();
        (stream.close)(&mut stream);
    }
    Ok(())
}

fn nop(_stream: &mut Stream) {}

fn stream_file(stack: &mut Stack) -> Result<Stream, Error> {
    let truncate = stack.pop().lock_ro().is_truthy();
    require_on_stack!(path, Str, stack, "FILE new-stream");
    Ok(Stream::new(
        OpenOptions::new()
            .read(!truncate)
            .write(true)
            .create(truncate)
            .truncate(truncate)
            .open(path)
            .map_err(|x| stack.error(ErrorKind::IO(x.to_string())))?,
        nop,
    ))
}

fn stream_tcp(stack: &mut Stack) -> Result<Stream, Error> {
    require_int_on_stack!(port, stack, "TCP new-stream");
    require_on_stack!(ip, Str, stack, "TCP new-stream");
    fn close_tcp(stream: &mut Stream) {
        unsafe {
            let f = ((stream.reader.as_mut() as *mut dyn Read).cast() as *mut TcpStream)
                .as_mut()
                .unwrap();
            let _ = f.shutdown(Shutdown::Both);
        }
    }
    Ok(Stream::new(
        TcpStream::connect((ip, port as u16))
            .map_err(|x| stack.error(ErrorKind::IO(x.to_string())))?,
        close_tcp,
    ))
}

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    if !*IS_INITIALIZED.lock_ro() {
        register_stream_type("file", stream_file);
        register_stream_type("tcp", stream_tcp);
        *IS_INITIALIZED.lock() = true;
    }

    type Fn = fn(&mut Stack) -> OError;
    let fns: [(&str, Fn, u32); 6] = [
        ("new-stream", new_stream, 1),
        ("write-stream", write_stream, 1),
        ("write-all-stream", write_all_stream, 0),
        ("read-stream", read_stream, 1),
        ("read-all-stream", read_all_stream, 0),
        ("close-stream", close_stream, 0),
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
