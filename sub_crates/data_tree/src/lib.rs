mod parse;

use parse::{ParseError, ParseEvent, Parser};

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
    EOF,
}

//----------------------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    ExpectedTypeNameOrClose(usize),
    ExpectedOpenOrIdent(usize),
    ExpectedOpen(usize),
    UnexpectedClose(usize),
    UnexpectedIdent(usize),
    UnexpectedEOF,
    IO(std::io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        match e {
            ParseError::ExpectedTypeNameOrClose(byte_offset) => {
                Error::ExpectedTypeNameOrClose(byte_offset)
            }
            ParseError::ExpectedOpenOrIdent(byte_offset) => Error::ExpectedOpenOrIdent(byte_offset),
            ParseError::ExpectedOpen(byte_offset) => Error::ExpectedOpen(byte_offset),
            ParseError::UnexpectedClose(byte_offset) => Error::UnexpectedClose(byte_offset),
            ParseError::UnexpectedIdent(byte_offset) => Error::UnexpectedIdent(byte_offset),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

//-------------------------------------------------------------

#[derive(Debug)]
pub struct DataTreeReader<R: std::io::BufRead> {
    parser: Parser,
    reader: R,
    buf: String,
    eof: bool,
}

impl<R: std::io::BufRead> DataTreeReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            parser: Parser::new(),
            reader: reader,
            buf: String::new(),
            eof: false,
        }
    }

    pub fn next_event<'a>(&'a mut self) -> Result<Event<'a>, Error> {
        loop {
            let valid_end = match self.parser.next_event()? {
                ParseEvent::ValidEnd => true,
                ParseEvent::NeedMoreInput => false,

                // The transmutes below are because the borrow checker is
                // over-conservative about this.  It thinks
                // the liftime isn't valid, but since we aren't
                // mutating self after returning (and in fact
                // can't because of the borrow) there's no way for
                // the references in this to become invalid.
                ParseEvent::InnerOpen {
                    type_name,
                    ident,
                    byte_offset,
                } => {
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(Event::InnerOpen {
                            type_name,
                            ident,
                            byte_offset,
                        })
                    });
                }
                ParseEvent::InnerClose { byte_offset } => {
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(Event::InnerClose { byte_offset })
                    });
                }
                ParseEvent::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } => {
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(Event::Leaf {
                            type_name,
                            contents,
                            byte_offset,
                        })
                    });
                }
            };

            if !self.eof {
                self.buf.clear();
                let read = self.reader.read_line(&mut self.buf)?;
                self.parser.push_data(&self.buf);
                if read == 0 {
                    self.eof = true;
                }
            } else if !valid_end {
                return Err(Error::UnexpectedEOF);
            } else {
                return Ok(Event::EOF);
            }
        }
    }

    pub fn peek_event<'a>(&'a mut self) -> Result<Event<'a>, Error> {
        loop {
            let valid_end = match self.parser.peek_event()? {
                ParseEvent::ValidEnd => true,
                ParseEvent::NeedMoreInput => false,

                // The transmutes below are because the borrow checker is
                // over-conservative about this.  It thinks
                // the liftime isn't valid, but since we aren't
                // mutating self after returning (and in fact
                // can't because of the borrow) there's no way for
                // the references in this to become invalid.
                ParseEvent::InnerOpen {
                    type_name,
                    ident,
                    byte_offset,
                } => {
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(Event::InnerOpen {
                            type_name,
                            ident,
                            byte_offset,
                        })
                    });
                }
                ParseEvent::InnerClose { byte_offset } => {
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(Event::InnerClose { byte_offset })
                    });
                }
                ParseEvent::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                } => {
                    return Ok(unsafe {
                        std::mem::transmute::<Event, Event>(Event::Leaf {
                            type_name,
                            contents,
                            byte_offset,
                        })
                    });
                }
            };

            if !self.eof {
                self.buf.clear();
                let read = self.reader.read_line(&mut self.buf)?;
                self.parser.push_data(&self.buf);
                if read == 0 {
                    self.eof = true;
                }
            } else if !valid_end {
                return Err(Error::UnexpectedEOF);
            } else {
                return Ok(Event::EOF);
            }
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.parser.byte_offset()
    }
}
