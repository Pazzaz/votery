use rand::seq::SliceRandom;

/// SOC - Strict Orders - Complete List
///
/// A packed list of complete strict orders, with related methods. Each vote is
/// a permutation of the elements
#[derive(Clone, Debug)]
pub struct StrictOrdersComplete {
    pub(crate) votes: Vec<usize>,
    pub(crate) elements: usize,
}

impl StrictOrdersComplete {
    pub fn new(elements: usize) -> Self {
        StrictOrdersComplete { votes: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn add(&mut self, vote: &[usize]) {
        debug_assert!(vote.len() == self.elements);
        self.votes.reserve(self.elements);
        let mut seen = vec![false; self.elements];
        for &i in vote {
            debug_assert!(i < self.elements || !seen[i]);
            seen[i] = true;
            self.votes.push(i);
        }
        debug_assert!(self.valid());
    }

    pub fn voters(&self) -> usize {
        debug_assert!(self.votes.len() % self.elements == 0);
        self.votes.len() / self.elements
    }

    /// Return true if it was a valid vote.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut vote = Vec::with_capacity(self.elements);
        let mut seen = vec![false; self.elements];
        for number in s.split(',') {
            let i: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if i >= self.elements || seen[i] {
                return false;
            }
            seen[i] = true;
            vote.push(i);
        }
        if vote.len() != self.elements {
            return false;
        }
        self.add(&vote);
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        for vote in self {
            let mut seen = vec![false; self.elements];
            for &i in vote {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    pub fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_voters: usize) {
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
        debug_assert!(self.valid());
    }
}

impl<'a> IntoIterator for &'a StrictOrdersComplete {
    type Item = &'a [usize];
    type IntoIter = StrictOrdersCompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StrictOrdersCompleteIterator { orig: self, i: 0 }
    }
}

pub struct StrictOrdersCompleteIterator<'a> {
    orig: &'a StrictOrdersComplete,
    i: usize,
}

impl<'a> Iterator for StrictOrdersCompleteIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.orig.elements;
        let start = self.i * self.orig.elements;
        let vote = &self.orig.votes[start..(start + len)];
        self.i += 1;
        Some(vote)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.voters() - self.i;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for StrictOrdersCompleteIterator<'a> {}
