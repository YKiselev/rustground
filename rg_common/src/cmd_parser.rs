use std::{ops::Range, str::CharIndices};

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

fn unquote(value: &str) -> &str {
    let mut chars = value.chars();
    if let (Some(first), Some(last)) = (chars.next(), chars.next_back()) {
        if is_quote(first) && first == last {
            &value[1..value.len() - 1]
        } else {
            value
        }
    } else {
        value
    }
}

pub fn parse_command_line(str: &str) -> (Option<&str>, Option<Vec<&str>>) {
    let mut chars = str.char_indices();
    let mut result = Vec::new();
    let mut state = State::Whitespace;
    let mut segment_start = 0usize;
    let mut length = 0usize;
    while let Some((index, ch)) = chars.next() {
        match state {
            State::Normal => match ch {
                '\n' | ';' => {
                    break;
                }
                '\\' => {
                    state = State::Backslash('a');
                    length += ch.len_utf8();
                }
                '"' | '\'' => {
                    state = State::Quoted(ch);
                    length += ch.len_utf8();
                }
                _ if ch.is_whitespace() => {
                    state = State::Whitespace;
                    if length > 0 {
                        result.push(unquote(&str[segment_start..segment_start + length]));
                        segment_start = index;
                    }
                }
                _ => {
                    length += ch.len_utf8();
                }
            },
            State::Whitespace => {
                if !ch.is_whitespace() {
                    segment_start = index;
                    length = 0;
                    match ch {
                        ';' => {
                            break;
                        }
                        '\\' => {
                            state = State::Backslash(' ');
                            length += ch.len_utf8();
                        }
                        '"' | '\'' => {
                            state = State::Quoted(ch);
                            length += ch.len_utf8();
                        }
                        _ => {
                            state = State::Normal;
                            length += ch.len_utf8();
                        }
                    }
                }
            }
            State::Quoted(q) => {
                match ch {
                    '"' | '\'' if q == ch => {
                        state = State::Normal;
                    }
                    '\\' => {
                        state = State::Backslash(q);
                    }
                    _ => {}
                }
                length += ch.len_utf8();
            }
            State::Backslash(v) => {
                length += ch.len_utf8();
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
    if length > 0 {
        result.push(unquote(&str[segment_start..segment_start + length]));
    }
    let str_opt = if let Some((index, _h)) = chars.next() {
        Some(&str[index..])
    } else {
        None
    };
    let vec_opt = if result.is_empty() {
        None
    } else {
        Some(result)
    };
    (str_opt, vec_opt)
}

#[cfg(test)]
mod test {
    use super::*;

    fn parse(cmd: &str) -> Option<Vec<&str>> {
        let (_, parts) = parse_command_line(cmd);
        parts
    }

    fn assert(cmd: &str, expected: &[&str]) {
        let e: Vec<&str> = expected.to_vec();
        assert_eq!(parse(cmd), Some(e));
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
        let mut r = Vec::new();
        let mut str = a;
        while let (rest, Some(parts)) = parse_command_line(str) {
            r.push(parts);
            match rest {
                Some(s) => str = s,
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
