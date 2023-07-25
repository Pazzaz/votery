// A vote without any ties
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vote {
    order: Vec<usize>,
}

impl Vote {
    pub fn new(order: Vec<usize>) -> Self {
        debug_assert!(unique(&order));
        Vote { order }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn parse_vote(s: &str, candidates: usize) -> Option<Self> {
        let mut order: Vec<usize> = Vec::with_capacity(candidates);
        for number in s.split(',') {
            let n: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return None,
            };
            if !(n < candidates) {
                return None;
            }
            order.push(n);
        }

        Some(Vote::new(order))
    }
}

/// A vote with possible ties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TiedVote {
    pub order: Vec<usize>,
    pub tied: Vec<bool>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TiedVoteRef<'a> {
    pub order: &'a [usize],
    pub tied: &'a [bool],
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

    pub fn iter_groups(&'a self) -> GroupIterator<'a> {
        GroupIterator { vote: &self, start: 0 }
    }

    /// Returns group of candidate `c`. 0 is highest rank. Takes `O(n)` time
    pub fn group_of(&self, c: usize) -> Option<usize> {
        self.iter_groups().into_iter().position(|group| group.contains(&c))
    }

    pub fn winners(&self) -> &'a [usize] {
        let i = self.tied.iter().take_while(|x| **x).count();
        &self.order[0..=i]
    }
}

// Splits a vote up into its rankings
pub struct GroupIterator<'a> {
    vote: &'a TiedVoteRef<'a>,
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
                    break;
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



fn unique<T>(l: &[T]) -> bool
where
    T: std::cmp::PartialEq,
{
    for i in 0..l.len() {
        for j in 0..l.len() {
            if i == j {
                break;
            }
            if l[i] == l[j] {
                return false;
            }
        }
    }
    true
}