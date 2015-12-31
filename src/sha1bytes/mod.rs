use std::iter;
use std::mem;
use std::borrow::{Cow, ToOwned};
use std::iter::{Iterator, IntoIterator, ExactSizeIterator};
use std::slice::Chunks;
use crypto::digest::Digest;
use crypto::sha1::Sha1;

#[derive(Debug, PartialEq)]
pub struct SHA1Hash<'a>(Cow<'a, [u8]>);

impl <'a> SHA1Hash<'a> {
    pub fn from_prehashed(raw: &'a [u8]) -> SHA1Hash<'a> {
        SHA1Hash(Cow::Borrowed(raw))
    }

    pub fn from_bytes(bytes: &[u8]) -> SHA1Hash<'static> {
        let mut out: [u8; 20] = unsafe{ mem::uninitialized() };
        let mut hasher = Sha1::new();
        hasher.input(bytes);
        hasher.result(&mut out);
        // TODO: Remove copy here
        SHA1Hash(Cow::Owned(out.to_vec()))
    }
}

// ASSERT: len(input) % 20 == 0
#[derive(Debug, PartialEq)]
pub struct SHA1Hashes<'a>(pub &'a [u8]);

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

