pub enum Node {
    Internal {
        type_name: String,
        ident: Option<String>,
        children: Vec<Node>,
    },

    Leaf {
        type_name: String,
        contents: String,
    },
}

impl Node {
    fn from_string(text: &str) -> Node {
        let mut nodes = Vec::new();

        let mut ti = token_iter(text);
        while let Some(node) = parse_node(&mut ti) {
            nodes.push(node);
        }

        Node::Internal {
            type_name: "ROOT".to_string(),
            ident: None,
            children: nodes,
        }
    }
}


fn parse_node(ti: &mut TokenIter) -> Option<Node> {
    let type_name = if let Some(Token::TypeName(token)) = ti.next() {
        token
    } else {
        panic!("Parse error")
    };

    let ident = match ti.next() {
        Some(Token::Ident(token)) => Some(token),

        _ => None,
    };

    // TODO

    unimplemented!()
}


fn token_iter<'a>(text: &'a str) -> TokenIter<'a> {
    TokenIter {
        text: text,
        bytes_consumed: 0,
        after_open_leaf: false,
    }
}




// ================================================================

/// Tokens contain their starting byte offset in the original source
/// text.  Some variants also contain a string slice of the relevant
/// text.
#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    TypeName((usize, &'a str)),
    Ident((usize, &'a str)),
    OpenInner(usize),
    CloseInner(usize),
    OpenLeaf(usize),
    CloseLeaf(usize),
    LeafContents((usize, &'a str)),
    Unknown(usize),
}

struct TokenIter<'a> {
    text: &'a str,
    bytes_consumed: usize,
    after_open_leaf: bool,
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Token<'a>> {
        let mut token = None;
        let mut iter = self.text.char_indices().peekable();

        if !self.after_open_leaf {
            // Skip newlines, whitespace, and comments
            loop {
                let mut skipped = false;

                while let Some(&(_, c)) = iter.peek() {
                    if is_ws_char(c) || is_nl_char(c) {
                        iter.next();
                        skipped = true;
                    } else {
                        break;
                    }
                }

                if let Some(&(_, c)) = iter.peek() {
                    if is_comment_char(c) {
                        iter.next();
                        skipped = true;
                        while let Some(&(_, c)) = iter.peek() {
                            if !is_nl_char(c) {
                                iter.next();
                            } else {
                                break;
                            }
                        }
                        iter.next();
                    }
                }

                if !skipped {
                    break;
                }
            }

            // Parse the meat of the token
            if let Some(&(i, c)) = iter.peek() {
                // TypeName
                if is_ident_char(c) {
                    iter.next();
                    let i1 = i;
                    let i2 = {
                        let mut i2 = i1;
                        while let Some(&(i, c)) = iter.peek() {
                            if is_ident_char(c) {
                                iter.next();
                            } else {
                                i2 = i;
                                break;
                            }
                        }
                        i2
                    };
                    token = Some(Token::TypeName((self.bytes_consumed + i1, &self.text[i1..i2])));
                }
                // Ident
                else if c == '$' {
                    iter.next();
                    let i1 = i;
                    let i2 = {
                        let mut i2 = i1;
                        let mut escaped = false;
                        while let Some(&(i, c)) = iter.peek() {
                            if escaped {
                                escaped = false;
                            } else if c == '\\' {
                                escaped = true;
                            } else if !is_ident_char(c) {
                                i2 = i;
                                break;
                            }
                            iter.next();
                        }
                        i2
                    };
                    token = Some(Token::Ident((self.bytes_consumed + i1, &self.text[i1..i2])));
                }
                // Structural characters
                else if is_reserved_char(c) {
                    iter.next();
                    match c {
                        '{' => {
                            token = Some(Token::OpenInner(self.bytes_consumed + i));
                        }

                        '}' => {
                            token = Some(Token::CloseInner(self.bytes_consumed + i));
                        }

                        '[' => {
                            self.after_open_leaf = true;
                            token = Some(Token::OpenLeaf(self.bytes_consumed + i));
                        }

                        ']' => {
                            token = Some(Token::CloseLeaf(self.bytes_consumed + i));
                        }

                        _ => {
                            token = Some(Token::Unknown(self.bytes_consumed + i));
                        }
                    }
                }
            }
        }
        // Leaf contents
        else if let Some(&(i, _)) = iter.peek() {
            self.after_open_leaf = false;
            let i1 = i;
            let i2 = {
                let mut i2 = i1;
                let mut escaped = false;
                while let Some(&(i, c)) = iter.peek() {
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == ']' {
                        i2 = i;
                        break;
                    }
                    iter.next();
                }
                i2
            };
            token = Some(Token::LeafContents((self.bytes_consumed + i1, &self.text[i1..i2])));
        }

        // Finish up
        match iter.peek() {
            Some(&(i, _)) => {
                self.bytes_consumed += i;
                self.text = &self.text[i..];
            }

            None => {
                self.text = "";
            }
        }
        return token;
    }
}




// ================================================================

/// Returns whether the given unicode character is whitespace or not.
fn is_ws_char(c: char) -> bool {
    match c {
        ' ' | '\t' => true,
        _ => false,
    }
}


/// Returns whether the given utf character is a newline or not.
fn is_nl_char(c: char) -> bool {
    match c {
        '\n' | '\r' => true,
        _ => false,
    }
}


/// Returns whether the given utf character is a comment starter or not.
fn is_comment_char(c: char) -> bool {
    c == '#'
}


/// Returns whether the given utf character is a reserved character or not.
fn is_reserved_char(c: char) -> bool {
    match c {
        '{' | '}' | '[' | ']' | '\\' | '$' => true,
        _ => false,
    }
}


/// Returns whether the given utf character is a legal identifier character or not.
fn is_ident_char(c: char) -> bool {
    // Anything that isn't whitespace, reserved, or an operator character
    if !is_ws_char(c) && !is_nl_char(c) && !is_reserved_char(c) && !is_comment_char(c) {
        true
    } else {
        false
    }
}




// ================================================================

#[cfg(test)]
mod tests {
    use super::{token_iter, Token};

    #[test]
    fn token_iter_1() {
        let s = r#"
            # This is a comment and should be skipped
            MyThing $ident { # This is another comment
                MyProp [Some content]
            }
        "#;

        let mut ti = token_iter(s);
        assert_eq!(ti.next(), Some(Token::TypeName((67, "MyThing"))));
        assert_eq!(ti.next(), Some(Token::Ident((75, "$ident"))));
        assert_eq!(ti.next(), Some(Token::OpenInner(82)));
        assert_eq!(ti.next(), Some(Token::TypeName((126, "MyProp"))));
        assert_eq!(ti.next(), Some(Token::OpenLeaf(133)));
        assert_eq!(ti.next(), Some(Token::LeafContents((134, "Some content"))));
        assert_eq!(ti.next(), Some(Token::CloseLeaf(146)));
        assert_eq!(ti.next(), Some(Token::CloseInner(160)));
        assert_eq!(ti.next(), None);
    }

    #[test]
    fn token_iter_2() {
        let s = r#"MyProp [Some content\] with \escaped \\characters]"#;

        let mut ti = token_iter(s);
        assert_eq!(ti.next(), Some(Token::TypeName((0, "MyProp"))));
        assert_eq!(ti.next(), Some(Token::OpenLeaf(7)));
        assert_eq!(ti.next(),
                   Some(Token::LeafContents((8, r#"Some content\] with \escaped \\characters"#))));
        assert_eq!(ti.next(), Some(Token::CloseLeaf(49)));
        assert_eq!(ti.next(), None);
    }

    #[test]
    fn token_iter_3() {
        let s = r#"MyThing $\ an\ ident\$\ with\\\{\[\ \#escaped\ content {}"#;

        let mut ti = token_iter(s);
        assert_eq!(ti.next(), Some(Token::TypeName((0, "MyThing"))));
        assert_eq!(ti.next(),
                   Some(Token::Ident((8, r#"$\ an\ ident\$\ with\\\{\[\ \#escaped\ content"#))));
        assert_eq!(ti.next(), Some(Token::OpenInner(55)));
        assert_eq!(ti.next(), Some(Token::CloseInner(56)));
        assert_eq!(ti.next(), None);
    }
}
