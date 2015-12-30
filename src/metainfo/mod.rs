use nom::IResult;
use bencode;
use bencode::{BVal, bdict};
use sha1bytes::SHA1Hashes;

use url;
use url::Url;

use std::path::PathBuf;
use std::collections::HashMap;
use std::result::Result;

#[derive(Debug, PartialEq)]
pub struct Metainfo<'a> {
    info: Info<'a>,
    announce: Url,
    // TODO: extensions
}

#[derive(Debug, PartialEq)]
pub struct Info<'a> {
    piece_length: i64,
    pieces: SHA1Hashes<'a>,
    mode: Mode<'a>,
}

#[derive(Debug, PartialEq)]
pub enum Mode<'a> {
    // TODO: include md5sum
    Single{ name: &'a str, length: i64 },
    Multi{ name: &'a str, files: Vec<File> },
}

#[derive(Debug, PartialEq)]
pub struct File {
    length: i64,
    path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum InfoError {
    ExtraBytes,
    MissingBytes,
    BencodeParseError,
    MissingKey(&'static str),
    BencodeValError{for_key: &'static str, err: bencode::ReadError},
    BadUrl(url::ParseError),
    HashesNotMultiple20Bytes(usize),
}

static ROOT_KEY: &'static str = "_root_";
static ANNOUNCE_KEY: &'static str = "announce";
static INFO_KEY: &'static str = "info";
static PIECE_LENGTH_KEY: &'static str = "piece length";
static PIECES_KEY: &'static str = "pieces";
static LENGTH_KEY: &'static str = "length";
static NAME_KEY: &'static str = "name";
static FILES_KEY: &'static str = "files";
static PATH_KEY: &'static str = "path";

pub fn parse(contents: &[u8]) -> Result<Metainfo, InfoError> {
    match bdict(contents) {
        IResult::Done(rest, bv) =>
            if rest == &b""[..] {
               metainfo_from_bval(bv)
            } else {
               Err(InfoError::ExtraBytes)
            },
        IResult::Incomplete(_) => Err(InfoError::MissingBytes),
        IResult::Error(_) => Err(InfoError::BencodeParseError),
    }
}

// TODO: Abstract the error checking more!
fn metainfo_from_bval<'a>(bv: BVal<'a>) -> Result<Metainfo, InfoError> {
    bv.as_bdict()
        .or_else(|e| Err(InfoError::BencodeValError{for_key: ROOT_KEY, err: e}))
        .and_then(|m| {
        let announce_opt = m.get(ANNOUNCE_KEY)
            .ok_or(InfoError::MissingKey(ANNOUNCE_KEY))
            .and_then(|a| a.as_bstring_str()
                      .or_else(|e| Err(InfoError::BencodeValError{for_key: ANNOUNCE_KEY, err: e})))
            .and_then(|url_str| Url::parse(url_str)
                      .or_else(|e| Err(InfoError::BadUrl(e))));
        let info_opt = m.get(INFO_KEY)
            .ok_or(InfoError::MissingKey(INFO_KEY))
            .and_then(|i| i.as_bdict_ref()
                      .or_else(|e| Err(InfoError::BencodeValError{for_key: INFO_KEY, err: e})))
            .and_then(|info_dict| info_from_dict(info_dict));

        // TODO: For paralellism, a zip here would be better
        announce_opt.and_then(|announce| {
            info_opt.map(|info| Metainfo{ info: info, announce: announce })
        })
    })
}

fn info_from_dict<'a>(dict: &HashMap<&'a str, BVal<'a>>) -> Result<Info<'a>, InfoError> {
    let piece_length_opt = dict.get(PIECE_LENGTH_KEY)
        .ok_or(InfoError::MissingKey(PIECE_LENGTH_KEY))
        .and_then(|p| p.as_bint()
                       .or_else(|e| Err(InfoError::BencodeValError{for_key: PIECE_LENGTH_KEY, err: e})));
    let pieces_opt = dict.get(PIECES_KEY)
        .ok_or(InfoError::MissingKey(PIECES_KEY))
        .and_then(|ps| ps.as_bstring_bytes()
                         .or_else(|e| Err(InfoError::BencodeValError{for_key: PIECES_KEY, err: e})))
        .and_then(|sha_bytes| shas_from_bytes(sha_bytes));
    let mode_opt = mode_from_dict(dict);

    // TODO: zip here
    piece_length_opt.and_then(|piece_length| {
        pieces_opt.and_then(|pieces| {
            mode_opt.map(|mode| {
              Info{ piece_length: piece_length, pieces: pieces, mode: mode }
            })
        })
    })
}

fn shas_from_bytes<'a>(bytes: &'a [u8]) -> Result<SHA1Hashes<'a>, InfoError> {
    if bytes.len() % 20 != 0 {
        Err(InfoError::HashesNotMultiple20Bytes(bytes.len()))
    } else {
        Ok(SHA1Hashes(bytes))
    }
}

fn mode_from_dict<'a>(dict: &HashMap<&'a str, BVal<'a>>) -> Result<Mode<'a>, InfoError> {
    let single_opt = dict.get(LENGTH_KEY)
        .ok_or(InfoError::MissingKey(LENGTH_KEY))
        .and_then(|l| l.as_bint()
                       .or_else(|e| Err(InfoError::BencodeValError{for_key: LENGTH_KEY, err: e})))
        .and_then(|length| {
            // if there's length then it's single
            dict.get(NAME_KEY)
                .ok_or(InfoError::MissingKey(NAME_KEY))
                .and_then(|s| s.as_bstring_str()
                               .or_else(|e| Err(InfoError::BencodeValError{for_key: NAME_KEY, err: e})))
                .map(|name| Mode::Single{ name: name, length: length })
        });

    match single_opt {
        Ok(_) => single_opt,
        Err(_) => {
            // otherwise it's multi
            let name_opt = dict.get(NAME_KEY)
                .ok_or(InfoError::MissingKey(NAME_KEY))
                .and_then(|s| s.as_bstring_str()
                               .or_else(|e| Err(InfoError::BencodeValError{for_key: NAME_KEY, err: e})));
            let files_opt: Result<Vec<File>, InfoError> = dict.get(FILES_KEY)
                .ok_or(InfoError::MissingKey(FILES_KEY))
                .and_then(|fs| fs.as_blist()
                                 .or_else(|e| Err(InfoError::BencodeValError{for_key: FILES_KEY, err: e})))
                .and_then(|file_dicts: &Vec<BVal<'a>>| {
                    let collected: Result<Vec<File>, InfoError> =
                    file_dicts.iter()
                        .map(|fd_bval|
                             fd_bval.as_bdict_ref()
                             .or_else(|e| Err(InfoError::BencodeValError{for_key: FILES_KEY, err: e})))
                        .map(|file_dict_opt|
                             file_dict_opt.and_then(|file_dict| {
                                 let length_opt = file_dict.get(LENGTH_KEY)
                                 .ok_or(InfoError::MissingKey(LENGTH_KEY))
                                 .and_then(|l| l.as_bint()
                                           .or_else(|e| Err(InfoError::BencodeValError{for_key: LENGTH_KEY, err: e})));
                             let path_opt = file_dict.get(PATH_KEY)
                                 .ok_or(InfoError::MissingKey(PATH_KEY))
                                 .and_then(|p| p.as_blist()
                                           .or_else(|e| Err(InfoError::BencodeValError{for_key: PATH_KEY, err: e})))
                                 .and_then(|ps| components_to_path(ps));

                             // TODO: zip here
                             length_opt.and_then(|length| {
                                 path_opt.map(|path| {
                                     File{ length: length, path: path }
                                 })
                             })
                        }))
                        .collect();

                    collected
                });
            // TODO: zip here
            name_opt.and_then(|name| {
                files_opt.map(|files| {
                    Mode::Multi{ name: name, files: files }
                })
            })
        },
    }
}

fn components_to_path<'a>(ps: &Vec<BVal<'a>>) -> Result<PathBuf, InfoError> {
    let maybe_strs: Result<Vec<&'a str>, InfoError> =
        ps.iter()
          .map(|component| component.as_bstring_str()
                                    .or_else(|e| Err(InfoError::BencodeValError{for_key: PATH_KEY, err: e})))
          .collect();

    maybe_strs.map(|strs| {
        strs.iter()
            .fold(PathBuf::new(), |mut acc, item| { acc.push(*item); acc })
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_torrent() {
        let bs = include_bytes!("../../nixos-sample.torrent");
        match parse(bs) {
            Ok(m) => panic!("Metainfo: {:?}", m),
            Err(e) => panic!("Bad err: {:?}", e),
        }
    }
}
