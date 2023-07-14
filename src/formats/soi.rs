use rand::{distributions::Uniform, prelude::Distribution, seq::SliceRandom};

use super::soc::StrictOrdersComplete;

/// SOI - Strict Orders - Incomplete List
///
/// A packed list of (possibly incomplete) strict orders, with related methods.
#[derive(Clone, Debug)]
pub struct StrictOrdersIncomplete {
    pub(crate) votes: Vec<usize>,

    // Length of each vote
    pub(crate) vote_len: Vec<usize>,
    pub candidates: usize,
}

impl StrictOrdersIncomplete {
    pub fn new(candidates: usize) -> Self {
        StrictOrdersIncomplete { votes: Vec::new(), vote_len: Vec::new(), candidates }
    }

    pub fn add(&mut self, vote: &[usize]) {
        debug_assert!(vote.len() < self.candidates);
        debug_assert!(0 < vote.len());
        self.votes.reserve(vote.len());
        let mut seen = vec![false; self.candidates];
        for &i in vote {
            debug_assert!(i < self.candidates || !seen[i]);
            seen[i] = true;
            self.votes.push(i);
        }
        self.vote_len.push(vote.len());
        debug_assert!(self.valid());
    }

    pub fn voters(&self) -> usize {
        self.vote_len.len()
    }

    /// Return true if it was a valid vote.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut vote = Vec::with_capacity(self.candidates);
        let mut seen = vec![false; self.candidates];
        for number in s.split(',') {
            let i: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if i >= self.candidates || seen[i] {
                return false;
            }
            seen[i] = true;
            vote.push(i);
        }
        self.add(&vote);
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        let mut seen = vec![false; self.candidates];
        for vote in self {
            seen.fill(false);
            for &i in vote {
                if i >= self.candidates || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    pub fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
        if self.candidates == 0 {
            return;
        }
        let mut v: Vec<usize> = (0..self.candidates).collect();
        self.votes.reserve(self.candidates * new_voters);
        let range = Uniform::from(0..self.candidates);
        for _ in 0..new_voters {
            let candidates = range.sample(rng) + 1;
            v.shuffle(rng);
            for i in 0..candidates {
                self.votes.push(v[i]);
            }
            self.vote_len.push(candidates);
        }
        debug_assert!(self.valid());
    }
}

impl<'a> IntoIterator for &'a StrictOrdersIncomplete {
    type Item = &'a [usize];
    type IntoIter = StrictOrdersIncompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StrictOrdersIncompleteIterator { orig: self, i: 0, start: 0 }
    }
}

pub struct StrictOrdersIncompleteIterator<'a> {
    orig: &'a StrictOrdersIncomplete,
    i: usize,
    start: usize,
}

impl<'a> Iterator for StrictOrdersIncompleteIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.orig.vote_len[self.i];
        let vote = &self.orig.votes[self.start..(self.start + len)];
        self.i += 1;
        self.start += len;
        Some(vote)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.vote_len.len() - self.i;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for StrictOrdersIncompleteIterator<'a> {}

impl From<StrictOrdersComplete> for StrictOrdersIncomplete {
    fn from(value: StrictOrdersComplete) -> Self {
        let voters: usize = value.voters();
        let s = StrictOrdersIncomplete {
            votes: value.votes,
            vote_len: vec![value.candidates; voters],
            candidates: value.candidates,
        };
        debug_assert!(s.valid());
        s
    }
}
