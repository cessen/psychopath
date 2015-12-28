#![allow(dead_code)]

use std::result;
use std::cmp::Eq;

#[derive(Debug)]
pub enum DataTree<'a> {
    Internal {
        type_: &'a str,
        name: Option<&'a str>,
        children: Vec<DataTree<'a>>,
    },

    Leaf {
        type_: &'a str,
        contents: &'a str,
    },
}


impl<'a> DataTree<'a> {
    pub fn from_str(source_text: &'a str) -> Option<Vec<DataTree<'a>>> {
        let mut items = Vec::new();
        let mut remaining_text = source_text;

        while let Ok((item, text)) = parse(remaining_text) {
            remaining_text = text;
            items.push(item);
        }

        remaining_text = skip_ws_and_comments(remaining_text);

        if remaining_text.len() > 0 {
            return None;
        } else {
            return Some(items);
        }
    }
}


#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    OpenInner,
    CloseInner,
    OpenLeaf,
    CloseLeaf,
    Type(&'a str),
    Name(&'a str),
    End,
    Unknown,
}

type ParseResult<'a> = result::Result<(DataTree<'a>, &'a str), ()>;


fn parse<'a>(source_text: &'a str) -> ParseResult<'a> {
    let (token, text1) = next_token(source_text);

    if let Token::Type(t) = token {
        match next_token(text1) {
            // Inner with name
            (Token::Name(n), text2) => {
                if let (Token::OpenInner, text3) = next_token(text2) {
                    let mut children = Vec::new();
                    let mut text_remaining = text3;
                    while let Ok((node, text4)) = parse(text_remaining) {
                        text_remaining = text4;
                        children.push(node);
                    }
                    if let (Token::CloseInner, text4) = next_token(text_remaining) {
                        return Ok((DataTree::Internal {
                            type_: t,
                            name: Some(n),
                            children: children,
                        },
                                   text4));
                    } else {
                        return Err(());
                    }
                } else {
                    return Err(());
                }
            }

            // Inner without name
            (Token::OpenInner, text2) => {
                let mut children = Vec::new();
                let mut text_remaining = text2;
                while let Ok((node, text3)) = parse(text_remaining) {
                    text_remaining = text3;
                    children.push(node);
                }
                if let (Token::CloseInner, text3) = next_token(text2) {
                    return Ok((DataTree::Internal {
                        type_: t,
                        name: None,
                        children: children,
                    },
                               text3));
                } else {
                    return Err(());
                }
            }

            // Leaf
            (Token::OpenLeaf, text2) => {
                if let Ok((lc, text3)) = parse_leaf_content(text2) {
                    if let (Token::CloseLeaf, text4) = next_token(text3) {
                        return Ok((DataTree::Leaf {
                            type_: t,
                            contents: lc,
                        },
                                   text4));
                    } else {
                        return Err(());
                    }
                } else {
                    return Err(());
                }
            }

            // Other
            _ => {
                return Err(());
            }
        }
    } else {
        return Err(());
    }
}


fn parse_leaf_content<'a>(source_text: &'a str) -> result::Result<(&'a str, &'a str), ()> {
    let mut escape = false;

    for (i, c) in source_text.char_indices() {
        if escape {
            escape = false;
            continue;
        }

        if c == ']' {
            return Ok((&source_text[0..i], &source_text[i..]));
        } else if c == '\\' {
            escape = true;
        }
    }

    return Err(());
}


pub fn next_token<'a>(source_text: &'a str) -> (Token<'a>, &'a str) {
    let text1 = skip_ws_and_comments(source_text);

    if let Some(c) = text1.chars().nth(0) {
        let text2 = &text1[c.len_utf8()..];
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
                let mut si = 0;
                let mut escape = false;
                let mut broke = false;

                for (i, c) in text2.char_indices() {
                    if c == '\\' {
                        escape = true;
                    } else if (is_reserved_char(c) || is_ws(c)) && !escape {
                        si = i;
                        broke = true;
                        break;
                    } else {
                        escape = false;
                    }
                }

                if broke {
                    return (Token::Name(&text1[0..si + 1]), &text1[si + 1..]);
                } else {
                    return (Token::Name(text1), "");
                }
            }

            _ => {
                // Parse type
                let mut si = 0;
                let mut broke = false;

                for (i, c) in text1.char_indices() {
                    if (is_reserved_char(c) || is_ws(c)) && c != '\\' {
                        si = i;
                        broke = true;
                        break;
                    }
                }

                if broke {
                    return (Token::Type(&text1[0..si]), &text1[si..]);
                } else {
                    return (Token::Type(text1), "");
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

fn is_reserved_char(c: char) -> bool {
    match c {
        '{' | '}' | '[' | ']' | '$' | '\\' => true,
        _ => false,
    }
}

fn skip_ws<'a>(text: &'a str) -> Option<&'a str> {
    for (i, c) in text.char_indices() {
        if !is_ws(c) {
            if i > 0 {
                return Some(&text[i..]);
            } else {
                return None;
            }
        }
    }

    if text.len() > 0 {
        return Some("");
    } else {
        return None;
    }
}

fn skip_comment<'a>(text: &'a str) -> Option<&'a str> {
    let mut tci = text.char_indices();
    if let Some((_, '#')) = tci.next() {
        for (i, c) in tci {
            match c {
                '\n' | '\r' => {
                    return Some(&text[i..]);
                }

                _ => {}
            }
        }

        return Some("");
    } else {
        return None;
    }
}

fn skip_ws_and_comments<'a>(text: &'a str) -> &'a str {
    let mut remaining_text = text;

    loop {
        let mut ws = 0;
        let mut comment = 0;

        while let Some(t) = skip_ws(remaining_text) {
            remaining_text = t;
            ws += 1;
        }

        while let Some(t) = skip_comment(remaining_text) {
            remaining_text = t;
            comment += 1;
        }

        if ws == 0 && comment == 0 {
            break;
        }
    }

    return remaining_text;
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_1() {
        let input = "Thing";

        assert_eq!(next_token(input), (Token::Type("Thing"), ""));
    }

    #[test]
    fn test_tokenize_2() {
        let input = "  \n# gdfgdf gfdg dggdf\\sg dfgsd \n   Thing";

        assert_eq!(next_token(input), (Token::Type("Thing"), ""));
    }

    #[test]
    fn test_tokenize_3() {
        let input1 = " Thing { }";
        let (token1, input2) = next_token(input1);
        let (token2, input3) = next_token(input2);
        let (token3, input4) = next_token(input3);

        assert_eq!((token1, input2), (Token::Type("Thing"), " { }"));
        assert_eq!((token2, input3), (Token::OpenInner, " }"));
        assert_eq!((token3, input4), (Token::CloseInner, ""));
    }

    #[test]
    fn test_tokenize_4() {
        let input = " $hi_there ";

        assert_eq!(next_token(input), (Token::Name("$hi_there"), " "));
    }

    #[test]
    fn test_tokenize_5() {
        let input = " $hi\\ t\\#he\\[re ";

        assert_eq!(next_token(input), (Token::Name("$hi\\ t\\#he\\[re"), " "));
    }

    #[test]
    fn test_tokenize_6() {
        let input1 = " $hi the[re";
        let (token1, input2) = next_token(input1);
        let (token2, input3) = next_token(input2);
        let (token3, input4) = next_token(input3);
        let (token4, input5) = next_token(input4);

        assert_eq!((token1, input2), (Token::Name("$hi"), " the[re"));
        assert_eq!((token2, input3), (Token::Type("the"), "[re"));
        assert_eq!((token3, input4), (Token::OpenLeaf, "re"));
        assert_eq!((token4, input5), (Token::Type("re"), ""));
    }

    #[test]
    fn test_tokenize_7() {
        let input1 = "Thing $yar { # A comment\n\tThing2 []\n}";
        let (token1, input2) = next_token(input1);
        let (token2, input3) = next_token(input2);
        let (token3, input4) = next_token(input3);
        let (token4, input5) = next_token(input4);
        let (token5, input6) = next_token(input5);
        let (token6, input7) = next_token(input6);

        assert_eq!((token1, input2),
                   (Token::Type("Thing"), " $yar { # A comment\n\tThing2 []\n}"));
        assert_eq!((token2, input3),
                   (Token::Name("$yar"), " { # A comment\n\tThing2 []\n}"));
        assert_eq!((token3, input4),
                   (Token::OpenInner, " # A comment\n\tThing2 []\n}"));
        assert_eq!((token4, input5), (Token::Type("Thing2"), " []\n}"));
        assert_eq!((token5, input6), (Token::OpenLeaf, "]\n}"));
        assert_eq!((token6, input7), (Token::CloseLeaf, "\n}"));
    }
}
