use std::str;
use std::str::FromStr;
use std::collections::HashMap;
use nom::{IResult, Needed, is_digit, Err};

#[derive(Debug, PartialEq)]
pub enum BVal<'a> {
    BString(&'a [u8]),
    BInt(i64),
    BList(Vec<BVal<'a>>),
    BDict(HashMap<&'a str, BVal<'a>>),
}

impl <'a, 'b: 'a> BVal<'a> {
    pub fn as_bstring_str(&self) -> Option<&'a str> {
        match self {
            &BVal::BString(bs) => str::from_utf8(bs).ok(),
            _ => None,
        }
    }

    pub fn as_bstring_bytes(&self) -> Option<&'a [u8]> {
        match self {
            &BVal::BString(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bint(&self) -> Option<i64> {
        match self {
            &BVal::BInt(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_blist(&self) -> Option<&Vec<BVal<'a>>> {
        match *self {
            BVal::BList(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bdict_ref(&self) -> Option<&HashMap<&'a str, BVal<'a>>> {
        match *self {
            BVal::BDict(ref m) => Some(m),
            _ => None,
        }
    }

    pub fn as_bdict(self) -> Option<HashMap<&'a str, BVal<'a>>> {
        match self {
            BVal::BDict(m) => Some(m),
            _ => None,
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
            Some("hello")
        );

        assert_eq!(
            BVal::BInt(10).as_bstring_str(),
            None
        );
    }
}
