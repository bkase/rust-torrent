use std::iter;
use std::fmt;
use std::borrow::{Cow, Borrow};
use std::iter::{Iterator, IntoIterator};
use std::slice::Chunks;
use crypto::digest::Digest;
use crypto::sha1::Sha1;

#[derive(PartialEq)]
pub struct SHA1Hash<'a>(Cow<'a, [u8]>);

impl <'a> fmt::Debug for SHA1Hash<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SHA1Hash({})", self.to_hex_string())
    }
}

impl <'a> SHA1Hash<'a> {
    pub fn from_prehashed(raw: &'a [u8]) -> SHA1Hash<'a> {
        SHA1Hash(Cow::Borrowed(raw))
    }

    pub fn from_bytes(bytes: &[u8]) -> SHA1Hash<'static> {
        // TODO: Can we stack-allocate without zeroing?
        let mut out: Vec<u8> = vec![0; 20];
        let mut hasher = Sha1::new();
        hasher.input(bytes);
        hasher.result(&mut out);
        SHA1Hash(Cow::Owned(out))
    }

    // from http://illegalargumentexception.blogspot.com/2015/05/rust-byte-array-to-hex-string.html
    fn to_hex_string(&self) -> String {
        let &SHA1Hash(ref view) = self;
        let borrowed: &[u8] = view.borrow();
        let strs: Vec<String> = borrowed
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        strs.join(" ")
    }
}

// ASSERT: len(input) % 20 == 0
#[derive(PartialEq)]
pub struct SHA1Hashes<'a>(pub &'a [u8]);

impl <'a> fmt::Debug for SHA1Hashes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let strs: Vec<String> = self.iter()
            .map(|h| format!("{:?}", h))
            .collect();

        write!(f, "SHA1Hashes[{}]", strs.join(",\n"))
    }
}

impl <'a> SHA1Hashes<'a> {
    pub fn iter(&self) -> iter::Map<Chunks<'a, u8>, fn(&'a [u8]) -> SHA1Hash<'a>> {
        let &SHA1Hashes(view) = self;
        view.chunks(20).map(SHA1Hash::from_prehashed)
    }
}

impl <'a> IntoIterator for SHA1Hashes<'a> {
    type Item = SHA1Hash<'a>;
    type IntoIter = iter::Map<Chunks<'a, u8>, fn(&'a [u8]) -> SHA1Hash<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        let SHA1Hashes(view) = self;
        view.chunks(20).map(SHA1Hash::from_prehashed)
    }
}

