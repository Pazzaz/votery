use std::{fmt, fmt::Display, io::BufRead};

use rand::{
    distributions::{Distribution, Uniform},
    Rng,
};

use super::{remove_newline, toi::TiedOrdersIncomplete, VoteFormat};
use crate::pairwise_lt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Specific {
    // number of voters = votes.len()
    pub(crate) votes: Vec<usize>,
    pub(crate) candidates: usize,
}

impl Specific {
    pub fn new(candidates: usize) -> Self {
        Specific { votes: Vec::new(), candidates }
    }

    pub fn majority(&self) -> Option<usize> {
        if self.candidates == 1 {
            return Some(0);
        }
        let mut score = vec![0; self.candidates];
        for i in &self.votes {
            score[*i] += 1;
        }
        (0..self.candidates).find(|&i| score[i] > (self.votes.len() / 2))
    }

    // Checks if all invariants of the format are valid, used in debug_asserts and
    // tests
    fn valid(&self) -> bool {
        if self.candidates == 0 && !self.votes.is_empty() {
            return false;
        }

        for v in &self.votes {
            if *v >= self.candidates {
                return false;
            }
        }
        true
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.candidates == 0 {
            return Ok(());
        }

        // Now we start parsing the actual votes, consisting of a
        // number < candidates. We don't use `std::io::Lines`, because we want to
        // reuse `buf` for performance reasons.
        let mut buf = String::with_capacity(20);
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of vote"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            let vote: usize = buf.parse().or(Err("Vote is not a number"))?;
            if vote >= self.candidates {
                return Err("Vote assigned to non-existing candidate");
            }
            self.votes.push(vote);
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Set the number of candidates to a larger amount
    pub fn set_candidates(&mut self, candidates: usize) {
        debug_assert!(self.candidates <= candidates);
        self.candidates = candidates;
    }
}

impl Display for Specific {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for v in &self.votes {
            writeln!(f, "{}", v)?;
        }
        Ok(())
    }
}

impl<'a> VoteFormat<'a> for Specific {
    type Vote = usize;
    fn candidates(&self) -> usize {
        self.candidates
    }

    fn add(&mut self, v: Self::Vote) -> Result<(), &'static str> {
        // TODO: check
        self.votes.try_reserve(1).or(Err("Could not add vote"))?;
        self.votes.push(v);
        Ok(())
    }

    fn remove_candidate(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_candidates = self.candidates - targets.len();
        let mut j = 0;
        for i in 0..self.votes.len() {
            let v = self.votes[i];
            if let Err(offset) = targets.binary_search(&v) {
                self.votes[j] = v - offset;
                j += 1;
            }
        }
        self.votes.truncate(j);
        self.candidates = new_candidates;
        debug_assert!(self.valid());
        Ok(())
    }

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        let n = self.votes.len();
        TiedOrdersIncomplete {
            votes: self.votes,
            ties: Vec::new(),
            vote_len: vec![1; n],
            candidates: self.candidates,
        }
    }

    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_voters: usize) {
        if self.candidates == 0 || new_voters == 0 {
            return;
        }

        self.votes.reserve(new_voters);
        let dist = Uniform::from(0..self.candidates);
        for _ in 0..new_voters {
            let i = dist.sample(rng);
            self.votes.push(i);
        }
        debug_assert!(self.valid());
    }
}

impl<'a> FromIterator<usize> for Specific {
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        let ii = iter.into_iter();
        let (min_len, _) = ii.size_hint();
        let mut votes = Vec::with_capacity(min_len);
        let mut max = 0;
        for v in ii {
            votes.push(v);
            if v > max {
                max = v;
            }
        }
        Specific { votes, candidates: max + 1 }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::formats::tests::std_rng;

    impl Arbitrary for Specific {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut voters, mut candidates): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            voters = voters % g.size();
            candidates = candidates % g.size();

            let mut votes = Specific::new(candidates);
            votes.generate_uniform(&mut std_rng(g), voters);
            debug_assert!(votes.valid());
            votes
        }

        // We shrink both the number of candidates, and the votes.
        // fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        //     let c = self.candidates;
        //     let candidates: Vec<usize> = (0..c).collect();
        //     Box::new(self.votes.shrink().zip(candidates.shrink()).map(
        //         move |(shrink_votes, shrink_candidates)| {
        //             let mut new_votes = Specific { votes: shrink_votes, candidates: c
        // };             
        // new_votes.remove_candidates(&shrink_candidates).unwrap();            
        // debug_assert!(new_votes.valid());             new_votes
        //         },
        //     ))
        // }
    }

    #[quickcheck]
    fn majority_bound(votes: Specific) -> bool {
        let major = votes.majority();
        eprintln!("{:?}", major);
        match major {
            Some(i) => i < votes.candidates,
            None => true,
        }
    }

    #[quickcheck]
    fn majority_partial(votes: Specific) -> bool {
        let normal_majority = votes.majority();
        let partial_majority = votes.to_partial_ranking().majority();
        match (normal_majority, &partial_majority[..]) {
            (Some(i), [j]) => i == *j,
            (None, []) => true,
            (_, _) => false,
        }
    }

    #[quickcheck]
    fn to_partial_ranking(votes: Specific) -> bool {
        votes.to_partial_ranking().valid()
    }
}
