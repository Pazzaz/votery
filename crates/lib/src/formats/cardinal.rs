use std::{
    fmt::{self, Display},
    io::BufRead,
};

use rand::distributions::{Distribution, Uniform};

use super::{remove_newline, toi::TiedOrdersIncomplete, VoteFormat, Binary};
use crate::pairwise_lt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cardinal {
    pub(crate) votes: Vec<usize>,
    pub(crate) candidates: usize,
    pub(crate) voters: usize,
    pub min: usize,
    pub max: usize,
}

impl Cardinal {
    pub fn new(candidates: usize, min: usize, max: usize) -> Cardinal {
        debug_assert!(min <= max);
        Cardinal { votes: Vec::new(), candidates, voters: 0, min, max }
    }

    pub(crate) fn valid(&self) -> bool {
        if self.candidates == 0 && (self.voters != 0 || !self.votes.is_empty())
            || self.votes.len() != self.voters * self.candidates
        {
            return false;
        }
        for i in 0..self.voters {
            for j in 0..self.candidates {
                let v = self.votes[self.candidates * i + j];
                if v < self.min || v > self.max {
                    return false;
                }
            }
        }
        true
    }

    /// Multiply each vote score with constant `a`, changing the `min` and `max`
    /// score.
    pub fn mul(&mut self, a: usize) {
        if a == 1 {
            return;
        }
        let new_min = self.min.checked_mul(a).unwrap();
        let new_max = self.max.checked_mul(a).unwrap();
        for i in 0..self.voters {
            for j in 0..self.candidates {
                self.votes[i * self.candidates + j] *= a;
            }
        }
        self.min = new_min;
        self.max = new_max;
        debug_assert!(self.valid());
    }

    /// Add to each vote score a constant `a`, changing the `min` and `max`
    /// score.
    pub fn add(&mut self, a: usize) {
        if a == 0 {
            return;
        }
        let new_min = self.min.checked_add(a).unwrap();
        let new_max = self.max.checked_add(a).unwrap();
        for i in 0..self.voters {
            for j in 0..self.candidates {
                self.votes[i * self.candidates + j] += a;
            }
        }
        self.min = new_min;
        self.max = new_max;
        debug_assert!(self.valid());
    }

    /// Subtracts from each vote score a constant `a`, changing the `min` and
    /// `max` score.
    pub fn sub(&mut self, a: usize) {
        if a == 0 {
            return;
        }
        let new_min = self.min.checked_sub(a).unwrap();
        let new_max = self.max.checked_sub(a).unwrap();
        for i in 0..self.voters {
            for j in 0..self.candidates {
                self.votes[i * self.candidates + j] -= a;
            }
        }
        self.min = new_min;
        self.max = new_max;
        debug_assert!(self.valid());
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.candidates == 0 {
            return Ok(());
        }
        // The smallest each vote can be is all '0' seperated by ','
        let mut buf = String::with_capacity(self.candidates * 2);
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of vote"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            let mut count = 0;
            for s in buf.split(',') {
                count += 1;
                let v: usize = s.parse().or(Err("Vote is not a number"))?;
                if v > self.max {
                    return Err("Cardinal vote is larger than max value");
                } else if v < self.min {
                    return Err("Cardinal vote is smaller than min value");
                }
                self.votes.push(v);
            }
            if count > self.candidates {
                return Err("Too many candidates listed in vote");
            } else if count < self.candidates {
                return Err("Too few candidates listed in vote");
            }
            self.voters += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Number of valid values
    pub fn values(&self) -> usize {
        self.max - self.min + 1
    }

    /// The Kotze-Pereira transformation
    pub fn kp_tranform(&self) -> Result<Binary, &'static str> {
        let mut binary_votes: Vec<bool> = Vec::new();
        let vote_size = self.candidates
            .checked_mul(self.voters)
            .ok_or("Number of votes would be too large")?
            .checked_mul(self.values() - 1)
            .ok_or("Number of votes would be too large")?;
        binary_votes.try_reserve_exact(vote_size).or(Err("Could not allocate"))?;
        for i in 0..self.voters {
            let vote = &self.votes[i*self.candidates..(i+1)*self.candidates];
            for lower in self.min..self.max {
                for &j in vote {
                    binary_votes.push(j > lower);
                }
            }
        }
        let votes = Binary {
            votes: binary_votes,
            candidates: self.candidates,
            voters: self.voters * (self.values() - 1),
        };
        debug_assert!(votes.valid());
        Ok(votes)
    }

    /// Turn every vote into a binary vote, where every value larger or equal to
    /// `n` becomes an approval.
    ///
    /// # Panics
    /// Will panic if n is not contained in `self.min..=self.max`.
    pub fn to_binary_cutoff(&self, n: usize) -> Result<Binary, &'static str> {
        debug_assert!(self.min <= n && n <= self.max);
        let mut binary_votes: Vec<bool> = Vec::new();
        binary_votes
            .try_reserve_exact(self.candidates * self.voters)
            .or(Err("Could not allocate"))?;
        binary_votes.extend(self.votes.iter().map(|x| *x >= n));
        let votes =
            Binary { votes: binary_votes, candidates: self.candidates, voters: self.voters };
        debug_assert!(votes.valid());
        Ok(votes)
    }
}

impl Display for Cardinal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.voters {
            for j in 0..(self.candidates - 1) {
                let v = self.votes[i * self.candidates + j];
                write!(f, "{},", v)?;
            }
            let v_last = self.votes[i * self.candidates + (self.candidates - 1)];
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}

impl<'a> VoteFormat<'a> for Cardinal {
    type Vote = &'a [usize];
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

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        unimplemented!();
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
        if self.candidates == 0 || new_voters == 0 {
            return;
        }

        self.votes.reserve(new_voters);
        let dist = Uniform::from(self.min..=self.max);
        for _ in 0..new_voters {
            for _ in 0..self.candidates {
                let i = dist.sample(rng);
                self.votes.push(i);
            }
        }
        self.voters += new_voters;
        debug_assert!(self.valid());
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::formats::tests::std_rng;

    impl Arbitrary for Cardinal {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut voters, mut candidates, mut min, mut max): (usize, usize, usize, usize) =
                Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            voters = voters % g.size();
            candidates = candidates % g.size();
            min = min % g.size();
            max = max % g.size();

            if min > max {
                std::mem::swap(&mut min, &mut max);
            }

            let mut votes = Cardinal::new(candidates, min, max);
            votes.generate_uniform(&mut std_rng(g), voters);
            votes
        }
    }

    #[quickcheck]
    fn kp_tranform_voters(cv: Cardinal) -> bool {
        match cv.kp_tranform() {
            Ok(bv) => bv.voters == cv.voters * (cv.values() - 1),
            Err(_) => true,
        }
    }
}
