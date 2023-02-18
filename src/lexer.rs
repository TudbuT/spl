use crate::runtime::*;
use readformat::*;

pub fn lex(input: String, filename: String) -> Words {
    let mut str_words = Vec::new();
    for line in input.split('\n') {
        str_words.append(&mut parse_line(line));
    }
    read_block(&str_words[..], false, &FrameInfo { file: filename }).1
}

fn read_block(str_words: &[String], isfn: bool, origin: &FrameInfo) -> (Option<u32>, Words, usize) {
    let mut rem = None;
    let mut words = Vec::new();
    let mut i = 0;
    if str_words[0] == "{" {
        if isfn {
            let mut r = 0_u32;
            while str_words[r as usize + 1] != "|" {
                r += 1;
            }
            i += r as usize + 1;
            rem = Some(r);
        }
        i += 1;
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
                    let block = read_block(&str_words[i + 2..], true, &origin);
                    i += 2 + block.2;
                    words.push(Word::Key(Keyword::Func(
                        dat[0].to_owned(),
                        block.0.expect("LEXERR: Expected `{ <type> <...> |`."),
                        block.1,
                    )));
                }
            }
            "{" => {
                let block = read_block(&str_words[i..], true, &origin);
                i += block.2;
                words.push(Word::Const(Constant::Func(AFunc::new(Func {
                    ret_count: block.0.expect("LEXERR: Expected `{ <type> <...> |`."),
                    to_call: FuncImpl::SPL(block.1),
                    origin: origin.to_owned(),
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
                i += 2;
                while str_words[i] != ";" && str_words[i] != "}" {
                    fields.push((&str_words[i]).to_owned());
                    i += 1;
                }
                let mut methods = Vec::new();
                if str_words[i] == ";" {
                    i += 1;
                    while str_words[i] != "}" {
                        let name = (&str_words[i]).to_owned();
                        let block = read_block(&str_words[i + 1..], true, origin);
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
                words.push(Word::Key(Keyword::Construct(name, fields, methods)));
            }
            "include" => {
                if let Some(x) = readf(
                    "include\0{}\0in\0{}",
                    str_words[i..=i + 4].join("\0").as_str(),
                ) {
                    words.push(Word::Key(Keyword::Include(
                        x[0].to_owned(),
                        x[1].to_owned(),
                    )))
                } else {
                    panic!("LEXERR: Expected `include <typeA> in <typeB>`.");
                }
            }
            "while" => {
                let cond = read_block(&str_words[i + 1..], false, origin);
                i += 1 + cond.2;
                let blk = read_block(&str_words[i + 1..], false, origin);
                i += 1 + cond.2;
                words.push(Word::Key(Keyword::While(cond.1, blk.1)));
            }
            "if" => {
                let cond = read_block(&str_words[i + 1..], false, origin);
                i += 1 + cond.2;
                let blk = read_block(&str_words[i + 1..], false, origin);
                i += 1 + cond.2;
                words.push(Word::Key(Keyword::If(cond.1, blk.1)));
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
                words.push(Word::Const(Constant::Str(x[1..].to_owned())));
            }
            x if x.chars().all(|c| c.is_numeric() || c == '_') && !x.starts_with("_") => {
                words.push(Word::Const(Constant::Mega(x.parse().unwrap())));
            }
            x if x.chars().all(|c| c.is_numeric() || c == '.' || c == '_') && !x.starts_with("_") => {
                words.push(Word::Const(Constant::Double(x.parse().unwrap())));
            }
            mut x => {
                let mut ra = 0;
                while x.starts_with("&") {
                    ra += 1;
                    x = &x[1..];
                }
                if x.ends_with(";") {
                    words.push(Word::Call(x[..x.len() - 1].to_owned(), true, ra));
                } else {
                    words.push(Word::Call(x.to_owned(), false, ra));
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
    (rem, Words { words }, i)
}

fn parse_line(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut in_string = false;
    let mut escaping = false;
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
                escaping = false;
                continue;
            } else if c == '"' {
                in_string = false;
                escaping = false;
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
            if c == ' ' {
                if s == "" {
                    continue;
                }
                words.push(s);
                s = String::new();
                continue;
            }
        }
        s += String::from(c).as_str();
    }
    if s != "" {
        words.push(s);
    }
    words
}
