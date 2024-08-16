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

    pub fn next(&mut self) -> Option<Vec<String>> {
        let mut result = Vec::new();
        let mut buf = String::new();
        let mut state = State::Whitespace;
        while let Some(ch) = self.chars.next() {
            match state {
                State::Normal => match ch {
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
                            let last = buf.pop().unwrap();
                            if !Self::is_quote(last) {
                                buf.push(last);
                            }
                            if !buf.is_empty() {
                                result.push(buf);
                                buf = String::new();
                            }
                        }
                    }
                    _ => {
                        buf.push(ch);
                    }
                },
                State::Whitespace => {
                    if ch.is_whitespace() {
                    } else {
                        match ch {
                            '\\' => {
                                state = State::Backslash(' ');
                            }
                            '"' | '\'' => {
                                state = State::Quoted(ch);
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
                State::Backslash(v) => match v {
                    '"' | '\'' => {
                        state = State::Quoted(v);
                        buf.push(ch);
                    }
                    _ if ch.is_whitespace() => {
                        state = State::Whitespace;
                        result.push(buf);
                        buf = String::new();
                    }
                    _ => {
                        state = State::Normal;
                    }
                },
            }
        }
        if !buf.is_empty() {
            result.push(buf);
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
    fn parser() {
        assert("Ф Ы Ё", vec!["Ф", "Ы", "Ё"]);
        assert("a b c d", vec!["a", "b", "c", "d"]);
        assert("a \"b c\" d", vec!["a", "b c", "d"]);
        assert("a 'b c' d", vec!["a", "b c", "d"]);
        assert("a \" 'b c' \" d", vec!["a", " 'b c' ", "d"]);
        assert("a ' \"b c\" ' d", vec!["a", " \"b c\" ", "d"]);
        assert("a' b c 'd", vec!["a' b c 'd"]);
        assert("a\" b c \"d", vec!["a\" b c \"d"]);
        assert("a\" b c\\\" d\"", vec!["a\" b c\\\" d\""]);
    }
}
