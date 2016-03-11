#![allow(dead_code)]

use std::result::Result;
use std::cmp::Eq;

#[derive(Debug, Eq, PartialEq)]
pub enum DataTree<'a> {
    Internal {
        type_name: &'a str,
        ident: Option<&'a str>,
        children: Vec<DataTree<'a>>,
    },

    Leaf {
        type_name: &'a str,
        contents: &'a str,
    },
}


impl<'a> DataTree<'a> {
    pub fn from_str(source_text: &'a str) -> Result<DataTree<'a>, ParseError> {
        let mut items = Vec::new();
        let mut remaining_text = (0, source_text);

        while let Some((item, text)) = try!(parse_node(remaining_text)) {
            remaining_text = text;
            items.push(item);
        }

        remaining_text = skip_ws_and_comments(remaining_text);

        if remaining_text.1.len() == 0 {
            return Ok(DataTree::Internal {
                type_name: "ROOT",
                ident: None,
                children: items,
            });
        } else {
            // If the whole text wasn't parsed, something went wrong.
            return Err(ParseError::Other((0, "Failed to parse the entire string.")));
        }
    }

    pub fn get_first_child_with_type_name(&'a self, type_name: &str) -> Option<&'a DataTree> {
        if let &DataTree::Internal { ref children, .. } = self {
            for child in children.iter() {
                match child {
                    &DataTree::Internal { type_name: tn, .. } => {
                        if tn == type_name {
                            return Some(child);
                        }
                    }

                    &DataTree::Leaf { type_name: tn, .. } => {
                        if tn == type_name {
                            return Some(child);
                        }
                    }
                }
            }
            return None;
        } else {
            return None;
        }
    }

    pub fn count_children_with_type_name(&self, type_name: &str) -> usize {
        if let &DataTree::Internal { ref children, .. } = self {
            let mut count = 0;
            for child in children.iter() {
                match child {
                    &DataTree::Internal { type_name: tn, .. } => {
                        if tn == type_name {
                            count += 1;
                        }
                    }

                    &DataTree::Leaf { type_name: tn, .. } => {
                        if tn == type_name {
                            count += 1;
                        }
                    }
                }
            }
            return count;
        } else {
            return 0;
        }
    }

    // For unit tests
    fn internal_data_or_panic(&'a self) -> (&'a str, Option<&'a str>, &'a Vec<DataTree<'a>>) {
        if let DataTree::Internal { type_name, ident, ref children } = *self {
            (type_name, ident, children)
        } else {
            panic!("Expected DataTree::Internal, found DataTree::Leaf")
        }
    }
    fn leaf_data_or_panic(&'a self) -> (&'a str, &'a str) {
        if let DataTree::Leaf { type_name, contents } = *self {
            (type_name, contents)
        } else {
            panic!("Expected DataTree::Leaf, found DataTree::Internal")
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParseError {
    MissingOpener(usize),
    MissingOpenInternal(usize),
    MissingCloseInternal(usize),
    MissingOpenLeaf(usize),
    MissingCloseLeaf(usize),
    MissingTypeName(usize),
    UnexpectedIdent(usize),
    UnknownToken(usize),
    Other((usize, &'static str)),
}




// ================================================================

#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    OpenInner,
    CloseInner,
    OpenLeaf,
    CloseLeaf,
    TypeName(&'a str),
    Ident(&'a str),
    End,
    Unknown,
}

type ParseResult<'a> = Result<Option<(DataTree<'a>, (usize, &'a str))>, ParseError>;

fn parse_node<'a>(source_text: (usize, &'a str)) -> ParseResult<'a> {
    let (token, text1) = next_token(source_text);
    if let Token::TypeName(type_name) = token {
        match next_token(text1) {
            // Internal with name
            (Token::Ident(n), text2) => {
                if let (Token::OpenInner, text3) = next_token(text2) {
                    let mut children = Vec::new();
                    let mut text_remaining = text3;
                    while let Some((node, text4)) = try!(parse_node(text_remaining)) {
                        text_remaining = text4;
                        children.push(node);
                    }
                    if let (Token::CloseInner, text4) = next_token(text_remaining) {
                        return Ok(Some((DataTree::Internal {
                            type_name: type_name,
                            ident: Some(n),
                            children: children,
                        },
                                        text4)));
                    } else {
                        return Err(ParseError::MissingCloseInternal(text_remaining.0));
                    }
                } else {
                    return Err(ParseError::MissingOpenInternal(text2.0));
                }
            }

            // Internal without name
            (Token::OpenInner, text2) => {
                let mut children = Vec::new();
                let mut text_remaining = text2;
                while let Some((node, text3)) = try!(parse_node(text_remaining)) {
                    text_remaining = text3;
                    children.push(node);
                }

                if let (Token::CloseInner, text3) = next_token(text_remaining) {
                    return Ok(Some((DataTree::Internal {
                        type_name: type_name,
                        ident: None,
                        children: children,
                    },
                                    text3)));
                } else {
                    return Err(ParseError::MissingCloseInternal(text_remaining.0));
                }
            }

            // Leaf
            (Token::OpenLeaf, text2) => {
                let (contents, text3) = parse_leaf_content(text2);
                if let (Token::CloseLeaf, text4) = next_token(text3) {
                    return Ok(Some((DataTree::Leaf {
                        type_name: type_name,
                        contents: contents,
                    },
                                    text4)));
                } else {
                    return Err(ParseError::MissingCloseLeaf(text3.0));
                }
            }

            // Other
            _ => {
                return Err(ParseError::MissingOpener(text1.0));
            }
        }
    } else {
        return Ok(None);
    }
}


fn parse_leaf_content<'a>(source_text: (usize, &'a str)) -> (&'a str, (usize, &'a str)) {
    let mut si = 1;
    let mut escaped = false;
    let mut reached_end = true;
    for (i, c) in source_text.1.char_indices() {
        si = i;
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == ']' {
            reached_end = false;
            break;
        }
    }

    if reached_end {
        si = source_text.1.len();
    }

    return (&source_text.1[0..si],
            (source_text.0 + si, &source_text.1[si..]));
}


fn next_token<'a>(source_text: (usize, &'a str)) -> (Token<'a>, (usize, &'a str)) {
    let text1 = skip_ws_and_comments(source_text);

    if let Some(c) = text1.1.chars().nth(0) {
        let text2 = (text1.0 + c.len_utf8(), &text1.1[c.len_utf8()..]);
        match c {
            '{' => {
                return (Token::OpenInner, text2);
            }

            '}' => {
                return (Token::CloseInner, text2);
            }

            '[' => {
                return (Token::OpenLeaf, text2);
            }

            ']' => {
                return (Token::CloseLeaf, text2);
            }

            '$' => {
                // Parse name
                let mut si = 1;
                let mut escaped = false;
                let mut reached_end = true;
                for (i, c) in text1.1.char_indices().skip(1) {
                    si = i;
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if !is_ident_char(c) {
                        reached_end = false;
                        break;
                    }
                }

                if reached_end {
                    si = text1.1.len();
                }

                return (Token::Ident(&text1.1[0..si]),
                        (text1.0 + si, &text1.1[si..]));
            }

            _ => {
                if is_ident_char(c) {
                    // Parse type
                    let mut si = 0;
                    let mut reached_end = true;
                    for (i, c) in text1.1.char_indices() {
                        si = i;
                        if !is_ident_char(c) {
                            reached_end = false;
                            break;
                        }
                    }

                    if reached_end {
                        si = text1.1.len();
                    }

                    return (Token::TypeName(&text1.1[0..si]),
                            (text1.0 + si, &text1.1[si..]));
                }
            }

        }
    } else {
        return (Token::End, text1);
    }

    return (Token::Unknown, text1);
}

fn is_ws(c: char) -> bool {
    match c {
        '\n' | '\r' | '\t' | ' ' => true,
        _ => false,
    }
}

fn is_nl(c: char) -> bool {
    match c {
        '\n' | '\r' => true,
        _ => false,
    }
}

fn is_reserved_char(c: char) -> bool {
    match c {
        '{' | '}' | '[' | ']' | '$' | '#' | '\\' => true,
        _ => false,
    }
}

fn is_ident_char(c: char) -> bool {
    // Anything that isn't whitespace or a reserved character
    !is_ws(c) && !is_reserved_char(c)
}

fn skip_ws<'a>(text: &'a str) -> &'a str {
    let mut si = 0;
    let mut reached_end = true;
    for (i, c) in text.char_indices() {
        si = i;
        if !is_ws(c) {
            reached_end = false;
            break;
        }
    }

    if reached_end {
        si = text.len();
    }

    return &text[si..];
}

fn skip_comment<'a>(text: &'a str) -> &'a str {
    let mut si = 0;
    if Some('#') == text.chars().nth(0) {
        let mut reached_end = true;
        for (i, c) in text.char_indices() {
            si = i;
            if is_nl(c) {
                reached_end = false;
                break;
            }
        }

        if reached_end {
            si = text.len();
        }
    }

    return &text[si..];
}

fn skip_ws_and_comments<'a>(text: (usize, &'a str)) -> (usize, &'a str) {
    let mut remaining_text = text.1;

    loop {
        let tmp = skip_comment(skip_ws(remaining_text));

        if tmp.len() == remaining_text.len() {
            break;
        } else {
            remaining_text = tmp;
        }
    }

    let offset = text.0 + text.1.len() - remaining_text.len();
    return (offset, remaining_text);
}




// ================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::{next_token, Token};

    #[test]
    fn tokenize_1() {
        let input = (0, "Thing");

        assert_eq!(next_token(input), (Token::TypeName("Thing"), (5, "")));
    }

    #[test]
    fn tokenize_2() {
        let input = (0, "  \n# gdfgdf gfdg dggdf\\sg dfgsd \n   Thing");

        assert_eq!(next_token(input), (Token::TypeName("Thing"), (41, "")));
    }

    #[test]
    fn tokenize_3() {
        let input1 = (0, " Thing { }");
        let (token1, input2) = next_token(input1);
        let (token2, input3) = next_token(input2);
        let (token3, input4) = next_token(input3);

        assert_eq!((token1, input2.1), (Token::TypeName("Thing"), " { }"));
        assert_eq!((token2, input3.1), (Token::OpenInner, " }"));
        assert_eq!((token3, input4.1), (Token::CloseInner, ""));
    }

    #[test]
    fn tokenize_4() {
        let input = (0, " $hi_there ");

        assert_eq!(next_token(input), (Token::Ident("$hi_there"), (10, " ")));
    }

    #[test]
    fn tokenize_5() {
        let input = (0, " $hi\\ t\\#he\\[re ");

        assert_eq!(next_token(input),
                   (Token::Ident("$hi\\ t\\#he\\[re"), (15, " ")));
    }

    #[test]
    fn tokenize_6() {
        let input1 = (0, " $hi the[re");
        let (token1, input2) = next_token(input1);
        let (token2, input3) = next_token(input2);
        let (token3, input4) = next_token(input3);
        let (token4, input5) = next_token(input4);
        let (token5, input6) = next_token(input5);

        assert_eq!((token1, input2), (Token::Ident("$hi"), (4, " the[re")));
        assert_eq!((token2, input3), (Token::TypeName("the"), (8, "[re")));
        assert_eq!((token3, input4), (Token::OpenLeaf, (9, "re")));
        assert_eq!((token4, input5), (Token::TypeName("re"), (11, "")));
        assert_eq!((token5, input6), (Token::End, (11, "")));
    }

    #[test]
    fn tokenize_7() {
        let input1 = (0, "Thing $yar { # A comment\n\tThing2 []\n}");
        let (token1, input2) = next_token(input1);
        let (token2, input3) = next_token(input2);
        let (token3, input4) = next_token(input3);
        let (token4, input5) = next_token(input4);
        let (token5, input6) = next_token(input5);
        let (token6, input7) = next_token(input6);
        let (token7, input8) = next_token(input7);
        let (token8, input9) = next_token(input8);

        assert_eq!((token1, input2),
                   (Token::TypeName("Thing"),
                    (5, " $yar { # A comment\n\tThing2 []\n}")));
        assert_eq!((token2, input3),
                   (Token::Ident("$yar"), (10, " { # A comment\n\tThing2 []\n}")));
        assert_eq!((token3, input4),
                   (Token::OpenInner, (12, " # A comment\n\tThing2 []\n}")));
        assert_eq!((token4, input5),
                   (Token::TypeName("Thing2"), (32, " []\n}")));
        assert_eq!((token5, input6), (Token::OpenLeaf, (34, "]\n}")));
        assert_eq!((token6, input7), (Token::CloseLeaf, (35, "\n}")));
        assert_eq!((token7, input8), (Token::CloseInner, (37, "")));
        assert_eq!((token8, input9), (Token::End, (37, "")));
    }

    #[test]
    fn parse_1() {
        let input = r#"
            Thing {}
        "#;

        let dt = DataTree::from_str(input).unwrap();

        // Root
        let (t, i, c) = dt.internal_data_or_panic();
        assert_eq!(t, "ROOT");
        assert_eq!(i, None);
        assert_eq!(c.len(), 1);

        // First (and only) child
        let (t, i, c) = c[0].internal_data_or_panic();
        assert_eq!(t, "Thing");
        assert_eq!(i, None);
        assert_eq!(c.len(), 0);
    }
}
