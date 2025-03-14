use std::{
    fmt::{self, Display},
    io::BufRead,
};

// TODO: A lot of implementation details are shared between PartialRanking and
// TotalRanking. Should they be combined somehow?
use rand::seq::SliceRandom;

use super::{DenseOrders, remove_newline, toi::TiedOrdersIncomplete};
use crate::{get_order, pairwise_lt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotalRanking {
    // Has size elements * voters
    pub votes: Vec<usize>,
    pub(crate) elements: usize,
    pub voters: usize,
}

impl TotalRanking {
    pub fn new(elements: usize) -> Self {
        TotalRanking { votes: Vec::new(), elements, voters: 0 }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    // Check if a given total ranking is valid, i.e.
    // 1. len(votes) = elements * voters
    // 2. Every ranking is total
    fn valid(&self) -> bool {
        if self.elements == 0 && (self.voters != 0 || !self.votes.is_empty())
            || self.votes.len() != self.voters * self.elements
        {
            return false;
        }

        let mut seen = vec![false; self.elements];
        for i in 0..self.voters {
            seen.fill(false);
            for j in 0..self.elements {
                let vote = self.votes[i * self.elements + j];
                if vote >= self.elements {
                    return false;
                }
                if seen[vote] {
                    return false;
                }
                seen[vote] = true;
            }
            for j in 0..self.elements {
                if !seen[j] {
                    return false;
                }
            }
        }
        true
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.elements == 0 {
            return Ok(());
        }
        let mut buf = String::with_capacity(self.elements * 2);

        // Used to find gaps in a ranking
        let mut seen = vec![false; self.elements];
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
                if v >= self.elements {
                    return Err(
                        "Ranking of element larger than or equal to number of elements",
                    );
                }
                if seen[v] {
                    return Err("Not a total ranking");
                }
                seen[v] = true;
                self.votes.push(v);
            }
            if count > self.elements {
                return Err("Too many elements listed in vote");
            } else if count < self.elements {
                return Err("Too few elements listed in vote");
            }
            for i in 0..self.elements {
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
            for j in 0..(self.elements - 1) {
                let v = self.votes[i * self.elements + j];
                write!(f, "{},", v)?;
            }
            let v_last = self.votes[i * self.elements + (self.elements - 1)];
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}

impl<'a> DenseOrders<'a> for TotalRanking {
    type Order = &'a [usize];
    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        if v.len() != self.elements {
            return Err("Vote must contains all elements");
        }
        self.votes.try_reserve(self.elements).or(Err("Could not add vote"))?;
        for c in v {
            self.votes.push(*c);
        }
        self.voters += 1;
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.voters {
            let mut t_i = 0;
            let mut offset = 0;
            for j in 0..self.elements {
                if targets[t_i] == j {
                    t_i += 1;
                    offset += 1;
                } else {
                    let old_index = i * self.elements + j;
                    let new_index = i * new_elements + (j - offset);
                    debug_assert!(new_index <= old_index);
                    self.votes[new_index] = self.votes[old_index];
                }
            }
            let new_vote = &mut self.votes[(i * new_elements)..((i + 1) * new_elements)];

            // TODO: Can we do this in place?
            new_vote.clone_from_slice(&get_order(new_vote, false));
        }
        self.votes.truncate(self.voters * new_elements);
        self.elements = new_elements;
        debug_assert!(self.valid());
        Ok(())
    }

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        unimplemented!();
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
        if self.elements == 0 {
            return;
        }
        let mut v: Vec<usize> = (0..self.elements).collect();
        self.votes.reserve(self.elements * new_voters);
        for _ in 0..new_voters {
            v.shuffle(rng);
            for i in 0..self.elements {
                self.votes.push(v[i]);
            }
        }
        self.voters += new_voters;
        debug_assert!(self.valid());
    }
}
