use std::{
    fmt::{self, Display},
    io::BufRead,
};

use rand::{
    Rng,
    distributions::{Bernoulli, Distribution},
};

use super::{Cardinal, VoteFormat, remove_newline, toi::TiedOrdersIncomplete};
use crate::pairwise_lt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binary {
    pub votes: Vec<bool>,
    pub(crate) candidates: usize,
    pub voters: usize,
}

impl Binary {
    pub fn new(candidates: usize) -> Binary {
        Binary { votes: Vec::new(), candidates, voters: 0 }
    }

    pub fn candidates(&self) -> usize {
        self.candidates
    }

    pub(crate) fn valid(&self) -> bool {
        !(self.candidates == 0 && (self.voters != 0 || !self.votes.is_empty())
            || self.votes.len() != self.voters * self.candidates)
    }

    /// Sample and add `new_voters` new votes, where each candidates has a
    /// chance of `p` to be chosen, where 0.0 <= `p` <= 1.0
    pub fn bernoulli<R: Rng>(data: &mut Self, rng: &mut R, new_voters: usize, p: f64) {
        if data.candidates == 0 || new_voters == 0 {
            return;
        }

        data.votes.reserve(new_voters * data.candidates);
        let dist = Bernoulli::new(p).unwrap();
        for _ in 0..new_voters {
            for _ in 0..data.candidates {
                let b: bool = dist.sample(rng);
                data.votes.push(b);
            }
        }
        data.voters += new_voters;
        debug_assert!(data.valid());
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.candidates == 0 {
            return Ok(());
        }

        // Should fit each line, including "\r\n"
        let mut buf = String::with_capacity(self.candidates * 2 + 1);
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of vote"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            let bbuf = buf.as_bytes();
            // Each vote has a vote for each candidate and a comma after every
            // candidate, except for the last candidate.
            // => len = candidate + candidate - 1
            if bbuf.len() == (self.candidates * 2 - 1) {
                for i in 0..self.candidates {
                    match bbuf[i * 2] {
                        b'0' => self.votes.push(false),
                        b'1' => self.votes.push(true),
                        _ => return Err("Invalid vote"),
                    }
                    if i != self.candidates - 1 && bbuf[i * 2 + 1] != b',' {
                        return Err("Invalid vote");
                    }
                }
            } else {
                return Err("Invalid vote");
            }
            self.voters += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Convert each vote to a cardinal vote, with an approval being 1 and
    /// disapproval 0.
    ///
    /// Returns `Err` if it failed to allocate
    pub fn to_cardinal(&self) -> Result<Cardinal, &'static str> {
        let mut votes: Vec<usize> = Vec::new();
        votes.try_reserve_exact(self.candidates * self.voters).or(Err("Could not allocate"))?;
        votes.extend(self.votes.iter().map(|x| if *x { 1 } else { 0 }));
        let v =
            Cardinal { votes, candidates: self.candidates, voters: self.voters, min: 0, max: 1 };
        debug_assert!(v.valid());
        Ok(v)
    }
}

impl Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.voters {
            for j in 0..(self.candidates - 1) {
                let b = self.votes[i * self.candidates + j];
                let v = if b { '1' } else { '0' };
                write!(f, "{},", v)?;
            }
            let b_last = self.votes[i * self.candidates + (self.candidates - 1)];
            let v_last = if b_last { '1' } else { '0' };
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}

impl<'a> VoteFormat<'a> for Binary {
    type Vote = &'a [bool];
    fn candidates(&self) -> usize {
        self.candidates
    }

    fn add(&mut self, v: Self::Vote) -> Result<(), &'static str> {
        if v.len() != self.candidates {
            return Err("Vote must contains all candidates");
        }
        self.votes.try_reserve(self.candidates).or(Err("Could not add vote"))?;
        for c in v {
            self.votes.push(*c);
        }
        self.voters += 1;
        Ok(())
    }

    fn remove_candidate(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_candidates = self.candidates - targets.len();
        for i in 0..self.voters {
            let mut t_i = 0;
            let mut offset = 0;
            for j in 0..self.candidates {
                if targets[t_i] == j {
                    t_i += 1;
                    offset += 1;
                } else {
                    let old_index = i * self.candidates + j;
                    let new_index = i * new_candidates + (j - offset);
                    debug_assert!(new_index <= old_index);
                    self.votes[new_index] = self.votes[old_index];
                }
            }
        }
        self.votes.truncate(self.voters * new_candidates);
        self.candidates = new_candidates;
        debug_assert!(self.valid());
        Ok(())
    }

    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_voters: usize) {
        Binary::bernoulli(self, rng, new_voters, 0.5);
    }

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::formats::tests::std_rng;

    impl Arbitrary for Binary {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut voters, mut candidates): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            voters = voters % g.size();
            candidates = candidates % g.size();

            let mut votes = Binary::new(candidates);
            votes.generate_uniform(&mut std_rng(g), voters);
            debug_assert!(votes.valid());
            votes
        }
    }

    #[quickcheck]
    fn to_cardinal(votes: Binary) -> bool {
        let around: Binary = votes.to_cardinal().unwrap().to_binary_cutoff(1).unwrap();
        around == votes
    }
}
