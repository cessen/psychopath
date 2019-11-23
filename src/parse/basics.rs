//! Some basic nom parsers
#![allow(dead_code)]

use std::str::{self, FromStr};

use nom::{
    character::complete::{digit1, multispace0, one_of},
    combinator::{map_res, opt, recognize},
    number::complete::float,
    sequence::{delimited, tuple},
    IResult,
};

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
