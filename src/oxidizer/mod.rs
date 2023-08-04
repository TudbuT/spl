//! This module creates a rust application that runs the desired SPL.
//! At its current stage, this is just parsing and rewriting `@rust` functions from SPL into actual rust.
//! The future plan is for this to effectively become a compiler.

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env, fs,
    hash::{Hash, Hasher},
    io,
    process::{Child, Command},
};

use crate::{FuncImplType, Keyword, Word, Words};

mod splrs;

/// A specially compiled SPL version with custom parameters included.
pub struct RustApp {
    /// The path to the binary
    binary: String,
}

impl RustApp {
    /// Gets the path to the binary
    pub fn get_binary(&self) -> &str {
        &self.binary
    }

    /// Executes the binary with some args
    pub fn execute(&self, args: Vec<&str>) -> Result<Child, io::Error> {
        Command::new(self.binary.clone()).args(args).spawn()
    }
}

/// A rust function which was embedded in SPL
pub struct RustFunction {
    fn_name: String,
    content: String,
}

/// A builder for [`RustApp`]s. This is work-in-progress.
pub struct RustAppBuilder {
    rust_functions: Vec<RustFunction>,
    to_embed: HashMap<String, String>,
    default_file: String,
    name: Option<String>,
}

impl Hash for RustAppBuilder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.rust_functions.len());
        for f in &self.rust_functions {
            f.fn_name.hash(state);
        }
        for (k, _) in &self.to_embed {
            k.hash(state);
        }
    }
}

impl RustAppBuilder {
    pub fn new() -> RustAppBuilder {
        Self {
            rust_functions: Vec::new(),
            to_embed: HashMap::new(),
            default_file: "repl.spl".to_owned(),
            name: None,
        }
    }

    /// Embeds a file into the desired app
    pub fn add_source(&mut self, name: String, source: String) {
        self.to_embed.insert(name, source);
    }

    /// Sets the name of the folder it will sit in.
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Adds all `@rust` functions from the given SPL code's top level. Does NOT scan for lower levels at this time.
    pub fn prepare(&mut self, spl: Words) -> bool {
        let mut needs_new = false;
        for word in spl.words {
            match word {
                Word::Key(Keyword::FuncOf(name, content, FuncImplType::Rust)) => {
                    self.rust_functions.push(splrs::to_rust(name, content));
                    needs_new = true;
                }
                _ => (),
            }
        }
        needs_new
    }

    /// Sets the default file to start when none is provided. This will not work if the file is not also embedded.
    pub fn set_default_file(&mut self, name: String) {
        self.default_file = name;
    }

    /// Builds the desired app, including literally building it using cargo.
    pub fn build(self) -> Result<RustApp, io::Error> {
        // we need a temp folder!
        let tmp = "."; // TODO replace?
        let name = match self.name {
            Some(x) => x,
            None => {
                let mut hash = DefaultHasher::new();
                self.hash(&mut hash);
                let hash = hash.finish();
                hash.to_string()
            }
        };
        let _ = Command::new("cargo")
            .arg("new")
            .arg(format!("spl-{name}"))
            .current_dir(tmp)
            .spawn()
            .unwrap()
            .wait_with_output();
        Command::new("cargo")
            .arg("add")
            .arg(format!("spl@{}", env!("CARGO_PKG_VERSION")))
            .current_dir(format!("{tmp}/spl-{name}"))
            .spawn()
            .unwrap()
            .wait_with_output()?;
        let mut runtime_init = String::new();
        let mut code = String::new();
        for func in self.rust_functions.into_iter().enumerate() {
            code += &format!(
                "fn spl_oxidizer_{}(stack: &mut Stack) -> OError {{ {} Ok(()) }}",
                func.0, func.1.content
            );
            runtime_init += &format!(
                "rt.native_functions.insert({:?}, (0, FuncImpl::Native(spl_oxidizer_{})));",
                func.1.fn_name, func.0
            )
        }
        for (name, data) in self.to_embed.into_iter() {
            runtime_init += &format!("rt.embedded_files.insert({:?}, {:?});", name, data);
        }
        fs::write(
            format!("{tmp}/spl-{name}/src/main.rs"),
            stringify! {
                use spl::{runtime::*, *};

                use std::env::args;

                pub fn start_file(path: &str) -> Result<Stack, Error> {
                    let mut rt = Runtime::new();
                    runtime_init
                    rt.set();
                    (start_file_in_runtime(path), Runtime::reset()).0
                }

                fn main() {
                    if let Err(x) = start_file(
                        &args()
                            .nth(1)
                            .unwrap_or_else(|| find_in_splpath("default_file").expect("no file to be run")),
                    ) {
                        println!("{x:?}");
                    }
                }
            }.to_owned().replace("default_file", &self.default_file).replace("runtime_init", &runtime_init) + &code,
        )?;
        Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(format!("{tmp}/spl-{name}"))
            .spawn()
            .unwrap()
            .wait_with_output()?;
        Ok(RustApp {
            binary: {
                // insanity. will have to clean this up at some point.
                let dir = format!("{tmp}/spl-{name}/target/release/");
                fs::read_dir(dir)
                    .expect("unable to build: dir was not created.")
                    .filter(|x| {
                        let x = x
                            .as_ref()
                            .expect("file system did something i cannot comprehend");
                        let n = x.file_name().into_string().unwrap();
                        x.file_type().expect("file system uhhh?????").is_file()
                            && !n.ends_with(".d")
                            && !n.starts_with(".")
                    })
                    .next()
                    .expect("cargo was unable to build the binary")
                    .expect("file system did something i cannot comprehend")
                    .path()
                    .into_os_string()
                    .into_string()
                    .expect("bad unicode in file path")
            },
        })
    }
}

impl Default for RustAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
