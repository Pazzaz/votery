use rand::{
    Rng,
    distr::{Bernoulli, Uniform},
    seq::{IteratorRandom, SliceRandom},
};

use super::tied_rank_ref::TiedRankRef;
use crate::sort_using;

/// An order with possible ties.
#[derive(Debug, PartialEq, Eq, Default, PartialOrd)]
pub struct TiedRank {
    pub(crate) elements: usize,
    pub order: Vec<usize>,
    pub tied: Vec<bool>,
}

impl Clone for TiedRank {
    fn clone(&self) -> Self {
        Self { elements: self.elements, order: self.order.clone(), tied: self.tied.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.elements = source.elements;
        self.order.clone_from(&source.order);
        self.tied.clone_from(&source.tied);
    }
}

impl<'a> TiedRank {
    pub fn new(elements: usize, order: Vec<usize>, tied: Vec<bool>) -> Self {
        assert!(tied.len() + 1 == order.len() || tied.is_empty() && order.is_empty());
        TiedRank { elements, order, tied }
    }

    pub unsafe fn new_unchecked(elements: usize, order: Vec<usize>, tied: Vec<bool>) -> Self {
        TiedRank { elements, order, tied }
    }

    pub fn new_tied_from_slice(elements: usize, order: &[usize]) -> Self {
        let tie_len = order.len().saturating_sub(1);
        let tied = vec![true; tie_len];
        TiedRank::new(elements, order.to_vec(), tied)
    }

    pub fn as_ref(&'a self) -> TiedRankRef<'a> {
        TiedRankRef::new(self.elements, &self.order[..], &self.tied[..])
    }

    /// Return the number of ranked elements.
    ///
    /// ```
    /// use orders::tied_rank::TiedRank;
    ///
    /// let empty = TiedRank::new_zero();
    /// assert!(empty.len() == 0);
    ///
    /// let full = TiedRank::new_tied(10);
    /// assert!(full.len() == 10);
    /// ```
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Become a copy of `rank`, useful to reuse allocations.
    pub fn copy_from(&mut self, rank: TiedRankRef) {
        self.order.clear();
        self.order.extend_from_slice(rank.order());
        self.tied.clear();
        self.tied.extend_from_slice(rank.tied());
        // TODO: Do we really want to do this?
        self.elements = rank.elements;
    }

    /// Create a new ranking of `elements`, where every element is tied.
    ///
    /// ```
    /// use orders::tied_rank::TiedRank;
    ///
    /// let c = 10;
    /// let rank = TiedRank::new_tied(c);
    /// assert!(rank.as_ref().winners().len() == c);
    /// ```
    pub fn new_tied(elements: usize) -> Self {
        if elements == 0 {
            return TiedRank::new(0, Vec::new(), Vec::new());
        }
        let mut order = Vec::with_capacity(elements);
        for i in 0..elements {
            order.push(i);
        }
        let tied = vec![true; elements - 1];
        TiedRank::new(elements, order, tied)
    }

    pub fn increase_elements(&mut self, elements: usize) {
        debug_assert!(self.elements <= elements);
        self.elements = elements;
    }

    /// Try to parse a ranking of `elements` from `s`. Returns None if `s` is
    /// not a valid ranking.
    ///
    /// ```
    /// use orders::tied_rank::TiedRank;
    ///
    /// let order_str = "2,{0,1},4";
    /// let order = TiedRank::parse_order(5, order_str).expect("Parse failed");
    /// assert_eq!(format!("{}", order.as_ref()), order_str);
    /// ```
    ///
    /// There can be multiple string representations for the same ranking, This
    /// means that `f`, the function from valid string representations of
    /// rankings to actual rankings, is not injective. Example:
    /// ```
    /// use orders::tied_rank::TiedRank;
    ///
    /// let rank = TiedRank::parse_order(5, "0,{1}").unwrap();
    /// assert!(rank.as_ref().to_string() == "0,1");
    /// ```
    pub fn parse_order(elements: usize, s: &str) -> Option<Self> {
        if s.is_empty() {
            let mut rank = TiedRank::new_zero();
            rank.increase_elements(elements);
            return Some(rank);
        }
        let l = (s.len() / 2).min(elements);
        let mut order: Vec<usize> = Vec::with_capacity(l);
        let mut tied: Vec<bool> = Vec::with_capacity(l);
        let mut grouped = false;
        for mut part in s.split(',') {
            // Are we starting a group?
            if !grouped {
                part = part.strip_prefix('{').map_or(part, |s| {
                    grouped = true;
                    s
                });
            }

            // Are we ending a group? We check both cases as this part may be a group with
            // only one element.
            if grouped {
                part = part.strip_suffix('}').map_or(part, |s| {
                    grouped = !grouped;
                    s
                })
            }
            let n: usize = match part.parse() {
                Ok(n) => n,
                Err(_) => return None,
            };
            if n >= elements {
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
        Some(TiedRank::new(elements, order, tied))
    }

    pub fn single(elements: usize, n: usize) -> TiedRank {
        debug_assert!(n < elements);
        let order = vec![n];
        let tied = Vec::new();
        TiedRank::new(elements, order, tied)
    }

    /// Given a score to every element, create a new TiedRank of those
    /// elements. Higher score is better.
    pub fn from_scores(elements: usize, v: &[usize]) -> TiedRank {
        debug_assert!(v.len() == elements);
        let mut list: Vec<(usize, usize)> = v.iter().cloned().enumerate().collect();
        list.sort_by(|(_, a), (_, b)| a.cmp(b).reverse());
        let tied: Vec<bool> = list.windows(2).map(|w| w[0].1 == w[1].1).collect();
        let order: Vec<usize> = list.into_iter().map(|(i, _)| i).collect();
        TiedRank::new(elements, order, tied)
    }

    /// Make the order into a ranking which ranks all `elements`. Use
    /// `tied_last` to decide if the newly added elements should be tied
    /// with the last ranking element in the order.
    pub fn make_complete(&mut self, tied_last: bool) {
        let empty_first = self.is_empty();
        if self.order.len() == self.elements {
            // It's already complete
            return;
        }
        self.order.reserve_exact(self.elements);
        self.tied.reserve_exact(self.elements - 1);
        let seen: &mut [bool] = &mut vec![false; self.elements];
        for &i in &self.order {
            debug_assert!(!seen[i]);
            seen[i] = true;
        }
        for (i, el) in seen.iter().enumerate() {
            if !el {
                self.order.push(i);
            }
        }
        if !empty_first {
            self.tied.push(tied_last);
        }
        self.tied.resize(self.elements - 1, true)
    }

    pub fn from_score(elements: usize, mut order: Vec<usize>, score: &mut [usize]) -> TiedRank {
        let l = order.len();
        debug_assert!(l != 0);
        sort_using(&mut order, score);
        let mut tied = Vec::with_capacity(l - 1);
        for i in 0..(l - 1) {
            tied.push(order[i] == order[i + 1]);
        }
        TiedRank::new(elements, order, tied)
    }

    /// Reverses the ranking in place.
    pub fn reverse(&mut self) {
        self.order.reverse();
        self.tied.reverse();
    }

    /// Remove every element from the ranking which had the highest ranking
    pub fn remove_winners(&mut self) {
        let l = self.order.len();
        if l == 0 {
            return;
        }
        let mut winners = 1;
        for &b in &self.tied {
            if b {
                winners += 1;
            }
        }
        self.order.copy_within(winners..l, 0);
        self.order.truncate(l - winners);
        if l == winners {
            self.tied.clear();
        } else {
            self.tied.copy_within(winners..(l - 1), 0);
            self.tied.truncate(l - 1 - winners);
        }
    }

    /// Remove every element from the ranking which had the lowest ranking
    pub fn remove_last(&mut self) {
        let l = self.order.len();
        if l == 0 {
            return;
        }
        let mut losers = 1;
        for b in self.tied.iter().rev() {
            if *b {
                losers += 1;
            } else {
                break;
            }
        }
        self.order.truncate(l - losers);
        if l == losers {
            self.tied.clear();
        } else {
            self.tied.truncate(l - 1 - losers);
        }
    }

    /// Create a ranking of zero elements
    pub fn new_zero() -> Self {
        TiedRank::new(0, Vec::new(), Vec::new())
    }

    /// Generate a random tied ranking of `elements`.
    pub fn random<R: Rng>(rng: &mut R, elements: usize) -> Self {
        if elements == 0 {
            return TiedRank::new_zero();
        }
        let order_len = rng.sample(Uniform::new(0, elements).unwrap());
        let mut order = (0..elements).choose_multiple(rng, order_len);
        order.shuffle(rng);
        let tied_len = order_len.saturating_sub(1);
        let mut tied = Vec::with_capacity(tied_len);
        let d = Bernoulli::new(0.5).unwrap();
        for _ in 0..tied_len {
            tied.push(rng.sample(d));
        }
        TiedRank::new(elements, order, tied)
    }

    /// Normalize the inner representation of `self`, i.e. sorting the tied
    /// groups.
    ///
    /// ```
    /// use orders::tied_rank::TiedRank;
    ///
    /// let a = TiedRank::parse_order(3, "{0,1,2}").unwrap();
    /// let mut b = TiedRank::parse_order(3, "{2,1,0}").unwrap();
    /// assert!(a != b);
    /// b.normalize();
    /// assert!(a == b);
    /// ```
    pub fn normalize(&mut self) {
        let max = self.len();
        if max < 2 {
            return;
        }
        let mut start = 0;
        while start < max {
            let mut end = start + 1;
            for b in &self.tied[start..] {
                if *b {
                    end += 1;
                } else {
                    break;
                }
            }
            // We sort the group but not `self.tied`, because all of these are tied.
            let group = &mut self.order[start..end];
            group.sort();
            start = end;
        }
    }

    pub fn keep_top(&mut self, n: usize) {
        if n == 0 {
            self.order.clear();
            self.tied.clear();
            return;
        }
        debug_assert!(n <= self.len());
        let mut i = n;
        while i < self.order.len() {
            if self.tied[i - 1] {
                i += 1;
            } else {
                break;
            }
        }
        self.order.truncate(i);
        self.tied.truncate(i - 1);
    }

    /// Return the group which is on the threshold of being top `n`.
    /// If the ties would be broken, then we would have a top `n`.
    /// Will return empty lists if top `n` is already decided.
    pub fn top_n_threshold(&mut self, n: usize) -> (&mut [usize], &mut [bool]) {
        if n == 0 {
            return (&mut [], &mut []);
        }
        let mut i = n;
        while i < self.order.len() {
            if self.tied[i - 1] {
                i += 1;
            } else {
                break;
            }
        }
        (&mut self.order[(n - 1)..i], &mut self.tied[(n - 1)..(i - 1)])
    }

    pub fn random_total<R: Rng>(rng: &mut R, elements: usize, order: &[usize]) -> TiedRank {
        let mut v = order.to_vec();
        v.shuffle(rng);
        let tied_len = v.len().saturating_sub(1);
        let tied = vec![false; tied_len];
        TiedRank::new(elements, v, tied)
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for TiedRank {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let elements = <usize as Arbitrary>::arbitrary(g) % g.size();
            TiedRank::random(&mut std_rng(g), elements)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1))).rev().map(move |i| {
                TiedRank::new(
                    x.elements,
                    x.order[0..i].to_vec(),
                    x.tied[0..(i.saturating_sub(1))].to_vec(),
                )
            });
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn reverse_involution(before: TiedRank) -> bool {
        let mut after = before.clone();
        after.reverse();
        after.reverse();
        before == after
    }

    #[quickcheck]
    fn owned(rank: TiedRank) -> bool {
        rank == rank.as_ref().owned()
    }

    #[test]
    fn iter_groups_zero() {
        let rank = TiedRank::new_zero();
        let first_group = rank.as_ref().iter_groups().next();
        assert!(first_group.is_none());
    }

    #[quickcheck]
    fn iter_groups_len(rank: TiedRank) -> bool {
        let calc_len = rank.as_ref().iter_groups().map(|g| g.len()).sum::<usize>();
        rank.len() == calc_len
    }

    #[quickcheck]
    fn top_len(rank: TiedRank, n: usize) -> bool {
        let values = if rank.len() == 0 { 0 } else { n % rank.len() };
        let l = rank.as_ref().top(values).len();
        values <= l && l <= rank.len()
    }

    #[quickcheck]
    fn make_complete_len(mut rank: TiedRank, tied_last: bool) -> bool {
        rank.make_complete(tied_last);
        rank.len() == rank.elements
    }

    // We have that rank.to_str.to_rank == rank.
    #[quickcheck]
    fn parse_random(rank: TiedRank) -> bool {
        let new_rank_o = TiedRank::parse_order(rank.elements, &format!("{}", rank.as_ref()));
        match new_rank_o {
            Some(new_rank) => rank == new_rank,
            None => false,
        }
    }

    #[test]
    fn top_exact_four() {
        let elements = 5;
        let x = 4;
        let examples = [
            "0,1,2,3,4",
            "{0,1},2,3,4",
            "0,{1,2},3,4",
            "0,1,{2,3},4",
            "{0,1,2},3,4",
            "0,{1,2,3},4",
            "{0,1,2,3},4",
        ];
        for s in examples {
            let rank = TiedRank::parse_order(elements, s).expect("Could not parse");
            assert!(rank.as_ref().top(x).len() == x);
        }
    }

    #[test]
    fn tied_remove_last() {
        let mut r = TiedRank::new_tied(20);
        r.remove_last();
        assert!(r.len() == 0);
    }

    #[quickcheck]
    fn top_idempotent(rank: TiedRank, n: usize) -> bool {
        let values = if rank.len() == 0 { 0 } else { n % rank.len() };
        let first = rank.as_ref().top(values);
        let second = first.top(values);
        first == second
    }

    #[quickcheck]
    fn remove_last_complete(rank: TiedRank) -> bool {
        let mut before = rank.clone();
        before.make_complete(true);
        let mut after = before.clone();
        after.remove_last();
        after.make_complete(false);

        after.normalize();
        before.normalize();

        after == before
    }

    #[quickcheck]
    fn keep_top_n_threshold(mut rank: TiedRank, i: usize) -> bool {
        let n = if rank.len() == 0 { 0 } else { i % rank.len() };
        let (order_group, tied_group) = rank.top_n_threshold(n);
        let o = order_group.to_vec();
        let t = tied_group.to_vec();
        rank.keep_top(n);
        let (new_order_group, new_tied_group) = rank.top_n_threshold(n);

        o == new_order_group && t == new_tied_group
    }

    #[quickcheck]
    fn keep_top_n_len(mut rank: TiedRank, i: usize) -> bool {
        let n = if rank.len() == 0 { 0 } else { i % rank.len() };
        let l1 = rank.len();
        rank.keep_top(n);
        let l2 = rank.len();
        n <= l2 && l2 <= l1
    }

    #[test]
    fn parse_rank_tied_examples() {
        // Arbitrary
        let elements = 10;

        let examples = [
            ("", true),
            ("1", true),
            ("{1}", true),
            ("{0},{1}", true),
            ("{0},{1}", true),
            (",", false),
            (",,", false),
            (",1", false),
            ("1,", false),
            ("{1", false),
            ("1}", false),
            ("{0,1,{2,3}", false),
            ("{{0}}", false),
            ("{0}}", false),
            ("{0},}", false),
            ("{,{0},}", false),
            (" 1", false),
        ];
        for (s, some) in examples {
            let order_o = TiedRank::parse_order(elements, s);
            match (order_o, some) {
                (Some(_), true) | (None, false) => {}
                (None, true) => panic!("`{}` could not be parsed", s),
                (Some(order), false) => panic!("`{}` was parsed to `{}`", s, order.as_ref()),
            }
        }
    }
}
