//! Some basic nom parsers
#![allow(dead_code)]

use std::str;

use nom::{
    call, complete, delimited, digit, do_parse, error_position, multispace, named, one_of, opt,
    tag, tuple_parser,
    IResult::{self, *},
    Needed,
};

// Consumes any whitespace, including zero whitespace
named!(any_space<Option<&[u8]>>, opt!(complete!(multispace)));

// Parsers for numbers surrounded by whitespace
named!(pub ws_u32<u32>, delimited!(any_space, u32_utf8, any_space));
named!(pub ws_u64<u64>, delimited!(any_space, u64_utf8, any_space));
named!(pub ws_usize<usize>, delimited!(any_space, usize_utf8, any_space));
named!(pub ws_i32<i32>, delimited!(any_space, i32_utf8, any_space));
named!(pub ws_i64<i64>, delimited!(any_space, i64_utf8, any_space));
named!(pub ws_isize<isize>, delimited!(any_space, isize_utf8, any_space));
named!(pub ws_f32<f32>, delimited!(any_space, f32_utf8, any_space));
named!(pub ws_f64<f64>, delimited!(any_space, f64_utf8, any_space));

// ========================================================

named!(pub u32_utf8<u32>,
    do_parse!(
        bytes: digit >>
        (str::from_utf8(bytes).unwrap().parse::<u32>().unwrap())
    )
);

named!(pub i32_utf8<i32>,
    do_parse!(
        sign: opt!(one_of!("-+")) >>
        bytes: digit >>
        ({
            match sign {
                Some(s) if s == '-' => -str::from_utf8(bytes).unwrap().parse::<i32>().unwrap(),
                _ => str::from_utf8(bytes).unwrap().parse::<i32>().unwrap(),
            }
        })
    )
);

named!(pub u64_utf8<u64>,
    do_parse!(
        bytes: digit >>
        (str::from_utf8(bytes).unwrap().parse::<u64>().unwrap())
    )
);

named!(pub i64_utf8<i64>,
    do_parse!(
        sign: opt!(one_of!("-+")) >>
        bytes: digit >>
        ({
            match sign {
                Some(s) if s == '-' => -str::from_utf8(bytes).unwrap().parse::<i64>().unwrap(),
                _ => str::from_utf8(bytes).unwrap().parse::<i64>().unwrap(),
            }
        })
    )
);

named!(pub usize_utf8<usize>,
    do_parse!(
        bytes: digit >>
        (str::from_utf8(bytes).unwrap().parse::<usize>().unwrap())
    )
);

named!(pub isize_utf8<isize>,
    do_parse!(
        sign: opt!(one_of!("-+")) >>
        bytes: digit >>
        ({
            match sign {
                Some(s) if s == '-' => -str::from_utf8(bytes).unwrap().parse::<isize>().unwrap(),
                _ => str::from_utf8(bytes).unwrap().parse::<isize>().unwrap(),
            }
        })
    )
);

named!(pub f32_utf8<f32>,
    do_parse!(
        bytes: take_decimal_real >>
        (str::from_utf8(bytes).unwrap().parse::<f32>().unwrap())
    )
);

named!(pub f64_utf8<f64>,
    do_parse!(
        bytes: take_decimal_real >>
        (str::from_utf8(bytes).unwrap().parse::<f64>().unwrap())
    )
);

fn take_decimal_integer(i: &[u8]) -> IResult<&[u8], &[u8]> {
    named!(rr<&[u8], ()>,
        do_parse!(
            opt!(one_of!("-+")) >>
            digit >>
            ()
        )
    );

    match rr(i) {
        Done(remaining, _) => {
            let m = i.len() - remaining.len();
            if m == 0 {
                Incomplete(Needed::Unknown)
            } else {
                Done(&i[m..], &i[0..m])
            }
        }

        Error(e) => Error(e),

        Incomplete(e) => Incomplete(e),
    }
}

fn take_decimal_real(i: &[u8]) -> IResult<&[u8], &[u8]> {
    named!(rr<&[u8], ()>,
        do_parse!(
            opt!(one_of!("-+")) >>
            digit >>
            opt!(complete!(do_parse!(
                tag!(".") >>
                digit >>
                ()
            ))) >>
            ()
        )
    );

    match rr(i) {
        Done(remaining, _) => {
            let m = i.len() - remaining.len();
            if m == 0 {
                Incomplete(Needed::Unknown)
            } else {
                Done(&i[m..], &i[0..m])
            }
        }

        Error(e) => Error(e),

        Incomplete(e) => Incomplete(e),
    }
}

// ========================================================

#[cfg(test)]
mod test {
    use super::take_decimal_real;
    use super::*;

    #[test]
    fn ws_u32_1() {
        assert_eq!(ws_u32(b"42"), Done(&b""[..], 42));
        assert_eq!(ws_u32(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_u32(b"42   "), Done(&b""[..], 42));
        assert_eq!(ws_u32(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_u32(b"     42   53"), Done(&b"53"[..], 42));
    }

    #[test]
    fn ws_i32_1() {
        assert_eq!(ws_i32(b"42"), Done(&b""[..], 42));
        assert_eq!(ws_i32(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_i32(b"42   "), Done(&b""[..], 42));
        assert_eq!(ws_i32(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_i32(b"     42   53"), Done(&b"53"[..], 42));
    }

    #[test]
    fn ws_i32_2() {
        assert_eq!(ws_i32(b"-42"), Done(&b""[..], -42));
        assert_eq!(ws_i32(b"     -42"), Done(&b""[..], -42));
        assert_eq!(ws_i32(b"-42   "), Done(&b""[..], -42));
        assert_eq!(ws_i32(b"     -42   "), Done(&b""[..], -42));
        assert_eq!(ws_i32(b"     -42   53"), Done(&b"53"[..], -42));
    }

    #[test]
    fn ws_u64_1() {
        assert_eq!(ws_u64(b"42"), Done(&b""[..], 42));
        assert_eq!(ws_u64(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_u64(b"42   "), Done(&b""[..], 42));
        assert_eq!(ws_u64(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_u64(b"     42   53"), Done(&b"53"[..], 42));
    }

    #[test]
    fn ws_i64_1() {
        assert_eq!(ws_i64(b"42"), Done(&b""[..], 42));
        assert_eq!(ws_i64(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_i64(b"42   "), Done(&b""[..], 42));
        assert_eq!(ws_i64(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_i64(b"     42   53"), Done(&b"53"[..], 42));
    }

    #[test]
    fn ws_i64_2() {
        assert_eq!(ws_i64(b"-42"), Done(&b""[..], -42));
        assert_eq!(ws_i64(b"     -42"), Done(&b""[..], -42));
        assert_eq!(ws_i64(b"-42   "), Done(&b""[..], -42));
        assert_eq!(ws_i64(b"     -42   "), Done(&b""[..], -42));
        assert_eq!(ws_i64(b"     -42   53"), Done(&b"53"[..], -42));
    }

    #[test]
    fn ws_usize_1() {
        assert_eq!(ws_usize(b"42"), Done(&b""[..], 42));
        assert_eq!(ws_usize(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_usize(b"42   "), Done(&b""[..], 42));
        assert_eq!(ws_usize(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_usize(b"     42   53"), Done(&b"53"[..], 42));
    }

    #[test]
    fn ws_isize_1() {
        assert_eq!(ws_isize(b"42"), Done(&b""[..], 42));
        assert_eq!(ws_isize(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_isize(b"42   "), Done(&b""[..], 42));
        assert_eq!(ws_isize(b"     42"), Done(&b""[..], 42));
        assert_eq!(ws_isize(b"     42   53"), Done(&b"53"[..], 42));
    }

    #[test]
    fn ws_isize_2() {
        assert_eq!(ws_isize(b"-42"), Done(&b""[..], -42));
        assert_eq!(ws_isize(b"     -42"), Done(&b""[..], -42));
        assert_eq!(ws_isize(b"-42   "), Done(&b""[..], -42));
        assert_eq!(ws_isize(b"     -42   "), Done(&b""[..], -42));
        assert_eq!(ws_isize(b"     -42   53"), Done(&b"53"[..], -42));
    }

    #[test]
    fn ws_f32_1() {
        assert_eq!(ws_f32(b"42"), Done(&b""[..], 42.0));
        assert_eq!(ws_f32(b"     42"), Done(&b""[..], 42.0));
        assert_eq!(ws_f32(b"42   "), Done(&b""[..], 42.0));
        assert_eq!(ws_f32(b"     42"), Done(&b""[..], 42.0));
        assert_eq!(ws_f32(b"     42   53"), Done(&b"53"[..], 42.0));
    }

    #[test]
    fn ws_f32_2() {
        assert_eq!(ws_f32(b"42.5"), Done(&b""[..], 42.5));
        assert_eq!(ws_f32(b"     42.5"), Done(&b""[..], 42.5));
        assert_eq!(ws_f32(b"42.5   "), Done(&b""[..], 42.5));
        assert_eq!(ws_f32(b"     42.5"), Done(&b""[..], 42.5));
        assert_eq!(ws_f32(b"     42.5   53"), Done(&b"53"[..], 42.5));
    }

    #[test]
    fn ws_f32_3() {
        assert_eq!(ws_f32(b"-42.5"), Done(&b""[..], -42.5));
        assert_eq!(ws_f32(b"     -42.5"), Done(&b""[..], -42.5));
        assert_eq!(ws_f32(b"-42.5   "), Done(&b""[..], -42.5));
        assert_eq!(ws_f32(b"     -42.5"), Done(&b""[..], -42.5));
        assert_eq!(ws_f32(b"     -42.5   53"), Done(&b"53"[..], -42.5));
    }

    #[test]
    fn ws_f64_1() {
        assert_eq!(ws_f64(b"42"), Done(&b""[..], 42.0));
        assert_eq!(ws_f64(b"     42"), Done(&b""[..], 42.0));
        assert_eq!(ws_f64(b"42   "), Done(&b""[..], 42.0));
        assert_eq!(ws_f64(b"     42"), Done(&b""[..], 42.0));
        assert_eq!(ws_f64(b"     42   53"), Done(&b"53"[..], 42.0));
    }

    #[test]
    fn ws_f64_2() {
        assert_eq!(ws_f64(b"42.5"), Done(&b""[..], 42.5));
        assert_eq!(ws_f64(b"     42.5"), Done(&b""[..], 42.5));
        assert_eq!(ws_f64(b"42.5   "), Done(&b""[..], 42.5));
        assert_eq!(ws_f64(b"     42.5"), Done(&b""[..], 42.5));
        assert_eq!(ws_f64(b"     42.5   53"), Done(&b"53"[..], 42.5));
    }

    #[test]
    fn ws_f64_3() {
        assert_eq!(ws_f64(b"-42.5"), Done(&b""[..], -42.5));
        assert_eq!(ws_f64(b"     -42.5"), Done(&b""[..], -42.5));
        assert_eq!(ws_f64(b"-42.5   "), Done(&b""[..], -42.5));
        assert_eq!(ws_f64(b"     -42.5"), Done(&b""[..], -42.5));
        assert_eq!(ws_f64(b"     -42.5   53"), Done(&b"53"[..], -42.5));
    }

    #[test]
    fn take_decimal_real_1() {
        assert_eq!(take_decimal_real(b"-42.3"), Done(&b""[..], &b"-42.3"[..]));
        assert_eq!(take_decimal_real(b"42.3"), Done(&b""[..], &b"42.3"[..]));
        assert_eq!(take_decimal_real(b"-42"), Done(&b""[..], &b"-42"[..]));
        assert_eq!(take_decimal_real(b"+42.3"), Done(&b""[..], &b"+42.3"[..]));
    }

    #[test]
    fn u32_utf8_1() {
        assert_eq!(u32_utf8(b"42"), Done(&b""[..], 42));
        assert_eq!(u32_utf8(b"-42").is_err(), true);
    }

    #[test]
    fn i32_utf8_1() {
        assert_eq!(i32_utf8(b"42"), Done(&b""[..], 42));
        assert_eq!(i32_utf8(b"-42"), Done(&b""[..], -42));
        assert_eq!(i32_utf8(b"+42"), Done(&b""[..], 42));
        assert_eq!(i32_utf8(b"--42").is_err(), true);
        assert_eq!(i32_utf8(b"+-42").is_err(), true);
    }

    #[test]
    fn u64_utf8_1() {
        assert_eq!(u64_utf8(b"42"), Done(&b""[..], 42));
        assert_eq!(u64_utf8(b"-42").is_err(), true);
    }

    #[test]
    fn i64_utf8_1() {
        assert_eq!(i64_utf8(b"42"), Done(&b""[..], 42));
        assert_eq!(i64_utf8(b"-42"), Done(&b""[..], -42));
        assert_eq!(i64_utf8(b"+42"), Done(&b""[..], 42));
        assert_eq!(i64_utf8(b"--42").is_err(), true);
        assert_eq!(i64_utf8(b"+-42").is_err(), true);
    }

    #[test]
    fn f32_utf8_1() {
        assert_eq!(f32_utf8(b"-42.3"), Done(&b""[..], -42.3));
        assert_eq!(f32_utf8(b"+42.3"), Done(&b""[..], 42.3));
        assert_eq!(f32_utf8(b"42.3"), Done(&b""[..], 42.3));
        assert_eq!(f32_utf8(b"42"), Done(&b""[..], 42.0));
    }

    #[test]
    fn f64_utf8_1() {
        assert_eq!(f64_utf8(b"-42.3"), Done(&b""[..], -42.3));
        assert_eq!(f64_utf8(b"+42.3"), Done(&b""[..], 42.3));
        assert_eq!(f64_utf8(b"42.3"), Done(&b""[..], 42.3));
        assert_eq!(f64_utf8(b"42"), Done(&b""[..], 42.0));
    }
}
