use std::str::Chars;

///
/// Command line parser
///

pub struct CmdParser<'a> {
    chars: Chars<'a>,
}

enum State {
    Normal,
    Quoted(char),
    Backslash(char),
    Whitespace,
}

impl<'a> CmdParser<'a> {
    pub fn new(cmd_line: &'a str) -> Self {
        CmdParser {
            chars: cmd_line.chars(),
        }
    }

    fn is_quote(ch: char) -> bool {
        ch == '\"' || ch == '\''
    }

    fn strip_last_quote(mut value: String) -> Option<String> {
        let result = if value.len() >= 2
            && Self::is_quote(value.chars().next().unwrap())
            && Self::is_quote(value.chars().next_back().unwrap())
        {
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

    pub fn next(&mut self) -> Option<Vec<String>> {
        let mut result = Vec::new();
        let mut buf = String::new();
        let mut state = State::Whitespace;
        while let Some(ch) = self.chars.next() {
            match state {
                State::Normal => match ch {
                    ';' => {
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
                            if let Some(v) = Self::strip_last_quote(buf) {
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
        if let Some(v) = Self::strip_last_quote(buf) {
            result.push(v);
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

#[cfg(test)]
mod test {
    use super::CmdParser;

    fn assert(cmd: &str, expected: Vec<&str>) {
        let mut parser = CmdParser::new(cmd);
        let e = expected
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        assert_eq!(parser.next(), Some(e));
    }

    #[test]
    fn multibyte_chars() {
        assert("Ф Ы Ё", vec!["Ф", "Ы", "Ё"]);
    }

    #[test]
    fn whitespaces() {
        assert("a b c d", vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn quotes() {
        assert("a \"b c\" d", vec!["a", "b c", "d"]);
        assert("a 'b c' d", vec!["a", "b c", "d"]);
        assert("a \" 'b c' \" d", vec!["a", " 'b c' ", "d"]);
        assert("a ' \"b c\" ' d", vec!["a", " \"b c\" ", "d"]);
        assert("a' b c 'd", vec!["a' b c 'd"]);
    }

    #[test]
    fn backslash() {
        assert("a\" b c \"d", vec!["a\" b c \"d"]);
        assert("a\" b c\\\" d\"", vec!["a\" b c\\\" d\""]);
    }

    #[test]
    fn semicolon() {
        assert("a b; c d", vec!["a", "b"]);
        assert("a b\\; c d", vec!["a", "b\\;", "c", "d"]);
        assert("a \"b; c d\"", vec!["a", "b; c d"]);
    }
}
