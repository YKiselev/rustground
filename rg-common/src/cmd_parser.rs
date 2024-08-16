///
/// Command line parser
///

pub struct CmdParser<'a> {
    data: &'a str,
}

enum State {
    Normal,
    Quoted,
    Backslash,
    Whitespace,
}

impl<'a> CmdParser<'a> {
    pub fn new(cmd_line: &'a str) -> Self {
        CmdParser { data: cmd_line }
    }

    pub fn next(&mut self) -> Option<Vec<&str>> {
        let mut result = Vec::new();
        let mut pos: usize = 0;
        let mut len: usize = 0;
        let mut chars = self.data.chars();
        let mut state = State::Whitespace;
        let mut squotes: usize = 0;
        let mut dquotes: usize = 0;
        //let mut quotes = Vec::new();
        while let Some(ch) = chars.next() {
            match state {
                State::Normal => match ch {
                    '\\' => {
                        state = State::Backslash;
                        //result.push(&self.data[pos..pos + len]);
                        //pos += len;
                        //len = 0;
                    }
                    '"' | '\'' => {
                        if ch == '"' {
                            dquotes += 1;
                        } else {
                            squotes += 1;
                        }
                        //quotes.push(ch);
                        state = State::Quoted;
                        result.push(&self.data[pos..pos + len]);
                        pos += len;
                        len = 0;
                    }
                    _ if ch.is_whitespace() => {
                        state = State::Whitespace;
                        result.push(&self.data[pos..pos + len]);
                        pos += len + 1;
                        len = 0;
                    }
                    _ => {
                        len += 1;
                    }
                },
                State::Whitespace => {
                    if ch.is_whitespace() {
                        pos += 1;
                    } else {
                        match ch {
                            '\\' => {
                                state = State::Backslash;
                                pos += 1;
                            }
                            '"' | '\'' => {
                                if ch == '"' {
                                    dquotes += 1;
                                } else {
                                    squotes += 1;
                                }
                                state = State::Quoted;
                                pos += 1;
                            }
                            _ => {
                                state = State::Normal;
                                len += 1;
                            }
                        }
                    }
                }
                State::Quoted => match ch {
                    '"' | '\\' => {

                    }
                    _ => {
                        len += 1;
                    }
                },
                State::Backslash => {}
            }
        }
        if len > 0 {
            result.push(&self.data[pos..pos + len]);
            pos += len;
        }
        if pos > 0 {
            self.data = &self.data[pos..];
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

    #[test]
    fn parser() {
        let mut parser = CmdParser::new("a b c d");
        assert_eq!(parser.next(), Some(vec!["a", "b", "c", "d"]));
        // while let Some(args) = parser.next() {
        //     println!("Got command: {args:?}");
        // }
    }
}
