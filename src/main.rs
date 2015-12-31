#[macro_use] extern crate nom;
extern crate url;
extern crate crypto;

mod bencode;
mod metainfo;
mod sha1bytes;

use bencode::BVal;

fn main() {
    println!("Hello world: {:?}", BVal::BString(&b"bval"[..]))
}

