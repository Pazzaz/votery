use rand::{distributions::Bernoulli, prelude::Distribution, seq::SliceRandom};

use crate::formats::orders::TiedVote;

use super::{
    orders::TiedVoteRef, soc::StrictOrdersComplete, toi::TiedOrdersIncomplete, Cardinal, Specific,
};

/// TOC - Orders with Ties - Complete List
///
/// A packed list of complete orders with ties, with related methods.
#[derive(Clone, Debug)]
pub struct TiedOrdersComplete {
    // Has length voters * candidates
    pub(crate) votes: Vec<usize>,

    // Says if a value is tied with the next value.
    // Has length voters * (candidates - 1)
    pub(crate) ties: Vec<bool>,
    pub candidates: usize,
}

impl TiedOrdersComplete {
    pub fn new(candidates: usize) -> Self {
        TiedOrdersComplete { votes: Vec::new(), ties: Vec::new(), candidates }
    }

    pub fn add(&mut self, v: TiedVoteRef) {
        let vote = v.order;
        let tie = v.tied;
        debug_assert!(vote.len() == self.candidates);
        debug_assert!(0 < vote.len());
        debug_assert!(tie.len() + 1 == vote.len());
        self.votes.reserve(vote.len() * self.candidates);
        self.ties.reserve(tie.len() * (self.candidates - 1));
        let mut seen = vec![false; self.candidates];
        for &i in vote {
            debug_assert!(i < self.candidates || !seen[i]);
            seen[i] = true;
            self.votes.push(i);
        }
        self.ties.extend(tie);
        debug_assert!(self.valid());
    }

    pub fn voters(&self) -> usize {
        debug_assert!(self.votes.len() % self.candidates == 0);
        self.votes.len() / self.candidates
    }

    /// Add a single vote from a string. Return true if it was a valid vote.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut vote: Vec<usize> = Vec::with_capacity(self.candidates);
        let mut tie: Vec<bool> = Vec::with_capacity(self.candidates);
        let mut grouped = false;
        for part in s.split(',') {
            let number: &str = if grouped {
                part.strip_suffix('}').map_or(part, |s| {
                    grouped = !grouped;
                    s
                })
            } else {
                part.strip_prefix('{').map_or(part, |s| {
                    grouped = !grouped;
                    s
                })
            };
            let n: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if !(n < self.candidates) {
                return false;
            }
            vote.push(n);
            tie.push(grouped);
        }
        // The last one will never be tied, so we'll ignore it.
        tie.pop();

        // We didn't end our group or we didn't list all candidates
        if grouped || vote.len() != self.candidates {
            return false;
        }
        self.add(TiedVoteRef::new(&vote, &tie));
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        if self.votes.len() != self.voters() * self.candidates
            || self.ties.len() != self.voters() * (self.candidates - 1)
        {
            return false;
        }
        let mut seen = vec![false; self.candidates];
        for vote in self {
            seen.fill(false);
            if vote.order.len() != self.candidates || vote.tied.len() != self.candidates - 1 {
                return false;
            }
            for &i in vote.order {
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
        self.votes.reserve(new_voters * self.candidates);
        self.ties.reserve(new_voters * (self.candidates - 1));
        let dist = Bernoulli::new(0.5).unwrap();
        for _ in 0..new_voters {
            v.shuffle(rng);
            for i in 0..self.candidates {
                self.votes.push(v[i]);
            }

            for _ in 0..(self.candidates - 1) {
                let b = dist.sample(rng);
                self.ties.push(b);
            }
        }
        debug_assert!(self.valid());
    }

    pub fn to_specific_using<R: rand::Rng>(self, rng: &mut R) -> Specific {
        let candidates = self.candidates;
        let mut votes: Specific =
            self.into_iter().map(|v| *v.winners().choose(rng).unwrap()).collect();

        votes.set_candidates(candidates);
        votes
    }

    /// Convert each vote to a cardinal vote, with the highest rank candidates
    /// receiving a score of `self.candidates`.
    ///
    /// Returns `Err` if it failed to allocate
    pub fn to_cardinal(&self) -> Result<Cardinal, &'static str> {
        let mut votes: Vec<usize> = Vec::new();
        votes.try_reserve_exact(self.candidates * self.voters()).or(Err("Could not allocate"))?;
        let max = self.candidates - 1;
        let mut new_vote = vec![0; self.candidates];
        for vote in self {
            for (i, group) in vote.iter_groups().enumerate() {
                for &c in group {
                    debug_assert!(max >= i);
                    new_vote[c] = max - i;
                }
            }
            // `vote` is a ranking of all candidates, so `new_vote` will be different
            // between iterations.
            votes.extend(&new_vote);
        }
        let v = Cardinal { votes, candidates: self.candidates, voters: self.voters(), min: 0, max };
        debug_assert!(v.valid());
        Ok(v)
    }

    pub fn to_toi(self) -> Result<TiedOrdersIncomplete, &'static str> {
        let mut vote_len = Vec::new();
        vote_len.try_reserve_exact(self.voters()).or(Err("Could not allocate"))?;
        vote_len.resize(self.voters(), self.candidates);
        let v = TiedOrdersIncomplete {
            votes: self.votes,
            ties: self.ties,
            vote_len,
            candidates: self.candidates,
        };
        debug_assert!(v.valid());
        Ok(v)
    }
}

impl<'a> IntoIterator for &'a TiedOrdersComplete {
    type Item = TiedVoteRef<'a>;
    type IntoIter = TiedOrdersCompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TiedOrdersCompleteIterator { orig: self, i: 0 }
    }
}

pub struct TiedOrdersCompleteIterator<'a> {
    orig: &'a TiedOrdersComplete,
    i: usize,
}

impl<'a> Iterator for TiedOrdersCompleteIterator<'a> {
    type Item = TiedVoteRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i == self.orig.voters() {
            return None;
        }
        let len1 = self.orig.candidates;
        let len2 = self.orig.candidates - 1;
        let start1 = self.i * len1;
        let start2 = self.i * len2;
        let vote = &self.orig.votes[start1..(start1 + len1)];
        let tie = &self.orig.ties[start2..(start2 + len2)];
        self.i += 1;
        debug_assert!(tie.len() + 1 == vote.len());

        Some(TiedVoteRef::new(vote, tie))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.voters() - self.i;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for TiedOrdersCompleteIterator<'a> {}

impl From<StrictOrdersComplete> for TiedOrdersComplete {
    fn from(value: StrictOrdersComplete) -> Self {
        let voters: usize = value.voters();
        let s = TiedOrdersComplete {
            votes: value.votes,
            ties: vec![false; (value.candidates - 1) * voters],
            candidates: value.candidates,
        };
        debug_assert!(s.valid());
        s
    }
}
