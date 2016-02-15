#[macro_use] extern crate nom;
extern crate url;
extern crate crypto;
extern crate rotor;
extern crate rotor_stream;
extern crate rotor_http;
extern crate time;
extern crate httparse;

mod bencode;
mod metainfo;
mod sha1bytes;
mod tracker;

use bencode::{BVal, bval};

fn main() {
    println!("Hello world: {:?}", BVal::BString(&b"bval"[..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    use tracker;
    use metainfo;

    #[test]
    fn make_request() {
        let bs = include_bytes!("../sample.mp4.torrent");
        let mi: metainfo::Metainfo = metainfo::parse(bs).unwrap();

        tracker::start_every_interval(mi)
    }
}

