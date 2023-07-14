use std::{
    fmt::{self, Display},
    io::BufRead,
};

use rand::seq::SliceRandom;

use super::{remove_newline, Cardinal, VoteFormat};
use crate::{methods::get_order, pairwise_gt, pairwise_lt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialRanking {
    // Has size candidates * voters
    pub votes: Vec<usize>,
    pub candidates: usize,
    pub voters: usize,
}

impl PartialRanking {
    pub fn new(candidates: usize) -> Self {
        PartialRanking { votes: Vec::new(), candidates, voters: 0 }
    }

    // Check if a given partial ranking is valid, i.e.
    // 1. len(votes) = candidates * voters
    // 2. Every ranking starts with zero and then has no gaps in it's ranking
    fn valid(&self) -> bool {
        if self.candidates == 0 && (self.voters != 0 || !self.votes.is_empty())
            || self.votes.len() != self.voters * self.candidates
        {
            return false;
        }

        let mut seen = vec![false; self.candidates];
        for v in 0..self.voters {
            seen.fill(false);
            let mut max_seen = 0;
            for i in 0..self.candidates {
                let vote = self.votes[v * self.candidates + i];
                if vote >= self.candidates {
                    return false;
                }
                seen[vote] = true;
                if vote > max_seen {
                    max_seen = vote;
                }
            }
            for i in 0..(max_seen + 1) {
                if !seen[i] {
                    return false;
                }
            }
        }
        true
    }

    // Returns all candidates who more than 50% of voters has ranked as their
    // highest alternative. If multiple candidates are tied as their highest
    // alternative, then they all count. This is why we return a `Vec<usize>`.
    pub fn majority(&self) -> Vec<usize> {
        if self.candidates == 1 {
            return vec![0];
        }
        let mut a = vec![0; self.candidates];
        for i in 0..self.voters {
            for j in 0..self.candidates {
                let k = self.votes[i * self.candidates + j];
                if k == 0 {
                    // This voter ranked j as the highest rank
                    a[j] += 1;
                }
            }
        }
        let mut out = Vec::new();
        for i in 0..self.candidates {
            // Check if it's > 50%
            if a[i] > (self.voters / 2) {
                out.push(i);
            }
        }
        out
    }

    /// Add a new candidate as a clone of another candidate, and make every
    /// voter rank both candidates the same
    pub fn add_clone(&mut self, orig: usize) -> Result<(), &'static str> {
        let c = self.candidates;
        debug_assert!(orig < c);

        // We could create a new vec, but doing it in-place is probably faster. `0` is
        // used as a dummy value
        self.votes.resize(self.votes.len() + self.voters, 0);

        // We go backwards to avoid overwriting old values
        for i in (0..self.voters).rev() {
            // Add the clone
            self.votes[i * (c + 1) + c] = self.votes[i * c + orig];

            // Copy old candidates
            if i != 0 {
                self.votes.copy_within((i * c)..((i + 1) * c), i * (c + 1));
            }
        }
        self.candidates = c + 1;
        debug_assert!(self.valid());
        Ok(())
    }

    /// Check if a set of candidates is a set of clones such that there does not
    /// exists a candidate outside the set with ranking i, and two candidates in
    /// the set with ranking n and m, where n <= i <= m. Returns `false` if
    /// clones.len() < 2.
    pub fn is_clone_set(&self, clones: &Vec<usize>) -> bool {
        debug_assert!(pairwise_lt(&clones));
        let mut rest: Vec<usize> = Vec::with_capacity(self.candidates - clones.len());
        let mut j = 0;
        for i in 0..self.candidates {
            if i == clones[j] {
                j += 1;
            } else {
                rest.push(i);
            }
        }
        // "No voter ranks any candidate outside the set between (or equal to) any
        // candidates that are in the set."
        for i in 0..self.voters {
            let vote = &self.votes[(i * self.candidates)..((i + 1) * self.candidates)];
            for &r in &rest {
                for &n in clones {
                    for &m in clones {
                        if n == m {
                            break;
                        }
                        // TODO: Is this the correct criteria?
                        if vote[n] <= vote[r] && vote[r] <= vote[m]
                            || vote[m] <= vote[r] && vote[r] <= vote[n]
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Convert partial rankings of candidates to cardinal rankings of
    /// candidates. The list of scores should be in sorted order from largest to
    /// smallest score; as many numbers as the number of candidates.
    pub fn to_cardinal(mut self, scores: &[usize]) -> Cardinal {
        debug_assert!(scores.len() == self.candidates);
        debug_assert!(pairwise_gt(scores));
        let max = scores[0];
        let min = *scores.last().unwrap();
        for v in self.votes.iter_mut() {
            *v = scores[*v];
        }
        let c = Cardinal {
            votes: self.votes,
            candidates: self.candidates,
            voters: self.voters,
            min,
            max,
        };
        debug_assert!(c.valid());
        c
    }
}

impl Display for PartialRanking {
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

impl<'a> VoteFormat<'a> for PartialRanking {
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
        debug_assert!(self.valid());
        Ok(())
    }

    fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.candidates == 0 {
            return Ok(());
        }
        // The smallest each vote can be is all '0' seperated by ','
        let mut buf = String::with_capacity(self.candidates * 2);

        // Used to find gaps in a ranking
        let mut seen = vec![false; self.candidates];
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of vote"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            seen.fill(false);
            let mut max_seen = 0;
            let mut count = 0;
            for s in buf.split(',') {
                count += 1;
                let v: usize = s.parse().or(Err("Vote is not a number"))?;
                if v > max_seen {
                    max_seen = v;
                }
                if v >= self.candidates {
                    return Err(
                        "Ranking of candidate larger than or equal to number of candidates",
                    );
                }
                seen[v] = true;
                self.votes.push(v);
            }
            if count > self.candidates {
                return Err("Too many candidates listed in vote");
            } else if count < self.candidates {
                return Err("Too few candidates listed in vote");
            }
            for i in 0..(max_seen + 1) {
                if !seen[i] {
                    return Err("Invalid vote, gap in ranking");
                }
            }
            self.voters += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }

    fn remove_candidates(&mut self, targets: &[usize]) -> Result<(), &'static str> {
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
            let new_vote = &mut self.votes[(i * new_candidates)..((i + 1) * new_candidates)];

            // TODO: Can we do this in place?
            new_vote.clone_from_slice(&get_order(new_vote, false));
        }
        self.votes.truncate(self.voters * new_candidates);
        self.candidates = new_candidates;
        debug_assert!(self.valid());
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
        // TODO: This is copied from `TotalRanking`. It's easy to sample uniformly from
        // total orders, but how does one sample partial orders?
        if self.candidates == 0 {
            return;
        }
        let mut v: Vec<usize> = (0..self.candidates).collect();
        self.votes.reserve(self.candidates * new_voters);
        for _ in 0..new_voters {
            v.shuffle(rng);
            for i in 0..self.candidates {
                self.votes.push(v[i]);
            }
        }
        self.voters += new_voters;
        debug_assert!(self.valid());
    }

    fn to_partial_ranking(self) -> PartialRanking {
        self
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::formats::tests::std_rng;

    impl Arbitrary for PartialRanking {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut voters, mut candidates): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            voters = voters % g.size();
            candidates = candidates % g.size();

            let mut votes = PartialRanking::new(candidates);
            votes.generate_uniform(&mut std_rng(g), voters);
            votes
        }
    }

    #[quickcheck]
    fn clone_remove(votes: PartialRanking, i: usize) -> bool {
        let mut votes = votes.clone();
        let c = votes.candidates;
        if c == 0 {
            return true;
        }
        if let Err(_) = votes.add_clone(i % c) {
            false
        } else if let Err(_) = votes.remove_candidates(&vec![c]) {
            false
        } else {
            true
        }
    }
}
