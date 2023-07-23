use std::{
    fmt::{self, Display},
    io::BufRead,
};

// TODO: A lot of implementation details are shared between PartialRanking and
// TotalRanking. Should they be combined somehow?
use rand::seq::SliceRandom;

use super::{remove_newline, toi::TiedOrdersIncomplete, VoteFormat};
use crate::{methods::get_order, pairwise_lt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotalRanking {
    // Has size candidates * voters
    pub votes: Vec<usize>,
    pub candidates: usize,
    pub voters: usize,
}

impl TotalRanking {
    pub fn new(candidates: usize) -> Self {
        TotalRanking { votes: Vec::new(), candidates, voters: 0 }
    }

    // Check if a given total ranking is valid, i.e.
    // 1. len(votes) = candidates * voters
    // 2. Every ranking is total
    fn valid(&self) -> bool {
        if self.candidates == 0 && (self.voters != 0 || !self.votes.is_empty())
            || self.votes.len() != self.voters * self.candidates
        {
            return false;
        }

        let mut seen = vec![false; self.candidates];
        for i in 0..self.voters {
            seen.fill(false);
            for j in 0..self.candidates {
                let vote = self.votes[i * self.candidates + j];
                if vote >= self.candidates {
                    return false;
                }
                if seen[vote] {
                    return false;
                }
                seen[vote] = true;
            }
            for j in 0..self.candidates {
                if !seen[j] {
                    return false;
                }
            }
        }
        true
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.candidates == 0 {
            return Ok(());
        }
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
            let mut count = 0;
            for s in buf.split(',') {
                count += 1;
                let v: usize = s.parse().or(Err("Vote is not a number"))?;
                if v >= self.candidates {
                    return Err(
                        "Ranking of candidate larger than or equal to number of candidates",
                    );
                }
                if seen[v] {
                    return Err("Not a total ranking");
                }
                seen[v] = true;
                self.votes.push(v);
            }
            if count > self.candidates {
                return Err("Too many candidates listed in vote");
            } else if count < self.candidates {
                return Err("Too few candidates listed in vote");
            }
            for i in 0..self.candidates {
                if !seen[i] {
                    return Err("Invalid vote, gap in ranking");
                }
            }
            self.voters += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }
}

impl Display for TotalRanking {
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

impl<'a> VoteFormat<'a> for TotalRanking {
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
            let new_vote = &mut self.votes[(i * new_candidates)..((i + 1) * new_candidates)];

            // TODO: Can we do this in place?
            new_vote.clone_from_slice(&get_order(new_vote, false));
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
}
