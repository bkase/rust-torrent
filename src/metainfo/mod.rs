use nom::IResult;
use bencode::{BVal, bdict};
use sha1bytes::SHA1Hashes;

use url::Url;

use std::path::PathBuf;
use std::collections::HashMap;

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
pub enum InfoErr {
    ExtraBytes,
    MissingBytes,
    BencodeParseError,
    WrongBencode,
}

pub fn parse(contents: &[u8]) -> Result<Metainfo, InfoErr> {
    match bdict(contents) {
        IResult::Done(rest, bv) => 
            if rest == &b""[..] {
               metainfo_from_bval(bv).ok_or(InfoErr::WrongBencode)
            } else {
               Result::Err(InfoErr::ExtraBytes)
            },
        IResult::Incomplete(_) => Result::Err(InfoErr::MissingBytes),
        IResult::Error(_) => Result::Err(InfoErr::BencodeParseError),
    }
}

fn metainfo_from_bval<'a>(bv: BVal<'a>) -> Option<Metainfo> {
    bv.as_bdict().and_then(|m| {
        let announce_opt = m.get("announce")
            .and_then(|a| a.as_bstring_str())
            .and_then(|url_str| Url::parse(url_str).ok());
        let info_opt = m.get("info")
            .and_then(|i| i.as_bdict_ref())
            .and_then(|info_dict| info_from_dict(info_dict));
        announce_opt.into_iter().zip(info_opt)
            .map(|(announce, info)| Metainfo{ info: info, announce: announce })
            .next()
    })
}

fn info_from_dict<'a>(dict: &HashMap<&'a str, BVal<'a>>) -> Option<Info<'a>> {
    let piece_length_opt = dict.get("piece_length")
        .and_then(|p| p.as_bint());
    let pieces_opt = dict.get("pieces")
        .and_then(|ps| ps.as_bstring_bytes())
        .and_then(|sha_bytes| shas_from_bytes(sha_bytes));
    let mode_opt = mode_from_dict(dict);

    piece_length_opt.into_iter()
        .zip(pieces_opt.into_iter().zip(mode_opt).next())
        .map(|(piece_length, (pieces, mode))| 
             Info{ piece_length: piece_length, pieces: pieces, mode: mode })
        .next()
}

fn shas_from_bytes<'a>(bytes: &'a [u8]) -> Option<SHA1Hashes<'a>> {
    if bytes.len() % 20 != 0 {
        None
    } else {
        Some(SHA1Hashes(bytes))
    }
}

fn mode_from_dict<'a>(dict: &HashMap<&'a str, BVal<'a>>) -> Option<Mode<'a>> {
    let single_opt = dict.get("length").and_then(|l| l.as_bint())
        .and_then(|length| {
            // if there's length then it's single
            dict.get("name")
                .and_then(|s| s.as_bstring_str())
                .map(|name| Mode::Single{ name: name, length: length })
        });

    match single_opt {
        Some(_) => single_opt,
        None => {
            // otherwise it's multi
            let name_opt = dict.get("name")
                .and_then(|s| s.as_bstring_str());
            let files_opt: Option<Vec<File>> = dict.get("files")
                .and_then(|fs| fs.as_blist())
                .and_then(|file_dicts: &Vec<BVal<'a>>|
                          file_dicts.iter()
                                    .map(|fd_bval| fd_bval.as_bdict_ref())
                                    .map(|file_dict_opt|
                                         file_dict_opt.and_then(|file_dict| {
                              let length_opt = file_dict.get("length")
                                  .and_then(|l| l.as_bint());
                              let path_opt = file_dict.get("path")
                                  .and_then(|p| p.as_blist())
                                  .and_then(|ps| components_to_path(ps));

                              length_opt.into_iter()
                                        .zip(path_opt)
                                        .map(|(length, path)| 
                                             File{ length: length, path: path })
                                        .next()
                                    }))
                                    .collect()
                          );
            name_opt.into_iter()
                .zip(files_opt)
                .map(|(name, files)| Mode::Multi{ name: name, files: files })
                .next()
        },
    }
}

fn components_to_path<'a>(ps: &Vec<BVal<'a>>) -> Option<PathBuf> {
    let maybe_vec: Option<Vec<&'a str>> = ps.into_iter()
        .map(|component| component.as_bstring_str())
        .collect();

    maybe_vec.map(|strs| { 
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
            Result::Ok(m) => println!("Metainfo: {:?}", m),
            Result::Err(e) => panic!("Bad err: {:?}", e),
        }
    }
}
