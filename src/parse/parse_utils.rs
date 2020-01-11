//! Some basic nom parsers
#![allow(dead_code)]

use std::{
    io::BufRead,
    str::{self, FromStr},
};

use nom::{
    character::complete::{digit1, multispace0, one_of},
    combinator::{map_res, opt, recognize},
    number::complete::float,
    sequence::{delimited, tuple},
    IResult,
};

use data_tree::{DataTreeReader, Event};

use super::psy::{PsyError, PsyResult};

// ========================================================

pub fn ws_f32(input: &str) -> IResult<&str, f32, ()> {
    delimited(multispace0, float, multispace0)(input)
}

pub fn ws_u32(input: &str) -> IResult<&str, u32, ()> {
    map_res(delimited(multispace0, digit1, multispace0), u32::from_str)(input)
}

pub fn ws_usize(input: &str) -> IResult<&str, usize, ()> {
    map_res(delimited(multispace0, digit1, multispace0), usize::from_str)(input)
}

pub fn ws_i32(input: &str) -> IResult<&str, i32, ()> {
    map_res(
        delimited(
            multispace0,
            recognize(tuple((opt(one_of("-")), digit1))),
            multispace0,
        ),
        i32::from_str,
    )(input)
}

//---------------------------------------------------------

/// Ensures that we encounter a InnerClose event, and returns a useful
/// error if we don't.
pub fn ensure_close(events: &mut DataTreeReader<impl BufRead>) -> PsyResult<()> {
    match events.next_event()? {
        Event::InnerClose { .. } => Ok(()),
        Event::InnerOpen {
            type_name,
            byte_offset,
            ..
        } => Err(PsyError::ExpectedInternalNodeClose(
            byte_offset,
            format!(
                "Expected the node to be closed, but instead found a '{}'.",
                type_name
            ),
        )),
        Event::Leaf {
            type_name,
            byte_offset,
            ..
        } => Err(PsyError::ExpectedInternalNodeClose(
            byte_offset,
            format!(
                "Expected the node to be closed, but instead found a '{}'.",
                type_name
            ),
        )),
        _ => Err(PsyError::UnknownError(events.byte_offset())),
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Range {
    Full,
    From(usize),
    To(usize),
    Range(usize, usize),
}

impl Range {
    pub fn contains(self, n: usize) -> bool {
        match self {
            Range::Full => true,
            Range::From(start) => n >= start,
            Range::To(end) => n < end,
            Range::Range(start, end) => n >= start && n < end,
        }
    }

    /// Checks if the value is within the upper bound of the range.
    /// Ignores any lower bound.
    pub fn contains_upper(self, n: usize) -> bool {
        match self {
            Range::Full | Range::From(_) => true,
            Range::To(end) => n < end,
            Range::Range(_, end) => n < end,
        }
    }

    pub fn lower(self) -> usize {
        match self {
            Range::Full => 0,
            Range::From(start) => start,
            Range::To(_) => 0,
            Range::Range(start, _) => start,
        }
    }

    pub fn upper(self) -> usize {
        match self {
            Range::Full => std::usize::MAX,
            Range::From(_) => std::usize::MAX,
            Range::To(end) => end,
            Range::Range(_, end) => end,
        }
    }
}

impl std::convert::From<std::ops::RangeFull> for Range {
    fn from(_r: std::ops::RangeFull) -> Self {
        Range::Full
    }
}

impl std::convert::From<std::ops::RangeFrom<usize>> for Range {
    fn from(r: std::ops::RangeFrom<usize>) -> Self {
        Range::From(r.start)
    }
}

impl std::convert::From<std::ops::RangeTo<usize>> for Range {
    fn from(r: std::ops::RangeTo<usize>) -> Self {
        Range::To(r.end)
    }
}

impl std::convert::From<std::ops::Range<usize>> for Range {
    fn from(r: std::ops::Range<usize>) -> Self {
        Range::Range(r.start, r.end)
    }
}

impl std::convert::From<usize> for Range {
    fn from(r: usize) -> Self {
        Range::Range(r, r + 1)
    }
}

/// Acts as an intermediary for parsing, ensuring that the right number of the
/// right subsections are encountered.  It loops over subsections, passing
/// through the `events` object untouched, so the passed closure needs to call
/// `next_event`.
///
/// Tracks a maximum of 64 different subsections.
pub fn ensure_subsections<F, DTR: BufRead>(
    events: &mut DataTreeReader<DTR>,
    subsections: &[(&str, bool, Range)], // (type name, is leaf, valid count range)
    mut f: F,
) -> PsyResult<()>
where
    F: FnMut(&mut DataTreeReader<DTR>) -> PsyResult<()>,
{
    let mut counts = [0usize; 64];

    // Loop through our events!
    loop {
        match events.peek_event()? {
            Event::Leaf {
                type_name,
                byte_offset,
                ..
            } => {
                if let Some(idx) = subsections
                    .iter()
                    .position(|(n, l, _)| *l == true && n == &type_name)
                {
                    // Increment count and make sure we're within the valid count
                    // range for this sub-sections.
                    counts[idx] += 1;
                    if !subsections[idx].2.contains_upper(counts[idx]) {
                        return Err(PsyError::WrongNodeCount(
                            byte_offset,
                            format!(
                                "Expected at most {} '{}' leaf nodes but found \
                                 at least {}.",
                                subsections[idx].2.upper() - 1,
                                subsections[idx].0,
                                counts[idx],
                            ),
                        ));
                    }

                    // Call handler.
                    f(events)?
                } else {
                    break;
                }
            }
            Event::InnerOpen {
                type_name,
                byte_offset,
                ..
            } => {
                if let Some(idx) = subsections
                    .iter()
                    .position(|(n, l, _)| *l == false && n == &type_name)
                {
                    // Increment count and make sure we're within the valid count
                    // range for this sub-sections.
                    counts[idx] += 1;
                    if !subsections[idx].2.contains_upper(counts[idx]) {
                        return Err(PsyError::WrongNodeCount(
                            byte_offset,
                            format!(
                                "Expected at most {} internal '{}' node(s) but \
                                 found at least {}.",
                                subsections[idx].2.upper() - 1,
                                subsections[idx].0,
                                counts[idx],
                            ),
                        ));
                    }

                    // Call handler.
                    f(events)?
                } else {
                    break;
                }
            }
            Event::InnerClose { .. } => {
                break;
            }
            Event::EOF => {
                break;
            }
        }
    }

    // Validation.
    for i in 0..subsections.len() {
        if !subsections[i].2.contains(counts[i]) {
            if subsections[i].1 {
                return Err(PsyError::WrongNodeCount(
                    events.byte_offset(),
                    format!(
                        "Expected at least {} '{}' leaf node(s) but only found {}.",
                        subsections[i].2.lower(),
                        subsections[i].0,
                        counts[i],
                    ),
                ));
            } else {
                return Err(PsyError::WrongNodeCount(
                    events.byte_offset(),
                    format!(
                        "Expected at least {} internal '{}' node(s) but only found {}.",
                        subsections[i].2.lower(),
                        subsections[i].0,
                        counts[i],
                    ),
                ));
            }
        }
    }

    Ok(())
}

// ========================================================

#[cfg(test)]
mod test {
    use super::*;
    use nom::{combinator::all_consuming, sequence::tuple};

    #[test]
    fn ws_u32_1() {
        assert_eq!(ws_u32("42"), Ok((&""[..], 42)));
        assert_eq!(ws_u32("     42"), Ok((&""[..], 42)));
        assert_eq!(ws_u32("42   "), Ok((&""[..], 42)));
        assert_eq!(ws_u32("     42"), Ok((&""[..], 42)));
        assert_eq!(ws_u32("     42   53"), Ok((&"53"[..], 42)));
    }

    #[test]
    fn ws_usize_1() {
        assert_eq!(ws_usize("42"), Ok((&""[..], 42)));
        assert_eq!(ws_usize("     42"), Ok((&""[..], 42)));
        assert_eq!(ws_usize("42   "), Ok((&""[..], 42)));
        assert_eq!(ws_usize("     42"), Ok((&""[..], 42)));
        assert_eq!(ws_usize("     42   53"), Ok((&"53"[..], 42)));
    }

    #[test]
    fn ws_i32_1() {
        assert_eq!(ws_i32("42"), Ok((&""[..], 42)));
        assert_eq!(ws_i32("     42"), Ok((&""[..], 42)));
        assert_eq!(ws_i32("42   "), Ok((&""[..], 42)));
        assert_eq!(ws_i32("     42"), Ok((&""[..], 42)));
        assert_eq!(ws_i32("     42   53"), Ok((&"53"[..], 42)));
    }

    #[test]
    fn ws_i32_2() {
        assert_eq!(ws_i32("-42"), Ok((&""[..], -42)));
        assert_eq!(ws_i32("     -42"), Ok((&""[..], -42)));
        assert_eq!(ws_i32("-42   "), Ok((&""[..], -42)));
        assert_eq!(ws_i32("     -42"), Ok((&""[..], -42)));
        assert_eq!(ws_i32("     -42   53"), Ok((&"53"[..], -42)));
        assert_eq!(ws_i32("--42").is_err(), true);
    }

    #[test]
    fn ws_f32_1() {
        assert_eq!(ws_f32("42"), Ok((&""[..], 42.0)));
        assert_eq!(ws_f32("     42"), Ok((&""[..], 42.0)));
        assert_eq!(ws_f32("42   "), Ok((&""[..], 42.0)));
        assert_eq!(ws_f32("     42"), Ok((&""[..], 42.0)));
        assert_eq!(ws_f32("     42   53"), Ok((&"53"[..], 42.0)));
    }

    #[test]
    fn ws_f32_2() {
        assert_eq!(ws_f32("42.5"), Ok((&""[..], 42.5)));
        assert_eq!(ws_f32("     42.5"), Ok((&""[..], 42.5)));
        assert_eq!(ws_f32("42.5   "), Ok((&""[..], 42.5)));
        assert_eq!(ws_f32("     42.5"), Ok((&""[..], 42.5)));
        assert_eq!(ws_f32("     42.5   53"), Ok((&"53"[..], 42.5)));
    }

    #[test]
    fn ws_f32_3() {
        assert_eq!(ws_f32("-42.5"), Ok((&""[..], -42.5)));
        assert_eq!(ws_f32("     -42.5"), Ok((&""[..], -42.5)));
        assert_eq!(ws_f32("-42.5   "), Ok((&""[..], -42.5)));
        assert_eq!(ws_f32("     -42.5"), Ok((&""[..], -42.5)));
        assert_eq!(ws_f32("     -42.5   53"), Ok((&"53"[..], -42.5)));
    }

    #[test]
    fn ws_f32_4() {
        assert_eq!(ws_f32("a1.0").is_err(), true);
        assert_eq!(all_consuming(ws_f32)("0abc").is_err(), true);
        assert_eq!(tuple((ws_f32, ws_f32))("0.abc 1.2").is_err(), true);
    }
}
