use std::{sync::Arc, time::SystemTime};

use crate::{Frame, Func, FuncImpl, FuncImplType, Keyword, Value, Word, Words};

/// Reads sasm, the text representation of an SPL AST.
pub fn sasm_read(s: String) -> Words {
    let mut lines = s.split("\n");
    sasm_read_func(&mut lines)
}

pub fn sasm_read_func<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Words {
    let mut words = Vec::new();
    while let Some(line) = lines.next() {
        let line = line.trim_start();
        if line == "end" {
            break;
        }
        sasm_parse(line, &mut words, lines);
    }
    Words::new(words)
}

fn sasm_parse<'a>(line: &str, words: &mut Vec<Word>, lines: &mut impl Iterator<Item = &'a str>) {
    let line: Vec<_> = line.split(" ").collect();
    match line[0] {
        "dump" => words.push(Word::Key(Keyword::Dump)),
        "def" => words.push(Word::Key(Keyword::Def(line[1].to_owned()))),
        "func" => words.push(Word::Key(Keyword::Func(
            line[1].to_owned(),
            line[2].parse().expect("invalid sasm func: func ... NAN"),
            sasm_read_func(lines),
        ))),
        "construct" => {
            let name = line[1].to_owned();
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            let mut iter = line.into_iter().skip(2);
            for word in &mut iter {
                if word == ";" {
                    break;
                }
                fields.push(word.to_owned());
            }
            while let Some(word) = iter.next() {
                if word == ";" {
                    break;
                }
                methods.push((
                    word.to_owned(),
                    (
                        iter.next()
                            .map(|x| x.parse().ok())
                            .flatten()
                            .expect("invalid sasm construct: construct .... ; ... NAN ...."),
                        sasm_read_func(lines),
                    ),
                ));
            }
            words.push(Word::Key(Keyword::Construct(
                name,
                fields,
                methods,
                match iter.next() {
                    None => false,
                    Some(x) => x == "namespace",
                },
            )))
        }
        "include" => words.push(Word::Key(Keyword::Include(
            line[1].to_owned(),
            line[2].to_owned(),
        ))),
        "use" => words.push(Word::Key(Keyword::Use(line[1].to_owned()))),
        "while" => words.push(Word::Key(Keyword::While(
            sasm_read_func(lines),
            sasm_read_func(lines),
        ))),
        "if" => words.push(Word::Key(Keyword::If(sasm_read_func(lines)))),
        "with" => words.push(Word::Key(Keyword::With(
            line.into_iter().skip(1).map(ToOwned::to_owned).collect(),
        ))),
        "catch" => words.push(Word::Key(Keyword::Catch(
            line.into_iter().skip(1).map(ToOwned::to_owned).collect(),
            sasm_read_func(lines),
            sasm_read_func(lines),
        ))),
        "objpush" => words.push(Word::Key(Keyword::ObjPush)),
        "objpop" => words.push(Word::Key(Keyword::ObjPop)),
        "func_of_Rust" => {
            // output += &format!(
            //     "func_of_{kind:?} {name} {t}\0\0{}\0\0end {t}\n",
            //     content.replace("\0", "\0\x01").replace("\n", "\0\0")
            // );
            let name = line[1];
            let marker = line[2];
            let mut s = String::new();
            let mut line;
            while (
                line = lines.next().expect("sasm string without end marker"),
                line,
            )
                .1
                != "end ".to_owned() + marker
            {
                s = s + line + "\n";
            }
            if let Some(l) = s.strip_suffix("\n") {
                s = l.to_owned();
            }
            words.push(Word::Key(Keyword::FuncOf(
                name.to_owned(),
                s,
                FuncImplType::Rust,
            )));
        }
        // main words
        "const" => match line[1] {
            "str" => {
                let marker = line[2];
                let mut s = String::new();
                let mut line;
                while (
                    line = lines.next().expect("sasm string without end marker"),
                    line,
                )
                    .1
                    != "end ".to_owned() + marker
                {
                    s = s + line + "\n";
                }
                if let Some(l) = s.strip_suffix("\n") {
                    s = l.to_owned();
                }
                words.push(Word::Const(Value::Str(s)));
            }
            "int" => {
                words.push(Word::Const(Value::Int(
                    line[2].parse().expect("invalid sasm const: const int NAN"),
                )));
            }
            "long" => {
                words.push(Word::Const(Value::Long(
                    line[2].parse().expect("invalid sasm const: const long NAN"),
                )));
            }
            "mega" => {
                words.push(Word::Const(Value::Mega(
                    line[2].parse().expect("invalid sasm const: const mega NAN"),
                )));
            }
            "float" => {
                words.push(Word::Const(Value::Float(
                    line[2]
                        .parse()
                        .expect("invalid sasm const: const float NAN"),
                )));
            }
            "double" => {
                words.push(Word::Const(Value::Double(
                    line[2]
                        .parse()
                        .expect("invalid sasm const: const double NAN"),
                )));
            }
            "func" => words.push(Word::Const(Value::Func(Arc::new(Func {
                ret_count: line[2].parse().expect("invalid sasm const: const fun NAN"),
                to_call: FuncImpl::SPL(sasm_read_func(lines)),
                origin: Arc::new(Frame::dummy()),
                fname: None,
                name: "dyn".to_owned(),
                run_as_base: false,
            })))),
            "null" => words.push(Word::Const(Value::Null)),
            "array" => panic!("invalid sasm const: array - not all Values can be consts!"),
            _ => panic!("invalid sasm const: {}", line[1]),
        },
        "call" => {
            let name = line[1].to_owned();
            let mut rem = false;
            let mut ra = 0;
            for word in line.into_iter().skip(2) {
                if word == "pop" {
                    rem = true;
                } else if word == "ref" {
                    ra += 1;
                } else {
                    panic!("invalid sasm call: words after name must be either `pop` or `ref`");
                }
            }
            words.push(Word::Call(name, rem, ra));
        }
        "objcall" => {
            let name = line[1].to_owned();
            let mut rem = false;
            let mut ra = 0;
            for word in line.into_iter().skip(2) {
                if word == "pop" {
                    rem = true;
                } else if word == "ref" {
                    ra += 1;
                } else {
                    panic!("invalid sasm objcall: words after name must be either `pop` or `ref`");
                }
            }
            words.push(Word::ObjCall(name, rem, ra));
        }
        "" => {}
        _ => panic!("invalid sasm instruction: {}", line[0]),
    }
}

pub fn sasm_write(words: Words) -> String {
    sasm_write_func(words)
        .replace("\0\0", "\n")
        .replace("\0\x01", "\0")
}

fn sasm_write_func(words: Words) -> String {
    let mut output = String::new();
    for word in words.words {
        match word {
            Word::Key(word) => match word {
                Keyword::Dump => {
                    output += "dump\n";
                }
                Keyword::Def(x) => {
                    output += "def ";
                    output += &x;
                    output += "\n";
                }
                Keyword::Func(name, returns, text) => {
                    output += &format!("func {name} {returns}\n\t");
                    let text = sasm_write_func(text).replace("\n", "\n\t");
                    let text = text.trim_end();
                    output += &text;
                    output += "\nend\n";
                }
                Keyword::Construct(name, vars, methods, is_namespace) => {
                    output += &format!("construct {name} ");
                    for var in vars {
                        output += &var;
                        output += " ";
                    }
                    output += ";";
                    for method in &methods {
                        output += " ";
                        output += &method.0;
                        output += " ";
                        output += &method.1 .0.to_string();
                    }
                    if is_namespace {
                        output += " ; namespace";
                    }
                    output += "\n";
                    for method in methods {
                        output += "\t";
                        output += &sasm_write_func(method.1 .1)
                            .replace("\n", "\n\t")
                            .trim_end();
                        output += "\nend\n";
                    }
                }
                Keyword::Include(type_to_include, t) => {
                    output += &format!("include {type_to_include} {t}\n");
                }
                Keyword::Use(path) => output += &format!("use {path}\n"),
                Keyword::While(cond, blk) => {
                    output += "while\n\t";
                    output += &sasm_write_func(cond).replace("\n", "\n\t").trim_end();
                    output += "\nend\n\t";
                    output += &sasm_write_func(blk).replace("\n", "\n\t").trim_end();
                    output += "\nend\n";
                }
                Keyword::If(blk) => {
                    output += "if\n\t";
                    output += &sasm_write_func(blk).replace("\n", "\n\t").trim_end();
                    output += "\nend\n";
                }
                Keyword::With(items) => {
                    output += "with";
                    for item in items {
                        output += " ";
                        output += &item;
                    }
                    output += "\n";
                }
                Keyword::Catch(kinds, blk, ctch) => {
                    output += "catch";
                    for kind in kinds {
                        output += " ";
                        output += &kind;
                    }
                    output += "\n\t";
                    output += &sasm_write_func(blk).replace("\n", "\n\t").trim_end();
                    output += "\nend\n\t";
                    output += &sasm_write_func(ctch).replace("\n", "\n\t").trim_end();
                    output += "\nend\n";
                }
                Keyword::ObjPush => output += "objpush\n",
                Keyword::ObjPop => output += "objpop\n",
                Keyword::FuncOf(name, content, kind) => {
                    fn time() -> String {
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_micros()
                            .to_string()
                    }
                    let mut t = time();
                    while content.contains(&t) {
                        t = time();
                    }
                    output += &format!(
                        "func_of_{kind:?} {name} {t}\0\0{}\0\0end {t}\n",
                        content.replace("\0", "\0\x01").replace("\n", "\0\0")
                    );
                }
            },
            Word::Const(item) => match item {
                Value::Null => output += "const null\n",
                Value::Int(x) => output += &format!("const int {x}\n"),
                Value::Long(x) => output += &format!("const long {x}\n"),
                Value::Mega(x) => output += &format!("const mega {x}\n"),
                Value::Float(x) => output += &format!("const float {x}\n"),
                Value::Double(x) => output += &format!("const double {x}\n"),
                Value::Func(x) => {
                    let text = match &x.to_call {
                        FuncImpl::Native(_) => panic!("sasm can't write native function"),
                        FuncImpl::NativeDyn(_) => panic!("sasm can't write native function"),
                        FuncImpl::SPL(x) => sasm_write_func(x.clone()).replace("\n", "\n\t"),
                    };
                    let text = text.trim_end();
                    output += &format!("const func {}\n\t{}\nend\n", x.ret_count, text);
                }
                Value::Array(_) => panic!("sasm can't write arrays"),
                Value::Str(text) => {
                    fn time() -> String {
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_micros()
                            .to_string()
                    }
                    let mut t = time();
                    while text.contains(&t) {
                        t = time();
                    }
                    output += &format!(
                        "const str {t}\0\0{}\0\0end {t}\n",
                        text.replace("\0", "\0\x01").replace("\n", "\0\0")
                    );
                }
            },
            Word::Call(name, rem, ra) => {
                output += "call ";
                output += &name;
                if rem {
                    output += " pop";
                }
                for _ in 0..ra {
                    output += " ref";
                }
                output += "\n";
            }
            Word::ObjCall(name, rem, ra) => {
                output += "objcall ";
                output += &name;
                if rem {
                    output += " pop";
                }
                for _ in 0..ra {
                    output += " ref";
                }
                output += "\n";
            }
        }
    }
    output
}
