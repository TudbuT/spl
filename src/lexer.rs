use std::sync::Arc;

use crate::runtime::*;
use readformat::*;

#[derive(Debug, PartialEq, Eq)]
pub enum LexerError {
    FunctionBlockExpected,
    WrongFunctionDeclaration,
    InvalidInclude,
    InvalidConstructBlock,
    InvalidNumber(String),
    ArgsWithoutCall,
}

pub fn lex(input: String) -> Result<Words, LexerError> {
    let mut str_words = Vec::new();
    for line in input.split('\n') {
        str_words.append(&mut parse_line(line));
    }
    Ok(read_block(&str_words[..], false)?.1)
}

fn read_block(str_words: &[String], isfn: bool) -> Result<(Option<u32>, Words, usize), LexerError> {
    if str_words.is_empty() {
        return Ok((None, Words::new(Vec::new()), 0));
    }
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
                if let Some(dat) = readf1("func\0{}\0{", str_words[i..=i + 2].join("\0").as_str()) {
                    let block = read_block(&str_words[i + 2..], true)?;
                    i += 2 + block.2;
                    words.push(Word::Key(Keyword::Func(
                        dat.to_owned(),
                        block.0.ok_or(LexerError::FunctionBlockExpected)?,
                        block.1,
                    )));
                }
            }
            "{" => {
                let block = read_block(&str_words[i..], true)?;
                i += block.2;
                words.push(Word::Const(Value::Func(AFunc::new(Func {
                    ret_count: block.0.ok_or(LexerError::FunctionBlockExpected)?,
                    to_call: FuncImpl::SPL(block.1),
                    run_as_base: false,
                    origin: Arc::new(Frame::dummy()),
                    fname: None,
                    name: "dyn".to_owned(),
                }))))
            }
            "<{" => {
                let block = read_block(&str_words[i + 1..], false)?;
                i += block.2 + 1;
                let mut block = block.1.words;
                match words.remove(words.len() - 1) {
                    Word::Call(a, b, c) => {
                        words.append(&mut block);
                        words.push(Word::Call(a, b, c));
                    }
                    Word::ObjCall(a, b, c) => {
                        words.push(Word::Key(Keyword::ObjPush));
                        words.append(&mut block);
                        words.push(Word::Key(Keyword::ObjPop));
                        words.push(Word::ObjCall(a, b, c));
                    }
                    _ => return Err(LexerError::ArgsWithoutCall),
                }
            }
            "construct" => {
                let name = str_words[i + 1].to_owned();
                let is_namespace = if str_words[i + 2] == "namespace" {
                    i += 1;
                    true
                } else {
                    false
                };
                if str_words[i + 2] != "{" {
                    return Err(LexerError::InvalidConstructBlock);
                }
                let mut fields = Vec::new();
                i += 3;
                while str_words[i] != ";" && str_words[i] != "}" {
                    fields.push(str_words[i].to_owned());
                    i += 1;
                }
                let mut methods = Vec::new();
                let mut has_construct = false;
                if str_words[i] == ";" {
                    i += 1;
                    while str_words[i] != "}" {
                        let name = str_words[i].to_owned();
                        if name == "construct" {
                            has_construct = true;
                        }
                        let block = read_block(&str_words[i + 1..], true)?;
                        i += 1 + block.2;
                        methods.push((
                            name,
                            (block.0.ok_or(LexerError::FunctionBlockExpected)?, block.1),
                        ));
                        i += 1;
                    }
                }
                if !has_construct && !is_namespace {
                    methods.push(("construct".to_string(), (1, Words { words: vec![] })));
                }
                words.push(Word::Key(Keyword::Construct(
                    name,
                    fields,
                    methods,
                    is_namespace,
                )));
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
                    return Err(LexerError::InvalidInclude);
                }
                i += 3;
            }
            "use" => {
                let item = str_words[i + 1].to_owned();
                i += 1;
                words.push(Word::Key(Keyword::Use(item)));
            }
            "while" => {
                let cond = read_block(&str_words[i + 2..], false)?;
                i += 2 + cond.2;
                let blk = read_block(&str_words[i + 2..], false)?;
                i += 2 + blk.2;
                words.push(Word::Key(Keyword::While(cond.1, blk.1)));
            }
            "if" => {
                let blk = read_block(&str_words[i + 2..], false)?;
                i += 2 + blk.2;
                words.push(Word::Key(Keyword::If(blk.1)));
            }
            "catch" => {
                let mut types = Vec::new();
                i += 1;
                while &str_words[i] != "{" {
                    types.push(str_words[i].to_owned());
                    i += 1;
                }
                let blk = read_block(&str_words[i + 1..], false)?;
                i += 1 + blk.2;
                let ctch = read_block(&str_words[i + 1..], false)?;
                i += 1 + ctch.2;
                words.push(Word::Key(Keyword::Catch(types, blk.1, ctch.1)))
            }
            "with" => {
                let mut vars = Vec::new();
                i += 1;
                while &str_words[i] != ";" {
                    vars.push(str_words[i].to_owned());
                    i += 1;
                }
                words.push(Word::Key(Keyword::With(vars)));
            }
            "}" => {
                break;
            }
            x if x.starts_with('\"') => {
                words.push(Word::Const(Value::Str(x[1..].to_owned())));
            }
            x if x.chars().all(|c| c.is_numeric() || c == '_' || c == '-')
                && !x.starts_with('_')
                && x.contains(char::is_numeric) =>
            {
                words.push(Word::Const(Value::Mega(
                    x.parse()
                        .map_err(|_| LexerError::InvalidNumber(x.to_owned()))?,
                )));
            }
            x if x
                .chars()
                .all(|c| c.is_numeric() || c == '.' || c == '_' || c == '-')
                && !x.starts_with('_')
                && x.contains(char::is_numeric) =>
            {
                words.push(Word::Const(Value::Double(
                    x.parse()
                        .map_err(|_| LexerError::InvalidNumber(x.to_owned()))?,
                )));
            }
            x => {
                let mut word = x.split(':').next().unwrap(); // SAFETY: One item always exists after a split.
                if !word.is_empty() {
                    let mut ra = 0;
                    while word.starts_with('&') {
                        ra += 1;
                        word = &word[1..];
                    }
                    if let Some(word) = word.strip_suffix(';') {
                        words.push(Word::Call(word.to_owned(), true, ra));
                    } else {
                        words.push(Word::Call(word.to_owned(), false, ra));
                    }
                }
                for mut word in x.split(':').skip(1) {
                    let mut ra = 0;
                    while word.starts_with('&') {
                        ra += 1;
                        word = &word[1..];
                    }
                    if let Some(word) = word.strip_suffix(';') {
                        words.push(Word::ObjCall(word.to_owned(), true, ra));
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
                if s.is_empty() {
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
    if !s.is_empty() {
        words.push(s);
    }
    words
}
