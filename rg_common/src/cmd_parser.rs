use std::str::Chars;

///
/// Command line parser
///

enum State {
    Normal,
    Quoted(char),
    Backslash(char),
    Whitespace,
}

fn is_quote(ch: char) -> bool {
    ch == '\"' || ch == '\''
}

fn unquote(mut value: String) -> Option<String> {
    let first = value.chars().next()?;
    let last = value.chars().next_back()?;
    let result = if is_quote(first) && is_quote(last) {
        value.remove(0);
        value.pop();
        value
    } else {
        value
    };
    if !result.is_empty() {
        Some(result)
    } else {
        None
    }
}

pub fn parse_command_line(chars: &mut Chars<'_>) -> Option<Vec<String>> {
    let mut result = Vec::new();
    let mut buf = String::new();
    let mut state = State::Whitespace;
    while let Some(ch) = chars.next() {
        match state {
            State::Normal => match ch {
                '\n' | ';' => {
                    break;
                }
                '\\' => {
                    state = State::Backslash('a');
                    buf.push(ch);
                }
                '"' | '\'' => {
                    state = State::Quoted(ch);
                    buf.push(ch);
                }
                _ if ch.is_whitespace() => {
                    state = State::Whitespace;
                    if !buf.is_empty() {
                        if let Some(v) = unquote(buf) {
                            result.push(v);
                        }
                        buf = String::new();
                    }
                }
                _ => {
                    buf.push(ch);
                }
            },
            State::Whitespace => {
                if !ch.is_whitespace() {
                    match ch {
                        ';' => {
                            break;
                        }
                        '\\' => {
                            state = State::Backslash(' ');
                            buf.push(ch);
                        }
                        '"' | '\'' => {
                            state = State::Quoted(ch);
                            buf.push(ch);
                        }
                        _ => {
                            state = State::Normal;
                            buf.push(ch);
                        }
                    }
                }
            }
            State::Quoted(q) => match ch {
                '"' | '\'' if q == ch => {
                    state = State::Normal;
                    buf.push(ch);
                }
                '\\' => {
                    state = State::Backslash(q);
                    buf.push(ch);
                }
                _ => {
                    buf.push(ch);
                }
            },
            State::Backslash(v) => {
                buf.push(ch);
                match v {
                    '"' | '\'' => {
                        state = State::Quoted(v);
                    }
                    _ => {
                        state = State::Normal;
                    }
                }
            }
        }
    }
    if let Some(v) = unquote(buf) {
        result.push(v);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod test {
    use crate::cmd_parser::parse_command_line;

    fn assert(cmd: &str, expected: &[&str]) {
        let e = expected
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let mut chars = cmd.chars();
        assert_eq!(parse_command_line(&mut chars), Some(e));
    }

    #[test]
    fn multibyte_chars() {
        assert("Ф Ы Ё", &["Ф", "Ы", "Ё"]);
    }

    #[test]
    fn whitespaces() {
        assert("a b c d", &["a", "b", "c", "d"]);
    }

    #[test]
    fn quotes() {
        assert("a \"b c\" d", &["a", "b c", "d"]);
        assert("a 'b c' d", &["a", "b c", "d"]);
        assert("a \" 'b c' \" d", &["a", " 'b c' ", "d"]);
        assert("a ' \"b c\" ' d", &["a", " \"b c\" ", "d"]);
        assert("a' b c 'd", &["a' b c 'd"]);
    }

    #[test]
    fn backslash() {
        assert("a\" b c \"d", &["a\" b c \"d"]);
        assert("a\" b c\\\" d\"", &["a\" b c\\\" d\""]);
    }

    #[test]
    fn semicolon() {
        assert("a b; c d", &["a", "b"]);
        assert("a b\\; c d", &["a", "b\\;", "c", "d"]);
        assert("a \"b; c d\"", &["a", "b; c d"]);
    }

    #[test]
    fn several_commands() {
        let a = "a 1 2 3 ; b 4 5; c 6\n d 7";
        let mut chars = a.chars();
        let mut r = Vec::new();
        loop {
            match parse_command_line(&mut chars) {
                Some(args) => r.push(args),
                None => break,
            }
        }
        assert_eq!(
            vec![
                vec!["a", "1", "2", "3"],
                vec!["b", "4", "5"],
                vec!["c", "6"],
                vec!["d", "7"]
            ],
            r
        );
    }
}
