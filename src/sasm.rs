use std::sync::Arc;

use crate::{Frame, Func, FuncImpl, Keyword, Value, Word, Words};

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
                    Some(x) => (|x| x == "namespace")(x),
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
