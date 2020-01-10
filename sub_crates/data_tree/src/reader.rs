use super::{Error, Event, Parser};

//-------------------------------------------------------------

#[derive(Debug)]
pub enum ReaderError {
    UnexpectedEOF,
    Parse(Error),
    IO(std::io::Error),
}

impl std::error::Error for ReaderError {}

impl std::fmt::Display for ReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<Error> for ReaderError {
    fn from(e: Error) -> Self {
        ReaderError::Parse(e)
    }
}

impl From<std::io::Error> for ReaderError {
    fn from(e: std::io::Error) -> Self {
        ReaderError::IO(e)
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

    pub fn next_event<'a>(&'a mut self) -> Result<Event<'a>, ReaderError> {
        loop {
            let valid_end = match self.parser.next_event()? {
                Event::ValidEnd => true,
                Event::NeedMoreInput => false,
                e => {
                    return Ok(unsafe {
                        // Transmute because the borrow checker is
                        // over-conservative about this.  It thinks
                        // the liftime isn't valid, but since we aren't
                        // mutating self after returning (and in fact
                        // can't because of the borrow) there's no way for
                        // the references in this to become invalid.
                        std::mem::transmute::<Event, Event>(e)
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
                return Err(ReaderError::UnexpectedEOF);
            } else {
                return Ok(Event::ValidEnd);
            }
        }
    }

    pub fn peek_event<'a>(&'a mut self) -> Result<Event<'a>, ReaderError> {
        loop {
            let valid_end = match self.parser.peek_event()? {
                Event::ValidEnd => true,
                Event::NeedMoreInput => false,
                e => {
                    return Ok(unsafe {
                        // Transmute because the borrow checker is
                        // over-conservative about this.  It thinks
                        // the liftime isn't valid, but since we aren't
                        // mutating self after returning (and in fact
                        // can't because of the borrow) there's no way for
                        // the references in this to become invalid.
                        std::mem::transmute::<Event, Event>(e)
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
                return Err(ReaderError::UnexpectedEOF);
            } else {
                return Ok(Event::ValidEnd);
            }
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.parser.byte_offset()
    }
}
