use std::sync::Arc;

use crate::runtime::*;
use readformat::*;

#[derive(Debug, PartialEq, Eq)]
pub enum LexerError {}

pub fn lex(input: String, filename: String, frame: Arc<Frame>) -> Result<Words, LexerError> {
    let mut str_words = Vec::new();
    for line in input.split('\n') {
        str_words.append(&mut parse_line(line));
    }
    Ok(read_block(
        &str_words[..],
        false,
        Arc::new(Frame::new_in(frame, filename)),
    )?
    .1)
}

fn read_block(
    str_words: &[String],
    isfn: bool,
    origin: Arc<Frame>,
) -> Result<(Option<u32>, Words, usize), LexerError> {
    let mut rem = None;
    let mut words = Vec::new();
    let mut i = 0;
    if str_words[0] == "{" && isfn {
        let mut r = 0_u32;
        while str_words[r as usize + 1] != "|" {
            r += 1;
        }
        i += r as usize + 2;
        rem = Some(r);
    }
    while i < str_words.len() {
        let word = str_words[i].to_owned();
        match word.as_str() {
            "def" => {
                words.push(Word::Key(Keyword::Def(str_words[i + 1].to_owned())));
                i += 1;
            }
            "func" => {
                if let Some(dat) = readf("func\0{}\0{", str_words[i..=i + 2].join("\0").as_str()) {
                    let block = read_block(
                        &str_words[i + 2..],
                        true,
                        Arc::new(Frame::new(origin.clone())),
                    )?;
                    i += 2 + block.2;
                    words.push(Word::Key(Keyword::Func(
                        dat[0].to_owned(),
                        block.0.expect("LEXERR: Expected `{ <type> <...> |`."),
                        block.1,
                    )));
                }
            }
            "{" => {
                let block =
                    read_block(&str_words[i..], true, Arc::new(Frame::new(origin.clone())))?;
                i += block.2;
                words.push(Word::Const(Value::Func(AFunc::new(Func {
                    ret_count: block.0.expect("LEXERR: Expected `{ <type> <...> |`."),
                    to_call: FuncImpl::SPL(block.1),
                    origin: origin.to_owned(),
                    cname: None,
                }))))
            }
            "construct" => {
                let name = (&str_words[i + 1]).to_owned();
                assert_eq!(
                    str_words[i + 2],
                    "{",
                    "LEXERR: Expected `construct <name> {{`, got `construct <name>`"
                );
                let mut fields = Vec::new();
                i += 3;
                while str_words[i] != ";" && str_words[i] != "}" {
                    fields.push((&str_words[i]).to_owned());
                    i += 1;
                }
                let mut methods = Vec::new();
                let mut has_construct = false;
                if str_words[i] == ";" {
                    i += 1;
                    while str_words[i] != "}" {
                        let name = (&str_words[i]).to_owned();
                        if name == "construct" {
                            has_construct = true;
                        }
                        let block = read_block(&str_words[i + 1..], true, origin.clone())?;
                        i += 1 + block.2;
                        methods.push((
                            name,
                            (
                                block.0.expect("LEXERR: Expected `{ <type> <...> |`."),
                                block.1,
                            ),
                        ));
                        i += 1;
                    }
                }
                if !has_construct {
                    methods.push(("construct".to_string(), (1, Words { words: vec![] })));
                }
                words.push(Word::Key(Keyword::Construct(name, fields, methods)));
            }
            "include" => {
                if let Some(x) = readf(
                    "include\0{}\0in\0{}",
                    str_words[i..i + 4].join("\0").as_str(),
                ) {
                    words.push(Word::Key(Keyword::Include(
                        x[0].to_owned(),
                        x[1].to_owned(),
                    )))
                } else {
                    panic!("LEXERR: Expected `include <typeA> in <typeB>`.");
                }
                i += 3;
            }
            "while" => {
                let cond = read_block(&str_words[i + 2..], false, origin.clone())?;
                i += 2 + cond.2;
                let blk = read_block(&str_words[i + 2..], false, origin.clone())?;
                i += 2 + blk.2;
                words.push(Word::Key(Keyword::While(cond.1, blk.1)));
            }
            "if" => {
                let blk = read_block(&str_words[i + 2..], false, origin.clone())?;
                i += 2 + blk.2;
                words.push(Word::Key(Keyword::If(blk.1)));
            }
            "with" => {
                let mut vars = Vec::new();
                i += 1;
                while &str_words[i] != ";" {
                    vars.push((&str_words[i]).to_owned());
                    i += 1;
                }
                words.push(Word::Key(Keyword::With(vars)));
            }
            "}" => {
                break;
            }
            x if x.starts_with("\"") => {
                words.push(Word::Const(Value::Str(x[1..].to_owned())));
            }
            x if x.chars().all(|c| c.is_numeric() || c == '_') && !x.starts_with("_") => {
                words.push(Word::Const(Value::Mega(x.parse().unwrap())));
            }
            x if x.chars().all(|c| c.is_numeric() || c == '.' || c == '_')
                && !x.starts_with("_") =>
            {
                words.push(Word::Const(Value::Double(x.parse().unwrap())));
            }
            x => {
                let mut word = x.split(":").next().unwrap();
                let mut ra = 0;
                while word.starts_with("&") {
                    ra += 1;
                    word = &word[1..];
                }
                if word.ends_with(";") {
                    words.push(Word::Call(word[..word.len() - 1].to_owned(), true, ra));
                } else {
                    words.push(Word::Call(word.to_owned(), false, ra));
                }
                for mut word in x.split(":").skip(1) {
                    let mut ra = 0;
                    while word.starts_with("&") {
                        ra += 1;
                        word = &word[1..];
                    }
                    if word.ends_with(";") {
                        words.push(Word::ObjCall(word[..word.len() - 1].to_owned(), true, ra));
                    } else {
                        words.push(Word::ObjCall(word.to_owned(), false, ra));
                    }
                }
            }
        }
        i += 1;
    }
    Ok((rem, Words { words }, i))
}

fn parse_line(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut in_string = false;
    let mut escaping = false;
    let mut was_in_string = false;
    let mut s = String::new();
    for c in line.chars() {
        if in_string {
            if escaping {
                if c == '\\' {
                    s += "\\";
                }
                if c == 'n' {
                    s += "\n";
                }
                if c == 'r' {
                    s += "\r";
                }
                if c == '"' {
                    s += "\"";
                }
                escaping = false;
                continue;
            } else if c == '"' {
                in_string = false;
                escaping = false;
                was_in_string = true;
                continue;
            }
            if c == '\\' {
                escaping = true;
                continue;
            }
        } else {
            if c == '"' {
                s += "\"";
                in_string = true;
                continue;
            }
            if c == ';' && was_in_string {
                s = String::new();
                continue;
            }
            if c == '(' || c == ')' {
                continue;
            }
            if c == ' ' {
                if s == "" {
                    continue;
                }
                words.push(s);
                s = String::new();
                was_in_string = false;
                continue;
            }
        }
        was_in_string = false;
        s += String::from(c).as_str();
    }
    if s != "" {
        words.push(s);
    }
    words
}
