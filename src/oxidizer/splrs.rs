use readformat::readf1;

use super::RustFunction;

/// Parses a #-expression and returns the string to be inserted in its place.
fn parse_hash_expr(s: String, name: &str) -> String {
    if &s == "pop" {
        return "stack.pop().lock_ro()".to_owned();
    }
    if &s == "pop_mut" {
        return "stack.pop().lock()".to_owned();
    }
    if &s == "pop:Array" {
        return format!("{{ require_array_on_stack!(tmp, stack, {name:?}); tmp }}");
    }
    if &s == "pop_mut:Array" {
        return format!("{{ require_mut_array_on_stack!(tmp, stack, {name:?}); tmp }}");
    }
    if let Some(s) = readf1("pop:{}", &s) {
        return format!("{{ require_on_stack!(tmp, {s}, stack, {name:?}); tmp }}");
    }
    if let Some(s) = readf1("push({})", &s) {
        return format!("stack.push(({s}).spl());");
    }
    panic!("invalid #-expr - this error will be handled in the future")
}

pub fn to_rust(name: String, mut splrs: String) -> RustFunction {
    RustFunction {
        content: {
            loop {
                let mut did_anything = false;

                let mut rs = String::new();
                let mut in_str = false;
                let mut escaping = false;
                let mut hash_expr = None;
                let mut brace = 0;
                for c in splrs.chars() {
                    if in_str {
                        if escaping {
                            escaping = false;
                        } else if c == '"' {
                            in_str = false;
                        }
                        if c == '\\' {
                            escaping = true;
                        }
                    } else if c == '"' {
                        in_str = true;
                    }
                    if !in_str && c == '#' && hash_expr.is_none() {
                        hash_expr = Some(String::new());
                        did_anything = true;
                        continue;
                    }
                    if let Some(ref mut expr) = hash_expr {
                        if c == '#' && brace == 0 {
                            rs += &parse_hash_expr(expr.to_owned(), &name);
                            hash_expr = None;
                            continue;
                        }
                        expr.push(c);
                        if c == '(' {
                            brace += 1;
                        }
                        if c == ')' {
                            brace -= 1;
                        }
                        continue;
                    }
                    rs += String::from(c).as_str();
                }
                if !did_anything {
                    break rs;
                }
                splrs = rs;
            }
        },
        fn_name: name,
    }
}
