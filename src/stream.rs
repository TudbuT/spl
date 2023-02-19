use std::{collections::HashMap, io::Read, io::Write, mem, sync::Arc};

use once_cell::sync::Lazy;

use crate::{mutex::Mut, runtime::*, type_err};

static STREAM_TYPES: Lazy<Arc<Mut<HashMap<String, StreamType>>>> =
    Lazy::new(|| Arc::new(Mut::new(HashMap::new())));

pub fn register_stream_type(
    name: &str,
    supplier: impl Fn(&mut Stack) -> Result<Stream, Error> + Sync + Send + 'static,
) {
    STREAM_TYPES
        .lock()
        .insert(name.to_owned(), StreamType::from(supplier));
}

pub fn get_stream_type(name: String) -> Option<StreamType> {
    STREAM_TYPES.lock_ro().get(&name).cloned()
}

#[derive(Clone)]
pub struct StreamType {
    func: Arc<Box<dyn Fn(&mut Stack) -> Result<Stream, Error> + Sync + Send + 'static>>,
}

impl StreamType {
    pub fn make_stream(&self, stack: &mut Stack) -> Result<Stream, Error> {
        (self.func)(stack)
    }
}

pub struct Stream {
    reader: Box<dyn Read + 'static>,
    writer: Box<dyn Write + 'static>,
}

impl Stream {
    pub fn new<T: Read + Write + 'static>(main: T) -> Self {
        let mut rw = Box::new(main);
        Self {
            // SAFETY: Because these are both in private fields on one object, they can not be
            // written to simultaneously or read from while writing due to the guards put in place
            // by the borrow checker on the Stream.
            reader: Box::new(unsafe { mem::transmute::<&mut _, &mut T>(rw.as_mut()) }),
            writer: rw,
        }
    }
    pub fn new_split(reader: impl Read + 'static, writer: impl Write + 'static) -> Self {
        Self {
            reader: Box::new(reader),
            writer: Box::new(writer),
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
    let Value::Str(s) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("new-stream".to_owned()))
    };
    let stream = runtime(|mut rt| {
        Ok(rt.register_stream(
            get_stream_type(s.clone())
                .ok_or_else(|| {
                    stack.error(ErrorKind::VariableNotFound(format!("__stream-type-{s}")))
                })?
                .make_stream(stack)?,
        ))
    })?;
    stack.push(Value::Mega(stream.0 as i128).spl());
    Ok(())
}

pub fn write_to_stream(stack: &mut Stack) -> OError {
    let binding = stack.pop();
    let Value::Array(ref a) = binding.lock_ro().native else {
        return stack.err(ErrorKind::InvalidCall("write-to-stream".to_owned()))
    };
    let Value::Mega(id) = stack.pop().lock_ro().native.clone() else {
        return stack.err(ErrorKind::InvalidCall("write-to-stream".to_owned()))
    };
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

pub fn register(r: &mut Stack, o: Arc<Frame>) {
    type Fn = fn(&mut Stack) -> OError;
    let fns: [(&str, Fn, u32); 1] = [("new-stream", new_stream, 1)];
    for f in fns {
        r.define_func(
            f.0.to_owned(),
            AFunc::new(Func {
                ret_count: f.2,
                to_call: FuncImpl::Native(f.1),
                run_at_base: false,
                origin: o.clone(),
                fname: None,
                name: f.0.to_owned(),
            }),
        );
    }
}
