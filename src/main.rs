#[macro_use] extern crate nom;
extern crate url;

mod bencode;
mod metainfo;

use bencode::BVal;

fn main() {
    println!("Hello world: {:?}", BVal::BString("bval"))
}

