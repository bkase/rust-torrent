use std::iter::{Iterator, IntoIterator, ExactSizeIterator};

#[derive(Debug, PartialEq)]
pub struct SHA1Hash<'a>(pub &'a [u8]);

// ASSERT: len(input) % 20 == 0
#[derive(Debug, PartialEq)]
pub struct SHA1Hashes<'a>(pub &'a [u8]);

impl <'a> SHA1Hashes<'a> {
    pub fn iter(&self) -> SHA1HashStepper<'a> {
        let &SHA1Hashes(view) = self;
        SHA1HashStepper{ pos: 0, view: view }
    }
}

impl <'a> IntoIterator for SHA1Hashes<'a> {
    type Item = SHA1Hash<'a>;
    type IntoIter = SHA1HashStepper<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let SHA1Hashes(view) = self;
        SHA1HashStepper{ pos: 0, view: view }
    }
}

pub struct SHA1HashStepper<'a> {
    pos: usize,
    view: &'a [u8],
}
impl <'a> Iterator for SHA1HashStepper<'a> {
    type Item = SHA1Hash<'a>;
    
    fn next(&mut self) -> Option<SHA1Hash<'a>> {
        let p = self.pos;
        if self.view.len() <= p*20 {
            None
        } else {
            self.pos += 1;
            Some(SHA1Hash(&self.view[p*20..p*20+20]))
        }
    }
}
impl <'a> ExactSizeIterator for SHA1HashStepper<'a> {
    fn len(&self) -> usize {
        self.view.len() / 20
    }
}

