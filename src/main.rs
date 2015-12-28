#[macro_use]
extern crate nom;

mod bencode;

use bencode::BVal;

fn main() {
    println!("Hello world: {:?}", BVal::BString("bval"))
}

