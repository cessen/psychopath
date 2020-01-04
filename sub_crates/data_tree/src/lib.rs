#![allow(dead_code)]

use std::io::{self, Read};

//--------------------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    ExpectedTypeNameOrInnerClose(usize),
    UnexpectedIdent(usize),
    ExpectedOpenOrIdent(usize),
    ExpectedInnerOpen(usize),
    UnexpectedInnerClose(usize),
    UnclosedInnerNode(usize),
    UnexpectedEOF(usize),
    IOError(io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<io::Error> for Error {
    fn from(other: io::Error) -> Self {
        Error::IOError(other)
    }
}

//---------------------------------------------------------------------

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Event<'a> {
    InnerOpen {
        type_name: &'a str,
        ident: Option<&'a str>,
        byte_offset: usize,
    },
    InnerClose {
        byte_offset: usize,
    },
    Leaf {
        type_name: &'a str,
        contents: &'a str,
        byte_offset: usize,
    },
    Done,
}

impl<'a> Event<'a> {
    fn add_to_byte_offset(&self, offset: usize) -> Event<'a> {
        match *self {
            Event::InnerOpen {
                type_name,
                ident,
                byte_offset,
            } => Event::InnerOpen {
                type_name: type_name,
                ident: ident,
                byte_offset: byte_offset + offset,
            },
            Event::InnerClose { byte_offset } => Event::InnerClose {
                byte_offset: byte_offset + offset,
            },
            Event::Leaf {
                type_name,
                contents,
                byte_offset,
            } => Event::Leaf {
                type_name: type_name,
                contents: contents,
                byte_offset: byte_offset + offset,
            },
            Event::Done => *self,
        }
    }
}

//---------------------------------------------------------------------

#[derive(Debug)]
pub struct Parser<R: Read> {
    reader: R,
    buffer: Vec<u8>,
    buf_fill_idx: usize,
    buf_consumed_idx: usize,
    total_bytes_processed: usize,
    inner_opens: usize,
}

impl<R: Read> Parser<R> {
    pub fn new(reader: R) -> Parser<R> {
        Parser {
            reader: reader,
            buffer: Vec::with_capacity(1024),
            buf_fill_idx: 0,
            buf_consumed_idx: 0,
            total_bytes_processed: 0,
            inner_opens: 0,
        }
    }

    pub fn next_event<'a>(&'a mut self) -> Result<Event<'a>, Error> {
        // Remove any consumed data.
        if self.buf_consumed_idx > 0 {
            self.buffer
                .copy_within(self.buf_consumed_idx..self.buf_fill_idx, 0);
            self.buf_fill_idx -= self.buf_consumed_idx;
            self.buf_consumed_idx = 0;
        }

        loop {
            // Read in new data and make a string from the valid prefix.
            let (read_count, valid_count) = self.do_read()?;
            let buffer_text = std::str::from_utf8(&self.buffer[..valid_count]).unwrap();

            // Try to parse an event from the valid prefix.
            match try_parse_event(buffer_text) {
                EventParse::Ok(event, bytes_consumed) => {
                    // Update internal state.
                    self.buf_consumed_idx += bytes_consumed;
                    self.total_bytes_processed += bytes_consumed;
                    if let Event::InnerOpen { .. } = event {
                        self.inner_opens += 1;
                    } else if let Event::InnerClose { byte_offset, .. } = event {
                        if self.inner_opens == 0 {
                            return Err(Error::UnexpectedInnerClose(
                                byte_offset + self.total_bytes_processed,
                            ));
                        } else {
                            self.inner_opens -= 1;
                        }
                    }

                    // Hack the borrow checker, which doesn't understand
                    // loops apparently, and return.
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(
                            event.add_to_byte_offset(
                                self.total_bytes_processed - self.buf_consumed_idx,
                            ),
                        )
                    });
                }
                EventParse::ReachedEnd => {
                    if self.inner_opens == 0 {
                        return Ok(Event::Done);
                    } else {
                        return Err(Error::UnclosedInnerNode(
                            self.total_bytes_processed + valid_count,
                        ));
                    }
                }
                EventParse::IncompleteData => {
                    // If we're at the end, it's a problem.
                    // Otherwise, wait for more data.
                    if read_count == 0 {
                        return Err(Error::UnexpectedEOF(
                            self.total_bytes_processed + valid_count,
                        ));
                    }
                }

                // Hard errors.
                EventParse::ExpectedTypeNameOrInnerClose(byte_offset) => {
                    return Err(Error::ExpectedTypeNameOrInnerClose(
                        byte_offset + self.total_bytes_processed,
                    ));
                }
                EventParse::ExpectedOpenOrIdent(byte_offset) => {
                    return Err(Error::ExpectedOpenOrIdent(
                        byte_offset + self.total_bytes_processed,
                    ));
                }
                EventParse::ExpectedInnerOpen(byte_offset) => {
                    return Err(Error::ExpectedInnerOpen(
                        byte_offset + self.total_bytes_processed,
                    ));
                }
                EventParse::UnexpectedIdent(byte_offset) => {
                    return Err(Error::UnexpectedIdent(
                        byte_offset + self.total_bytes_processed,
                    ));
                }
            }
        }
    }

    /// Returns (read_count, valid_utf8_bytes_count).
    /// The former is how many new bytes were added to the buffer,
    /// and the latter is the total valid prefix utf8 bytes in the
    /// buffer after the read.
    fn do_read(&mut self) -> io::Result<(usize, usize)> {
        // Make sure the buffer has space for more data.
        if (self.buf_fill_idx + 4) >= self.buffer.len() {
            let new_len = ((self.buffer.len() * 3) / 2) + 4;
            self.buffer.resize(new_len, 0);
        }

        // Read!
        let read_count = self.reader.read(&mut self.buffer[self.buf_fill_idx..])?;

        self.buf_fill_idx += read_count;

        // Determine how much of the buffer is valid utf8.
        let valid_count = match std::str::from_utf8(&self.buffer[..self.buf_fill_idx]) {
            Ok(_) => self.buf_fill_idx,
            Err(e) => e.valid_up_to(),
        };

        // Check for invalid utf8.
        if (self.buf_fill_idx - valid_count) >= 4
            || (read_count == 0 && self.buf_fill_idx > valid_count)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream contained invalid UTF-8",
            ));
        }

        return Ok((read_count, valid_count));
    }
}

//--------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum EventParse<'a> {
    Ok(Event<'a>, usize), // (event, bytes consumed)
    ReachedEnd,           // Reached the end of the buffer in a valid state, with no event.
    IncompleteData,       // Need more data to parse.

    // Errors.
    ExpectedTypeNameOrInnerClose(usize),
    ExpectedOpenOrIdent(usize),
    ExpectedInnerOpen(usize),
    UnexpectedIdent(usize),
}

fn try_parse_event<'a>(text: &'a str) -> EventParse<'a> {
    // Remove leading whitespace and comments.
    let mut source_text = skip_ws_and_comments((0, text));
    let start_idx = source_text.0;

    // First token.
    let type_name = match next_token(source_text) {
        // Type name, record and continue.
        (Token::TypeName(tn), tail) => {
            source_text = tail;
            tn
        }

        // Closing tag for inner node.  Return.
        (Token::CloseInner, tail) => {
            return EventParse::Ok(
                Event::InnerClose {
                    byte_offset: start_idx,
                },
                tail.0,
            );
        }

        // We consumed everything as whitespace and/or
        // comments.  Return.
        (Token::End, _) => {
            return EventParse::ReachedEnd;
        }

        // Invalid.
        _ => return EventParse::ExpectedTypeNameOrInnerClose(start_idx),
    };

    // Skip whitespace and comments to get the start of the
    // (possible) ident, for use later in error.
    source_text = skip_ws_and_comments(source_text);
    let ident_start_idx = source_text.0;

    // Possible second token: optional ident.
    let ident = if let (Token::Ident(id), tail) = next_token(source_text) {
        source_text = tail;
        Some(id)
    } else {
        None
    };

    // Skip whitespace and comments to get the start of the
    // where there should be an open tag, for use later in error.
    source_text = skip_ws_and_comments(source_text);
    let open_start_idx = source_text.0;

    // Last part of the event.
    match next_token(source_text) {
        // Begining of an inner node.
        (Token::OpenInner, tail) => {
            return EventParse::Ok(
                Event::InnerOpen {
                    type_name: type_name,
                    ident: ident,
                    byte_offset: start_idx,
                },
                tail.0,
            );
        }

        // Try to parse entire leaf node.
        (Token::OpenLeaf, tail) => {
            if ident != None {
                return EventParse::UnexpectedIdent(ident_start_idx);
            }

            // Get contents.
            let (contents, tail2) = parse_leaf_content(tail);
            source_text = tail2;

            // Try to get closing tag.
            match next_token(source_text) {
                // If it's a leaf closing tag, we're done!
                // Return the leaf event.
                (Token::CloseLeaf, tail) => {
                    return EventParse::Ok(
                        Event::Leaf {
                            type_name: type_name,
                            contents: contents,
                            byte_offset: start_idx,
                        },
                        tail.0,
                    );
                }

                // Otherwise...
                _ => {
                    if source_text.1.is_empty() {
                        // If there's no text left, we're just incomplete.
                        return EventParse::IncompleteData;
                    } else {
                        // Otherwise, this would be a parse error...
                        // except that this shouldn't be reachable,
                        // since everything should be consumable for
                        // leaf content up until a close tag.
                        unreachable!("Expected leaf close tag.")
                    }
                }
            }
        }

        // We consumed everything else as whitespace
        // and/or comments, so we're incomplete.  Return.
        (Token::End, _) => {
            return EventParse::IncompleteData;
        }

        // Invalid.
        _ => {
            if ident == None {
                return EventParse::ExpectedOpenOrIdent(open_start_idx);
            } else {
                return EventParse::ExpectedInnerOpen(open_start_idx);
            }
        }
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

//--------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
        '\n' => true,
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

//--------------------------------------------------------------------------

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
    fn try_parse_event_01() {
        assert_eq!(try_parse_event("H"), EventParse::IncompleteData,);
    }

    #[test]
    fn try_parse_event_02() {
        assert_eq!(try_parse_event("Hello $"), EventParse::IncompleteData,);
    }

    #[test]
    fn try_parse_event_03() {
        assert_eq!(try_parse_event("Hello $id "), EventParse::IncompleteData,);
    }

    #[test]
    fn try_parse_event_04() {
        assert_eq!(
            try_parse_event("Hello $id {"),
            EventParse::Ok(
                Event::InnerOpen {
                    type_name: "Hello",
                    ident: Some("$id"),
                    byte_offset: 0,
                },
                11
            ),
        );
    }

    #[test]
    fn try_parse_event_05() {
        assert_eq!(
            try_parse_event("  Hello $id {"),
            EventParse::Ok(
                Event::InnerOpen {
                    type_name: "Hello",
                    ident: Some("$id"),
                    byte_offset: 2,
                },
                13
            ),
        );
    }

    #[test]
    fn try_parse_event_06() {
        assert_eq!(
            try_parse_event("Hello {"),
            EventParse::Ok(
                Event::InnerOpen {
                    type_name: "Hello",
                    ident: None,
                    byte_offset: 0,
                },
                7
            ),
        );
    }

    #[test]
    fn try_parse_event_07() {
        assert_eq!(
            try_parse_event("Hello {  "),
            EventParse::Ok(
                Event::InnerOpen {
                    type_name: "Hello",
                    ident: None,
                    byte_offset: 0,
                },
                7
            ),
        );
    }

    #[test]
    fn try_parse_event_08() {
        assert_eq!(try_parse_event("Hello ["), EventParse::IncompleteData,);
    }

    #[test]
    fn try_parse_event_09() {
        assert_eq!(
            try_parse_event("Hello [some contents"),
            EventParse::IncompleteData,
        );
    }

    #[test]
    fn try_parse_event_10() {
        assert_eq!(
            try_parse_event("Hello [some contents]"),
            EventParse::Ok(
                Event::Leaf {
                    type_name: "Hello",
                    contents: "some contents",
                    byte_offset: 0,
                },
                21
            ),
        );
    }

    #[test]
    fn try_parse_event_11() {
        assert_eq!(
            try_parse_event("Hello [some contents]  "),
            EventParse::Ok(
                Event::Leaf {
                    type_name: "Hello",
                    contents: "some contents",
                    byte_offset: 0,
                },
                21
            ),
        );
    }

    #[test]
    fn try_parse_event_12() {
        assert_eq!(
            try_parse_event("  # A comment\n\n     "),
            EventParse::ReachedEnd,
        );
    }
}
