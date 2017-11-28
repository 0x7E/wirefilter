#[macro_use]
extern crate nom;

use nom::*;
use std::str::FromStr;
use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanEqual,
    LessThanEqual,
    Contains,
    Matches,
    BitwiseAnd,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IdentifierLike<'a> {
    Identifier(&'a str),
    Operator(Operator),
}

named!(pub parse_unsigned(&str) -> u64, alt!(
    preceded!(tag!("0x"), map_res!(hex_digit, |digits| u64::from_str_radix(digits, 16))) |
    preceded!(tag!("0"), map_res!(oct_digit, |digits| u64::from_str_radix(digits, 8))) |
    map_res!(digit, u64::from_str)
));

named!(pub parse_operator(&str) -> Operator, alt!(
    value!(Operator::Equal, tag!("==")) |
    value!(Operator::NotEqual, tag!("!=")) |
    value!(Operator::GreaterThanEqual, tag!(">=")) |
    value!(Operator::LessThanEqual, tag!("<=")) |
    value!(Operator::GreaterThan, tag!(">")) |
    value!(Operator::LessThan, tag!("<")) |
    value!(Operator::Matches, tag!("~")) |
    value!(Operator::BitwiseAnd, tag!("&"))
));

named!(pub parse_identifier_like(&str) -> IdentifierLike, switch!(alpha,
    "eq" => value!(IdentifierLike::Operator(Operator::Equal)) |
    "ne" => value!(IdentifierLike::Operator(Operator::NotEqual)) |
    "gt" => value!(IdentifierLike::Operator(Operator::GreaterThan)) |
    "lt" => value!(IdentifierLike::Operator(Operator::LessThan)) |
    "ge" => value!(IdentifierLike::Operator(Operator::GreaterThanEqual)) |
    "le" => value!(IdentifierLike::Operator(Operator::LessThanEqual)) |
    "contains" => value!(IdentifierLike::Operator(Operator::Contains)) |
    "matches" => value!(IdentifierLike::Operator(Operator::Matches)) |
    "bitwise_and" => value!(IdentifierLike::Operator(Operator::BitwiseAnd)) |
    other => value!(IdentifierLike::Identifier(other))
));

named!(ethernet_separator(&str) -> char, one_of!(":.-"));

named!(hex_byte(&str) -> u8, map_res!(take!(2), |digits| u8::from_str_radix(digits, 16)));

named!(hex_byte_pair(&str) -> [u8; 2], do_parse!(
    b1: hex_byte >>
    opt!(ethernet_separator) >>
    b2: hex_byte >>
    ([b1, b2])
));

named!(pub parse_ethernet_addr(&str) -> [u8; 6], do_parse!(
    w1: hex_byte_pair >>
    ethernet_separator >>
    w2: hex_byte_pair >>
    ethernet_separator >>
    w3: hex_byte_pair >>
    ([w1[0], w1[1], w2[0], w2[1], w3[0], w3[1]])
));

named!(dec_byte(&str) -> u8, map_res!(digit, u8::from_str));

named!(pub parse_ipv4(&str) -> [u8; 4], do_parse!(
    b1: dec_byte >>
    char!('.') >>
    b2: dec_byte >>
    char!('.') >>
    b3: dec_byte >>
    char!('.') >>
    b4: dec_byte >>
    ([b1, b2, b3, b4])
));

named!(oct_byte(&str) -> u8, map_res!(take!(3), |digits| u8::from_str_radix(digits, 8)));

named!(pub parse_string(&str) -> Cow<str>, do_parse!(
    char!('"') >>
    unprefixed: map!(is_not!("\"\\"), Cow::Borrowed) >>
    res: fold_many0!(preceded!(char!('\\'), tuple!(
        alt!(
            preceded!(char!('x'), map!(hex_byte, |b| b as char)) |
            map!(oct_byte, |b| b as char) |
            anychar
        ),
        map!(opt!(is_not!("\"\\")), Option::unwrap_or_default)
    )), unprefixed, |acc: Cow<str>, (ch, rest)| {
        let mut acc = acc.into_owned();
        acc.push(ch);
        acc.push_str(rest);
        Cow::Owned(acc)
    }) >>
    char!('"') >>
    (res)
));

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_ok {
        ($expr:expr, $out:expr, $rest:expr) => {
            assert_eq!($expr, IResult::Done($rest, $out));
        };
    }

    macro_rules! assert_err {
        (@error $code: expr, $rest: expr) => {
            error_position!($code, $rest)
        };

        (@error $code: expr, $rest: expr, $($tt:tt)+) => {
            error_node_position!($code, $rest, assert_err!(@error $($tt)+))
        };

        ($expr:expr, $($tt:tt)+) => {
            assert_eq!($expr, IResult::Error(assert_err!(@error $($tt)+)));
        };
    }

    macro_rules! assert_incomplete {
        ($expr: expr) => {
            assert_eq!($expr, IResult::Incomplete(Needed::Unknown));
        };

        ($expr: expr, $size: expr) => {
            assert_eq!($expr, IResult::Incomplete(Needed::Size($size)));
        };
    }

    #[test]
    fn test_unsigned() {
        assert_ok!(parse_unsigned("0x1f5+"), 501, "+");
        assert_ok!(parse_unsigned("0123;"), 83, ";");
        assert_ok!(parse_unsigned("78!"), 78, "!");
        assert_ok!(parse_unsigned("0xefg"), 239, "g");
    }

    #[test]
    fn test_operator() {
        assert_ok!(parse_operator("~1"), Operator::Matches, "1");
        assert_ok!(parse_operator(">=2"), Operator::GreaterThanEqual, "2");
        assert_ok!(parse_operator("<2"), Operator::LessThan, "2");
        assert_err!(parse_operator("xyz"), ErrorKind::Alt, "xyz");
    }

    #[test]
    fn test_identifier() {
        assert_ok!(
            parse_identifier_like("xyz1"),
            IdentifierLike::Identifier("xyz"),
            "1"
        );
        assert_ok!(
            parse_identifier_like("containst"),
            IdentifierLike::Identifier("containst"),
            ""
        );
        assert_ok!(
            parse_identifier_like("contains t"),
            IdentifierLike::Operator(Operator::Contains),
            " t"
        );
        assert_err!(
            parse_identifier_like("123"),
            ErrorKind::Switch,
            "123",
            ErrorKind::Alpha,
            "123"
        );
    }

    #[test]
    fn test_ethernet_addr() {
        assert_ok!(
            parse_ethernet_addr("12:34:56:78:90:abc"),
            [0x12, 0x34, 0x56, 0x78, 0x90, 0xab],
            "c"
        );
        assert_ok!(
            parse_ethernet_addr("12.34.56.78.90.abc"),
            [0x12, 0x34, 0x56, 0x78, 0x90, 0xab],
            "c"
        );
        assert_ok!(
            parse_ethernet_addr("12.34:56.78-90abc"),
            [0x12, 0x34, 0x56, 0x78, 0x90, 0xab],
            "c"
        );
        assert_err!(
            parse_ethernet_addr("12:34:56:7g:90:ab"),
            ErrorKind::MapRes,
            "7g:90:ab"
        );
        assert_err!(
            parse_ethernet_addr("12:34f:56:78:90:ab"),
            ErrorKind::OneOf,
            "f:56:78:90:ab"
        );
    }

    #[test]
    fn test_ipv4() {
        assert_ok!(parse_ipv4("12.34.56.78;"), [12, 34, 56, 78], ";");
        assert_err!(parse_ipv4("12.34.56.789"), ErrorKind::MapRes, "789");
    }

    #[test]
    fn test_string() {
        assert_ok!(
            parse_string(r#""hello, world";"#),
            Cow::Borrowed("hello, world"),
            ";"
        );
        assert_ok!(
            parse_string(r#""esca\x0a\ped\042";"#),
            Cow::Owned(String::from("esca\nped\"")),
            ";"
        );
        assert_incomplete!(parse_string("\"hello"), 7);
    }
}
