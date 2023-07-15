use rand::{
    distributions::{Bernoulli, Uniform},
    prelude::Distribution,
    seq::SliceRandom,
};

use super::{soi::StrictOrdersIncomplete, toc::TiedOrdersComplete, VoteFormat};

/// TOI - Orders with Ties - Incomplete List
///
/// A packed list of (possibly incomplete) orders with ties, with related
/// methods. One can see it as a `Vec<TiedVote>`, but more efficient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TiedOrdersIncomplete {
    // Has length voters * candidates
    pub(crate) votes: Vec<usize>,

    // Says if a value is tied with the next value.
    // Has length voters * (candidates - 1)
    pub(crate) ties: Vec<bool>,
    pub(crate) vote_len: Vec<usize>,
    pub(crate) candidates: usize,
}

/// A vote with possible ties.
pub struct TiedVote {
    order: Vec<usize>,
    tied: Vec<bool>,
}

impl TiedVote {
    /// A tiedvote is created using
    pub fn new(order: Vec<usize>, tied: Vec<bool>) -> Self {
        debug_assert!(tied.len() + 1 == order.len());
        TiedVote { order, tied }
    }

    pub fn slice(&self) -> TiedVoteRef {
        TiedVoteRef::new(&self.order[..], &self.tied[..])
    }

    pub fn len(&self) -> usize {
        debug_assert!(self.tied.len() + 1 == self.order.len());
        self.order.len()
    }

    pub fn parse_vote(s: &str, candidates: usize) -> Option<Self> {
        let mut order: Vec<usize> = Vec::with_capacity(candidates);
        let mut tied: Vec<bool> = Vec::with_capacity(candidates);
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
                Err(_) => return None,
            };
            if !(n < candidates) {
                return None;
            }
            order.push(n);
            tied.push(grouped);
        }
        // The last one will never be tied, so we'll ignore it.
        tied.pop();

        // We didn't end our group
        if grouped {
            return None;
        }
        Some(TiedVote::new(order, tied))
    }
}

pub struct TiedVoteRef<'a> {
    order: &'a [usize],
    tied: &'a [bool],
}

impl<'a> TiedVoteRef<'a> {
    pub fn new(order: &'a [usize], tied: &'a [bool]) -> Self {
        debug_assert!(tied.len() + 1 == order.len());
        TiedVoteRef { order, tied }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn owned(self) -> TiedVote {
        TiedVote::new(self.order.to_vec(), self.tied.to_vec())
    }

    pub fn iter_groups(self) -> GroupIterator<'a> {
        GroupIterator { vote: self, start: 0 }
    }
}

impl TiedOrdersIncomplete {
    pub fn new(candidates: usize) -> Self {
        TiedOrdersIncomplete {
            votes: Vec::new(),
            ties: Vec::new(),
            vote_len: Vec::new(),
            candidates,
        }
    }

    pub fn voters(&self) -> usize {
        self.vote_len.len()
    }

    /// Add a single vote from a string. Return true if it was a valid vote.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        self.add_from_str_i(s, 1)
    }

    /// Add a vote from a string, `i` times. Return true if it was a valid vote.
    pub fn add_from_str_i(&mut self, s: &str, i: usize) -> bool {
        debug_assert!(i != 0);
        match TiedVote::parse_vote(s, self.candidates) {
            Some(vote) => {
                for _ in 0..i {
                    self.add(vote.slice()).unwrap();
                    debug_assert!(self.valid());
                }
                true
            }
            None => false,
        }
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    pub(crate) fn valid(&self) -> bool {
        let mut votes_len = 0;
        let mut ties_len = 0;
        for &i in &self.vote_len {
            if i == 0 {
                return false;
            }
            votes_len += i;
            ties_len += i - 1;
        }
        if votes_len != self.votes.len() || ties_len != self.ties.len() {
            return false;
        }
        let mut seen = vec![false; self.candidates];
        for vote in self {
            seen.fill(false);
            for &i in vote.order {
                if i >= self.candidates || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    // Increase the number of candidates to `n`. Panics if `n < self.candidates`
    pub fn set_candidates(&mut self, n: usize) {
        debug_assert!(n >= self.candidates);
        self.candidates = n;
    }

    /// If a vote ranks candidate `n`, then add a tie with a new candidate,
    /// as if the new candidate was a clone of `n`.
    pub fn add_clone(&mut self, n: usize) {
        let c = self.candidates;
        let mut res: TiedOrdersIncomplete = self
            .into_iter()
            .map(|vote| {
                let mut order: Vec<usize> = vote.order.to_vec();
                let mut tied: Vec<bool> = vote.tied.to_vec();
                if let Some(i) = order.iter().position(|&x| x == n) {
                    order.insert(i, c);
                    tied.insert(i, true);
                };
                TiedVote::new(order, tied)
            })
            .collect();
        res.candidates = c + 1;
        debug_assert!(self.valid());
        *self = res;
    }

    // Returns all candidates who more than 50% of voters has ranked as their
    // highest alternative. If multiple candidates are tied as their highest
    // alternative, then they all count, so multiple candidates can be the
    // majority.
    pub fn majority(&self) -> Vec<usize> {
        if self.candidates == 1 {
            return vec![0];
        }
        let mut firsts = vec![0; self.candidates];
        for vote in self {
            for &c in vote.iter_groups().next().unwrap() {
                firsts[c] += 1;
            }
        }
        firsts
            .into_iter()
            .enumerate()
            .filter(|(_, score)| *score > self.voters() / 2)
            .map(|(i, _)| i)
            .collect()
    }

    /// Check if a set of candidates is a set of clones such that there does not
    /// exists a candidate outside the set with ranking i, and two candidates in
    /// the set with ranking n and m, where n <= i <= m.
    pub fn is_clone_set(&self, clones: &[usize]) -> bool {
        if clones.len() < 2 {
            return true;
        }
        let mut is_clone = vec![false; self.candidates];
        for &c in clones {
            debug_assert!(c < self.candidates);
            is_clone[c] = true;
        }
        for vote in self {
            let mut seen_n = false;
            let mut seen_i = false;
            for group in vote.iter_groups() {
                // We first check what's in the current group
                let mut has_clone = false;
                let mut has_normal = false;

                // Note that we do not do anything special when all of {n, i, m} are in the same
                // group. We just treat it as if we've encountered n and i.
                for &c in group {
                    if is_clone[c] {
                        has_clone = true;
                    } else {
                        has_normal = true;
                    }
                }
                if seen_i && has_clone || (seen_n && has_clone && has_normal) {
                    // We found "n <= i <= m" in the vote
                    return false;
                }
                if has_clone {
                    seen_n = true;
                }
                if seen_n && has_normal {
                    seen_i = true;
                }
            }
        }
        true
    }
}

impl<'a> VoteFormat<'a> for TiedOrdersIncomplete {
    type Vote = TiedVoteRef<'a>;
    /// List the number of candidates
    fn candidates(&self) -> usize {
        self.candidates
    }

    fn add(&mut self, vote: TiedVoteRef) -> Result<(), &'static str> {
        debug_assert!(vote.len() < self.candidates);
        debug_assert!(0 < vote.len());
        self.votes.reserve(vote.len());
        self.ties.reserve(vote.len() - 1);
        let mut seen = vec![false; self.candidates];
        for &i in vote.order {
            debug_assert!(i < self.candidates || !seen[i]);
            seen[i] = true;
            self.votes.push(i);
        }
        self.ties.extend(vote.tied);
        debug_assert!(self.valid());
        Ok(())
    }

    /// Remove the candidate with index `n`, and shift indices of candidates
    /// with higher index. May remove votes if they only voted for `n`.
    fn remove_candidate(&mut self, n: usize) -> Result<(), &'static str> {
        let mut res: TiedOrdersIncomplete = self
            .into_iter()
            .filter_map(|vote| {
                let mut order: Vec<usize> = Vec::with_capacity(vote.order.len() - 1);
                let mut tied: Vec<bool> = Vec::with_capacity(vote.tied.len().saturating_sub(1));
                for i in 0..order.len() {
                    let mut v = order[i];
                    if v == n {
                        continue;
                    }
                    if v > n {
                        v -= 1;
                    }
                    order.push(v);
                    if i != tied.len() {
                        tied.push(tied[i]);
                    }
                }
                if order.is_empty() {
                    None
                } else {
                    Some(TiedVote::new(order, tied))
                }
            })
            .collect();
        res.candidates -= 1;
        debug_assert!(self.valid());
        *self = res;
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
        if self.candidates == 0 {
            return;
        }
        let mut v: Vec<usize> = (0..self.candidates).collect();
        self.votes.reserve(new_voters * self.candidates);
        self.ties.reserve(new_voters * (self.candidates - 1));
        let dist = Bernoulli::new(0.5).unwrap();
        let range = Uniform::from(0..self.candidates);
        for _ in 0..new_voters {
            let candidates = range.sample(rng) + 1;
            v.shuffle(rng);
            for i in 0..candidates {
                self.votes.push(v[i]);
            }

            for _ in 0..(candidates - 1) {
                let b = dist.sample(rng);
                self.ties.push(b);
            }
            self.vote_len.push(candidates);
        }
        debug_assert!(self.valid());
    }

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        self
    }
}

// Splits a vote up into its rankings
pub struct GroupIterator<'a> {
    vote: TiedVoteRef<'a>,
    start: usize,
}

impl<'a> Iterator for GroupIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.vote.len() {
            return None;
        }
        let mut end = self.start;
        for i in self.start..self.vote.len() {
            if i == self.vote.tied.len() {
                end = i;
            } else {
                if !self.vote.tied[i] {
                    end = i;
                }
            }
        }
        let group = &self.vote.order[self.start..=end];
        self.start = end + 1;
        debug_assert!(group.len() != 0);
        Some(group)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.start == self.vote.order.len() {
            (0, Some(0))
        } else {
            (1, Some(self.vote.order.len() - self.start))
        }
    }
}

/// Will create a new `TiedOrdersIncomplete` from a stream of votes. Will scan
/// for the candidate with the largest index, and use it to set the number of
/// candidates. Use `TiedOrdersIncomplete::set_candidate` to manually set the
/// number of candidates.
impl<'a> FromIterator<TiedVote> for TiedOrdersIncomplete {
    fn from_iter<I: IntoIterator<Item = TiedVote>>(iter: I) -> Self {
        let mut votes: Vec<usize> = Vec::new();
        let mut ties: Vec<bool> = Vec::new();
        let mut vote_len: Vec<usize> = Vec::new();
        let mut max_candidate = 0;
        for vote in iter {
            if vote.order.len() == 0 {
                continue;
            }
            for &i in &vote.order {
                if i > max_candidate {
                    max_candidate = i;
                }
            }
            votes.extend(&vote.order);
            ties.extend(&vote.tied);
            vote_len.push(vote.len());
        }
        TiedOrdersIncomplete { votes, ties, vote_len, candidates: max_candidate + 1 }
    }
}

impl<'a> IntoIterator for &'a TiedOrdersIncomplete {
    type Item = TiedVoteRef<'a>;
    type IntoIter = TiedOrdersIncompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TiedOrdersIncompleteIterator { orig: self, i: 0, start: 0 }
    }
}

pub struct TiedOrdersIncompleteIterator<'a> {
    orig: &'a TiedOrdersIncomplete,
    i: usize,
    start: usize,
}

impl<'a> Iterator for TiedOrdersIncompleteIterator<'a> {
    type Item = TiedVoteRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i == self.orig.vote_len.len() {
            return None;
        }
        let len1 = self.orig.vote_len[self.i];
        let len2 = len1 - 1;
        let start1 = self.start;
        let start2 = start1 - self.i;
        let order = &self.orig.votes[start1..(start1 + len1)];
        let tied = &self.orig.ties[start2..(start2 + len2)];
        self.i += 1;
        self.start += len1;
        Some(TiedVoteRef::new(order, tied))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.voters() - self.i;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for TiedOrdersIncompleteIterator<'a> {}

impl From<StrictOrdersIncomplete> for TiedOrdersIncomplete {
    fn from(value: StrictOrdersIncomplete) -> Self {
        let voters: usize = value.voters();
        let s = TiedOrdersIncomplete {
            votes: value.votes,
            ties: vec![false; voters * (value.candidates - 1)],
            vote_len: value.vote_len,
            candidates: value.candidates,
        };
        debug_assert!(s.valid());
        s
    }
}

impl From<TiedOrdersComplete> for TiedOrdersIncomplete {
    fn from(value: TiedOrdersComplete) -> Self {
        let voters: usize = value.voters();
        let s = TiedOrdersIncomplete {
            votes: value.votes,
            ties: vec![false; voters * (value.candidates - 1)],
            vote_len: vec![value.candidates; voters],
            candidates: value.candidates,
        };
        debug_assert!(s.valid());
        s
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::formats::tests::std_rng;

    impl Arbitrary for TiedOrdersIncomplete {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut voters, mut candidates): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            voters = voters % g.size();
            candidates = candidates % g.size();

            let mut votes = TiedOrdersIncomplete::new(candidates);
            votes.generate_uniform(&mut std_rng(g), voters);
            votes
        }
    }

    #[quickcheck]
    fn clone_remove(votes: TiedOrdersIncomplete, i: usize) -> bool {
        let mut votes = votes.clone();
        let c = votes.candidates;
        if c == 0 {
            return true;
        }
        votes.add_clone(i % c);
        votes.remove_candidate(c).is_ok()
    }
}
