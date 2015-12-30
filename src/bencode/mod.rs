use std::str;
use std::str::{FromStr, Utf8Error};
use std::result::Result;
use std::collections::HashMap;
use nom::{IResult, Needed, is_digit, Err};

#[derive(Debug, PartialEq)]
pub enum BVal<'a> {
    BString(&'a [u8]),
    BInt(i64),
    BList(Vec<BVal<'a>>),
    BDict(HashMap<&'a str, BVal<'a>>),
}

#[derive(Debug, PartialEq)]
pub enum ReadError {
    WrongType{found: &'static str, expected: &'static str},
    BadString(Utf8Error),
}

pub static BSTRING_TYPE_NAME: &'static str = "BString";
pub static BINT_TYPE_NAME: &'static str = "BInt";
pub static BLIST_TYPE_NAME: &'static str = "BList";
pub static BDICT_TYPE_NAME: &'static str = "BDict";

impl <'a, 'b: 'a> BVal<'a> {
    fn report<T>(&self, expected: &'static str) -> Result<T, ReadError> {
        match self {
            &BVal::BString(_) => Err(ReadError::WrongType{found: BSTRING_TYPE_NAME, expected: expected}),
            &BVal::BInt(_) => Err(ReadError::WrongType{found: BINT_TYPE_NAME, expected: expected}),
            &BVal::BList(_) => Err(ReadError::WrongType{found: BLIST_TYPE_NAME, expected: expected}),
            &BVal::BDict(_) => Err(ReadError::WrongType{found: BDICT_TYPE_NAME, expected: expected}),
        }
    }

    pub fn as_bstring_str(&self) -> Result<&'a str, ReadError> {
        match self {
            &BVal::BString(bs) => str::from_utf8(bs)
                .or_else(|e| Err(ReadError::BadString(e))),
            _ => self.report(BSTRING_TYPE_NAME),
        }
    }

    pub fn as_bstring_bytes(&self) -> Result<&'a [u8], ReadError> {
        match self {
            &BVal::BString(s) => Result::Ok(s),
            _ => self.report(BSTRING_TYPE_NAME),
        }
    }

    pub fn as_bint(&self) -> Result<i64, ReadError> {
        match self {
            &BVal::BInt(i) => Result::Ok(i),
            _ => self.report(BINT_TYPE_NAME),
        }
    }

    pub fn as_blist(&self) -> Result<&Vec<BVal<'a>>, ReadError> {
        match *self {
            BVal::BList(ref v) => Result::Ok(v),
            _ => self.report(BLIST_TYPE_NAME),
        }
    }

    pub fn as_bdict_ref(&self) -> Result<&HashMap<&'a str, BVal<'a>>, ReadError> {
        match *self {
            BVal::BDict(ref m) => Result::Ok(m),
            _ => self.report(BDICT_TYPE_NAME),
        }
    }

    pub fn as_bdict(self) -> Result<HashMap<&'a str, BVal<'a>>, ReadError> {
        match self {
            BVal::BDict(m) => Result::Ok(m),
            _ => self.report(BDICT_TYPE_NAME),
        }
    }
}

/*
 * ===================
 * | BEncode Grammar |
 * ===================
 *
 * bval ::= bstring | bint | blist | bdict
 *
 * bstring ::= posnum ":" <posnum count of ascii bytes>
 *
 * bint ::= "i" num "e"
 *
 * blist ::= "l" bval* "e"
 *
 * bdict ::= "d" keyvalpair* "e"
 *
 * keyvalpair ::= bstring bval
 *
 * posnum ::= <the number in ascii (bounds check for i64)>
 *
 * num ::= "-" posnum | posnum
 */

named!(pub bval<BVal>, alt!(bstring | bint | blist | bdict));

named!(bstring_prelude<usize>,
       chain!(
           n: posnum ~
           tag!(":") ,
           ||{ n }
       )
);

fn bstring_bytes(chars: &[u8]) -> IResult<&[u8], &[u8]> {
    match bstring_prelude(chars) {
        IResult::Done(rest, n) => {
          if rest.len() >= n {
            IResult::Done(&rest[n..], &rest[..n])
          } else {
            IResult::Incomplete(Needed::Size(n))
          }
        },
        IResult::Error(e) => IResult::Error(e),
        IResult::Incomplete(n) => IResult::Incomplete(n),
    }
}

pub fn bstring(chars: &[u8]) -> IResult<&[u8], BVal> {
    match bstring_bytes(chars) {
        IResult::Done(rest, s) => IResult::Done(rest, BVal::BString(s)),
        IResult::Error(e) => IResult::Error(e),
        IResult::Incomplete(n) => IResult::Incomplete(n),
    }
}

named!(pub bint<BVal>,
       chain!(
           tag!("i") ~
           n: num ~
           tag!("e") ,
           ||{ BVal::BInt(n) }
       )
);

named!(pub blist<BVal>,
       chain!(
           tag!("l") ~
           bvs: many0!(bval) ~
           tag!("e") ,
           ||{ BVal::BList(bvs) }
       )
);

named!(pub bdict<BVal>,
       chain!(
           tag!("d") ~
           kvs: many0!(keyvalpair) ~
           tag!("e") ,
           ||{ BVal::BDict(kvs.into_iter().collect()) }
       )
);

named!(keyvalpair<(&str, BVal)>,
       chain!(
           key: map_res!(bstring_bytes, str::from_utf8) ~
           val: bval ,
           ||{ (key, val) }
       )
);

named!(posnum<usize>,
       map_res!(
           map_res!(take_while!(is_digit), str::from_utf8),
           FromStr::from_str
       )
);

named!(num<i64>,
       chain!(
           neg: opt!(tag!("-")) ~
           n: posnum ,
           ||{ neg.map(|_| -1).unwrap_or(1) * (n as i64) }
       )
);

#[cfg(test)]
mod tests {
    use super::*;
    use nom::IResult;

    fn done<'a, T>(t: T) -> IResult<&'a [u8], T> { IResult::Done(&b""[..], t) }

    #[test]
    fn bval_string() {
        assert_eq!(bval(&b"5:abcde"[..]), done(BVal::BString(&b"abcde"[..])));
    }

    #[test]
    fn bval_int() {
        // positive
        assert_eq!(bval(&b"i53e"[..]), done(BVal::BInt(53)));
        // negative
        assert_eq!(bval(&b"i-13e"[..]), done(BVal::BInt(-13)));
    }

    #[test]
    fn bval_list() {
        assert_eq!(bval(&b"li53ei-10ee"[..]), done(
                BVal::BList(
                    vec![BVal::BInt(53), BVal::BInt(-10)]
                ))
        );
    }

    #[test]
    fn bval_dict() {
        assert_eq!(bval(&b"d3:key5:value4:key2i10ee"[..]), done(
                BVal::BDict(
                    vec![
                        ("key", BVal::BString(&b"value"[..])),
                        ("key2", BVal::BInt(10)),
                    ].into_iter().collect()
                ))
        );
    }

    #[test]
    fn option_variant() {
        assert_eq!(
            BVal::BString(&b"hello"[..]).as_bstring_str(),
            Ok("hello")
        );

        assert_eq!(
            BVal::BInt(10).as_bstring_str(),
            Err(ReadError::WrongType{found: BINT_TYPE_NAME, expected: BSTRING_TYPE_NAME})
        );
    }
}
