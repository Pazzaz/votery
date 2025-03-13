use rand::{distributions::Uniform, prelude::Distribution, seq::SliceRandom};

use super::{orders::{Rank, RankRef}, soc::StrictOrdersComplete, VoteFormat};

/// SOI - Strict Orders - Incomplete List
///
/// A packed list of (possibly incomplete) strict orders, with related methods.
#[derive(Clone, Debug)]
pub struct StrictOrdersIncomplete {
    pub(super) votes: Vec<usize>,

    // Length of each vote
    pub(super) vote_len: Vec<usize>,
    pub(crate) candidates: usize,
}

impl StrictOrdersIncomplete {
    pub fn new(candidates: usize) -> Self {
        StrictOrdersIncomplete { votes: Vec::new(), vote_len: Vec::new(), candidates }
    }

    pub fn candidates(&self) -> usize {
        self.candidates
    }

    pub fn voters(&self) -> usize {
        self.vote_len.len()
    }

    /// Return true if it was a valid vote.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut order = Vec::with_capacity(self.candidates);
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
            order.push(i);
        }
        let vote = Rank::new(self.candidates, order);
        self.add(vote.as_ref()).unwrap();
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

    pub fn vote_i(&self, i: usize) -> RankRef {
        let start: usize = self.vote_len[0..i].iter().sum();
        let end = start + self.vote_len[i];
        RankRef::new(self.candidates, &self.votes[start..end])
    }
}

impl<'a> VoteFormat<'a> for StrictOrdersIncomplete {
    type Vote = RankRef<'a>;

    fn candidates(&self) -> usize {
        self.candidates
    }

    fn add(&mut self, v: Self::Vote) -> Result<(), &'static str> {
        debug_assert!(v.candidates == self.candidates);
        self.votes.reserve(v.len());
        let mut seen = vec![false; self.candidates];
        for &i in v.order {
            debug_assert!(i < self.candidates || !seen[i]);
            seen[i] = true;
        }
        self.vote_len.push(v.len());
        self.votes.extend_from_slice(v.order);
        debug_assert!(self.valid());
        Ok(())
    }

    fn remove_candidate(&mut self, target: usize) -> Result<(), &'static str> {
        if self.voters() == 0 { return Ok(()) }
        // where in `votes` will we write
        let mut j_1 = 0;
        // where in `vote_len` are we reading
        let mut i_2 = 0;
        // where in `vote_len` will we write
        let mut j_2 = 0;

        let mut last = 0;
        let mut i_1 = 0;
        while i_1 < self.votes.len() {
            let el = self.votes[i_1];
            if el == target {
                self.vote_len[i_2] -= 1;
            } else if el > target {
                self.votes[j_1] = el - 1;
                j_1 += 1;
            } else {
                self.votes[j_1] = el;
                j_1 += 1;
            }
            i_1 += 1;
            if i_1 == last + self.vote_len[i_2] {
                last += self.vote_len[i_2];
                if self.vote_len[i_2] != 0 {
                    self.vote_len[j_2] = self.vote_len[i_2];
                    j_2 += 1;
                }
                i_2 += 1;
            }
        }
        self.votes.drain(j_1..);
        self.vote_len.drain(i_2..);
        debug_assert!(self.valid());
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
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

    fn to_partial_ranking(self) -> super::toi::TiedOrdersIncomplete {
        todo!()
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
