#![allow(dead_code)]

use std::{iter::Iterator, result::Result, slice};

#[derive(Debug, Eq, PartialEq)]
pub enum DataTree<'a> {
    Internal {
        type_name: &'a str,
        ident: Option<&'a str>,
        children: Vec<DataTree<'a>>,
        byte_offset: usize,
    },

    Leaf {
        type_name: &'a str,
        contents: &'a str,
        byte_offset: usize,
    },
}

impl<'a> DataTree<'a> {
    pub fn from_str(source_text: &'a str) -> Result<DataTree<'a>, ParseError> {
        let mut items = Vec::new();
        let mut remaining_text = (0, source_text);

        while let Some((item, text)) = parse_node(remaining_text)? {
            remaining_text = text;
            items.push(item);
        }

        remaining_text = skip_ws_and_comments(remaining_text);

        if remaining_text.1.is_empty() {
            return Ok(DataTree::Internal {
                type_name: "ROOT",
                ident: None,
                children: items,
                byte_offset: 0,
            });
        } else {
            // If the whole text wasn't parsed, something went wrong.
            return Err(ParseError::Other((0, "Failed to parse the entire string.")));
        }
    }

    pub fn type_name(&'a self) -> &'a str {
        match *self {
            DataTree::Internal { type_name, .. } | DataTree::Leaf { type_name, .. } => type_name,
        }
    }

    pub fn ident(&'a self) -> Option<&'a str> {
        match *self {
            DataTree::Internal { ident, .. } => ident,
            DataTree::Leaf { .. } => None,
        }
    }

    pub fn byte_offset(&'a self) -> usize {
        match *self {
            DataTree::Internal { byte_offset, .. } | DataTree::Leaf { byte_offset, .. } => {
                byte_offset
            }
        }
    }

    pub fn is_internal(&self) -> bool {
        match *self {
            DataTree::Internal { .. } => true,
            DataTree::Leaf { .. } => false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        match *self {
            DataTree::Internal { .. } => false,
            DataTree::Leaf { .. } => true,
        }
    }

    pub fn leaf_contents(&'a self) -> Option<&'a str> {
        match *self {
            DataTree::Internal { .. } => None,
            DataTree::Leaf { contents, .. } => Some(contents),
        }
    }

    pub fn iter_children(&'a self) -> slice::Iter<'a, DataTree<'a>> {
        if let DataTree::Internal { ref children, .. } = *self {
            children.iter()
        } else {
            [].iter()
        }
    }

    pub fn iter_children_with_type(&'a self, type_name: &'static str) -> DataTreeFilterIter<'a> {
        if let DataTree::Internal { ref children, .. } = *self {
            DataTreeFilterIter {
                type_name: type_name,
                iter: children.iter(),
            }
        } else {
            DataTreeFilterIter {
                type_name: type_name,
                iter: [].iter(),
            }
        }
    }

    pub fn iter_internal_children_with_type(
        &'a self,
        type_name: &'static str,
    ) -> DataTreeFilterInternalIter<'a> {
        if let DataTree::Internal { ref children, .. } = *self {
            DataTreeFilterInternalIter {
                type_name: type_name,
                iter: children.iter(),
            }
        } else {
            DataTreeFilterInternalIter {
                type_name: type_name,
                iter: [].iter(),
            }
        }
    }

    pub fn iter_leaf_children_with_type(
        &'a self,
        type_name: &'static str,
    ) -> DataTreeFilterLeafIter<'a> {
        if let DataTree::Internal { ref children, .. } = *self {
            DataTreeFilterLeafIter {
                type_name: type_name,
                iter: children.iter(),
            }
        } else {
            DataTreeFilterLeafIter {
                type_name: type_name,
                iter: [].iter(),
            }
        }
    }

    // For unit tests
    fn internal_data_or_panic(&'a self) -> (&'a str, Option<&'a str>, &'a Vec<DataTree<'a>>) {
        if let DataTree::Internal {
            type_name,
            ident,
            ref children,
            ..
        } = *self
        {
            (type_name, ident, children)
        } else {
            panic!("Expected DataTree::Internal, found DataTree::Leaf")
        }
    }
    fn leaf_data_or_panic(&'a self) -> (&'a str, &'a str) {
        if let DataTree::Leaf {
            type_name,
            contents,
            ..
        } = *self
        {
            (type_name, contents)
        } else {
            panic!("Expected DataTree::Leaf, found DataTree::Internal")
        }
    }
}

/// An iterator over the children of a `DataTree` node that filters out the
/// children not matching a specified type name.
pub struct DataTreeFilterIter<'a> {
    type_name: &'a str,
    iter: slice::Iter<'a, DataTree<'a>>,
}

impl<'a> Iterator for DataTreeFilterIter<'a> {
    type Item = &'a DataTree<'a>;

    fn next(&mut self) -> Option<&'a DataTree<'a>> {
        loop {
            if let Some(dt) = self.iter.next() {
                if dt.type_name() == self.type_name {
                    return Some(dt);
                } else {
                    continue;
                }
            } else {
                return None;
            }
        }
    }
}

/// An iterator over the children of a `DataTree` node that filters out the
/// children that aren't internal nodes and that don't match a specified
/// type name.
pub struct DataTreeFilterInternalIter<'a> {
    type_name: &'a str,
    iter: slice::Iter<'a, DataTree<'a>>,
}

impl<'a> Iterator for DataTreeFilterInternalIter<'a> {
    type Item = (&'a str, Option<&'a str>, &'a Vec<DataTree<'a>>, usize);

    fn next(&mut self) -> Option<(&'a str, Option<&'a str>, &'a Vec<DataTree<'a>>, usize)> {
        loop {
            match self.iter.next() {
                Some(&DataTree::Internal {
                    type_name,
                    ident,
                    ref children,
                    byte_offset,
                }) => {
                    if type_name == self.type_name {
                        return Some((type_name, ident, children, byte_offset));
                    } else {
                        continue;
                    }
                }

                Some(&DataTree::Leaf { .. }) => {
                    continue;
                }

                None => {
                    return None;
                }
            }
        }
    }
}

/// An iterator over the children of a `DataTree` node that filters out the
/// children that aren't internal nodes and that don't match a specified
/// type name.
pub struct DataTreeFilterLeafIter<'a> {
    type_name: &'a str,
    iter: slice::Iter<'a, DataTree<'a>>,
}

impl<'a> Iterator for DataTreeFilterLeafIter<'a> {
    type Item = (&'a str, &'a str, usize);

    fn next(&mut self) -> Option<(&'a str, &'a str, usize)> {
        loop {
            match self.iter.next() {
                Some(&DataTree::Internal { .. }) => {
                    continue;
                }

                Some(&DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                }) => {
                    if type_name == self.type_name {
                        return Some((type_name, contents, byte_offset));
                    } else {
                        continue;
                    }
                }

                None => {
                    return None;
                }
            }
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
                    while let Some((node, text4)) = parse_node(text_remaining)? {
                        text_remaining = text4;
                        children.push(node);
                    }
                    if let (Token::CloseInner, text4) = next_token(text_remaining) {
                        return Ok(Some((
                            DataTree::Internal {
                                type_name: type_name,
                                ident: Some(n),
                                children: children,
                                byte_offset: text1.0,
                            },
                            text4,
                        )));
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
                while let Some((node, text3)) = parse_node(text_remaining)? {
                    text_remaining = text3;
                    children.push(node);
                }

                if let (Token::CloseInner, text3) = next_token(text_remaining) {
                    return Ok(Some((
                        DataTree::Internal {
                            type_name: type_name,
                            ident: None,
                            children: children,
                            byte_offset: text1.0,
                        },
                        text3,
                    )));
                } else {
                    return Err(ParseError::MissingCloseInternal(text_remaining.0));
                }
            }

            // Leaf
            (Token::OpenLeaf, text2) => {
                let (contents, text3) = parse_leaf_content(text2);
                if let (Token::CloseLeaf, text4) = next_token(text3) {
                    return Ok(Some((
                        DataTree::Leaf {
                            type_name: type_name,
                            contents: contents,
                            byte_offset: text1.0,
                        },
                        text4,
                    )));
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

fn parse_leaf_content(source_text: (usize, &str)) -> (&str, (usize, &str)) {
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

    return (
        &source_text.1[0..si],
        (source_text.0 + si, &source_text.1[si..]),
    );
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

                return (
                    Token::Ident(&text1.1[0..si]),
                    (text1.0 + si, &text1.1[si..]),
                );
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

                    return (
                        Token::TypeName(&text1.1[0..si]),
                        (text1.0 + si, &text1.1[si..]),
                    );
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

fn skip_ws(text: &str) -> &str {
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

fn skip_comment(text: &str) -> &str {
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

fn skip_ws_and_comments(text: (usize, &str)) -> (usize, &str) {
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

        assert_eq!(
            next_token(input),
            (Token::Ident("$hi\\ t\\#he\\[re"), (15, " "),)
        );
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

        assert_eq!(
            (token1, input2),
            (
                Token::TypeName("Thing"),
                (5, " $yar { # A comment\n\tThing2 []\n}",)
            )
        );
        assert_eq!(
            (token2, input3),
            (
                Token::Ident("$yar"),
                (10, " { # A comment\n\tThing2 []\n}",)
            )
        );
        assert_eq!(
            (token3, input4),
            (Token::OpenInner, (12, " # A comment\n\tThing2 []\n}",))
        );
        assert_eq!(
            (token4, input5),
            (Token::TypeName("Thing2"), (32, " []\n}"))
        );
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

    #[test]
    fn iter_1() {
        let dt = DataTree::from_str(
            r#"
            A {}
            B {}
            A []
            A {}
            B {}
        "#,
        )
        .unwrap();

        let i = dt.iter_children_with_type("A");
        assert_eq!(i.count(), 3);
    }

    #[test]
    fn iter_2() {
        let dt = DataTree::from_str(
            r#"
            A {}
            B {}
            A []
            A {}
            B {}
        "#,
        )
        .unwrap();

        let i = dt.iter_internal_children_with_type("A");
        assert_eq!(i.count(), 2);
    }

    #[test]
    fn iter_3() {
        let dt = DataTree::from_str(
            r#"
            A []
            B {}
            A {}
            A []
            B {}
        "#,
        )
        .unwrap();

        let i = dt.iter_leaf_children_with_type("A");
        assert_eq!(i.count(), 2);
    }
}
