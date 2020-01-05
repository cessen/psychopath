#![allow(dead_code)]

use std::{io::Cursor, iter::Iterator, result::Result, slice};

use data_tree::{Event, Parser};

#[derive(Debug, Eq, PartialEq)]
pub enum DataTree {
    Internal {
        type_name: String,
        ident: Option<String>,
        children: Vec<DataTree>,
        byte_offset: usize,
    },

    Leaf {
        type_name: String,
        contents: String,
        byte_offset: usize,
    },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParseError {
    Other(&'static str),
}

// #[derive(Copy, Clone, Eq, PartialEq, Debug)]
// pub enum ParseError {
//     MissingOpener(usize),
//     MissingOpenInternal(usize),
//     MissingCloseInternal(usize),
//     MissingOpenLeaf(usize),
//     MissingCloseLeaf(usize),
//     MissingTypeName(usize),
//     UnexpectedIdent(usize),
//     UnknownToken(usize),
//     Other((usize, &'static str)),
// }

impl<'a> DataTree {
    pub fn from_str(source_text: &'a str) -> Result<DataTree, ParseError> {
        let mut parser = Parser::new(Cursor::new(source_text));
        let mut items = Vec::new();

        loop {
            let event = parser.next_event();
            println!("{:?}", event);
            match event {
                Ok(Event::InnerOpen {
                    type_name,
                    ident,
                    byte_offset,
                }) => {
                    let type_name = type_name.into();
                    let ident = ident.map(|id| id.into());
                    items.push(parse_node(&mut parser, type_name, ident, byte_offset)?);
                }
                Ok(Event::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                }) => return Err(ParseError::Other("Unexpected leaf value at root level.")),
                Ok(Event::InnerClose { .. }) => {
                    return Err(ParseError::Other("Unexpected closing tag."))
                }
                Ok(Event::Done) => {
                    break;
                }

                Err(_) => return Err(ParseError::Other("Some error happened.")),
            }
        }

        return Ok(DataTree::Internal {
            type_name: "ROOT".into(),
            ident: None,
            children: items,
            byte_offset: 0,
        });
    }

    pub fn type_name(&'a self) -> &'a str {
        match self {
            DataTree::Internal { type_name, .. } | DataTree::Leaf { type_name, .. } => type_name,
        }
    }

    pub fn ident(&'a self) -> Option<&'a str> {
        match self {
            DataTree::Internal {
                ident: Some(id), ..
            } => Some(id.as_str()),
            _ => None,
        }
    }

    pub fn byte_offset(&'a self) -> usize {
        match self {
            DataTree::Internal { byte_offset, .. } | DataTree::Leaf { byte_offset, .. } => {
                *byte_offset
            }
        }
    }

    pub fn is_internal(&self) -> bool {
        match self {
            DataTree::Internal { .. } => true,
            DataTree::Leaf { .. } => false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            DataTree::Internal { .. } => false,
            DataTree::Leaf { .. } => true,
        }
    }

    pub fn leaf_contents(&'a self) -> Option<&'a str> {
        match self {
            DataTree::Internal { .. } => None,
            DataTree::Leaf { contents, .. } => Some(contents.as_str()),
        }
    }

    pub fn iter_children(&'a self) -> slice::Iter<'a, DataTree> {
        if let DataTree::Internal { ref children, .. } = self {
            children.iter()
        } else {
            [].iter()
        }
    }

    pub fn iter_children_with_type(&'a self, type_name: &'static str) -> DataTreeFilterIter<'a> {
        if let DataTree::Internal { ref children, .. } = self {
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
    fn internal_data_or_panic(&'a self) -> (&'a str, Option<&'a str>, &'a Vec<DataTree>) {
        if let DataTree::Internal {
            type_name,
            ref ident,
            ref children,
            ..
        } = self
        {
            (type_name, ident.as_ref().map(|id| id.as_str()), children)
        } else {
            panic!("Expected DataTree::Internal, found DataTree::Leaf")
        }
    }
    fn leaf_data_or_panic(&'a self) -> (&'a str, &'a str) {
        if let DataTree::Leaf {
            type_name,
            contents,
            ..
        } = self
        {
            (type_name, contents)
        } else {
            panic!("Expected DataTree::Leaf, found DataTree::Internal")
        }
    }
}

fn parse_node<R: std::io::Read>(
    parser: &mut Parser<R>,
    type_name: String,
    ident: Option<String>,
    byte_offset: usize,
) -> Result<DataTree, ParseError> {
    let mut children = Vec::new();
    loop {
        match parser.next_event() {
            Ok(Event::InnerOpen {
                type_name,
                ident,
                byte_offset,
            }) => {
                let type_name = type_name.into();
                let ident = ident.map(|id| id.into());
                children.push(parse_node(parser, type_name, ident, byte_offset)?);
            }
            Ok(Event::Leaf {
                type_name,
                contents,
                byte_offset,
            }) => {
                children.push(DataTree::Leaf {
                    type_name: type_name.into(),
                    contents: contents.into(),
                    byte_offset: byte_offset,
                });
            }
            Ok(Event::InnerClose { .. }) => break,
            Ok(Event::Done) => {
                return Err(ParseError::Other("Unexpected end of contents."));
            }
            Err(_) => {
                return Err(ParseError::Other("Some error happened."));
            }
        }
    }

    Ok(DataTree::Internal {
        type_name: type_name,
        ident: ident,
        children: children,
        byte_offset: byte_offset,
    })
}

/// An iterator over the children of a `DataTree` node that filters out the
/// children not matching a specified type name.
pub struct DataTreeFilterIter<'a> {
    type_name: &'a str,
    iter: slice::Iter<'a, DataTree>,
}

impl<'a> Iterator for DataTreeFilterIter<'a> {
    type Item = &'a DataTree;

    fn next(&mut self) -> Option<&'a DataTree> {
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
    iter: slice::Iter<'a, DataTree>,
}

impl<'a> Iterator for DataTreeFilterInternalIter<'a> {
    type Item = (&'a str, Option<&'a str>, &'a Vec<DataTree>, usize);

    fn next(&mut self) -> Option<(&'a str, Option<&'a str>, &'a Vec<DataTree>, usize)> {
        loop {
            match self.iter.next() {
                Some(DataTree::Internal {
                    type_name,
                    ident,
                    children,
                    byte_offset,
                }) => {
                    if type_name == self.type_name {
                        return Some((
                            type_name,
                            ident.as_ref().map(|id| id.as_str()),
                            children,
                            *byte_offset,
                        ));
                    } else {
                        continue;
                    }
                }

                Some(DataTree::Leaf { .. }) => {
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
    iter: slice::Iter<'a, DataTree>,
}

impl<'a> Iterator for DataTreeFilterLeafIter<'a> {
    type Item = (&'a str, &'a str, usize);

    fn next(&mut self) -> Option<(&'a str, &'a str, usize)> {
        loop {
            match self.iter.next() {
                Some(DataTree::Internal { .. }) => {
                    continue;
                }

                Some(DataTree::Leaf {
                    type_name,
                    contents,
                    byte_offset,
                }) => {
                    if type_name == self.type_name {
                        return Some((type_name, contents, *byte_offset));
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
