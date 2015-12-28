use nom::IResult;
use bencode::{BVal, bdict};

use url::Url;

use std::path;

#[derive(Debug, PartialEq)]
struct SHA1Hash(pub [u8; 20]);

#[derive(Debug, PartialEq)]
pub struct Metainfo<'a> {
    info: Info<'a>,
    announce: Url,
    // TODO: extensions
}

#[derive(Debug, PartialEq)]
pub struct Info<'a> {
    piece_length: i64,
    pieces: Vec<SHA1Hash>,
    mode: Mode<'a>,
}

#[derive(Debug, PartialEq)]
pub enum Mode<'a> {
    // TODO: include md5sum
    Single{ name: &'a str, length: i64 },
    Multi{ name: &'a str, files: Vec<File<'a>> },
}

#[derive(Debug, PartialEq)]
pub struct File<'a> {
    length: i64,
    path: &'a path::Path,
}

#[derive(Debug, PartialEq)]
pub enum InfoErr {
    ExtraBytes,
    MissingBytes,
    BencodeParseError,
}

pub fn parse(contents: &[u8]) -> Result<&Metainfo, InfoErr> {
    match bdict(contents) {
        IResult::Done(rest, bv) => 
            if rest == &b""[..] {
               metainfo_from_bval(bv)
            } else {
               Result::Err(InfoErr::ExtraBytes)
            },
        IResult::Incomplete(_) => Result::Err(InfoErr::MissingBytes),
        IResult::Error(_) => Result::Err(InfoErr::BencodeParseError),
    }
}

fn metainfo_from_bval(bv: BVal) -> Result<&Metainfo, InfoErr> {
    Result::Err(InfoErr::ExtraBytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compiles() {
        assert_eq!(parse(&b"d5:abcdei10ee"[..]), Result::Err(InfoErr::ExtraBytes))
    }
}
