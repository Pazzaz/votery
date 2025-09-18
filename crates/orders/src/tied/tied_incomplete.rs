use std::iter::repeat_n;

use rand::{
    Rng,
    distr::{Bernoulli, Uniform},
    seq::{IteratorRandom, SliceRandom},
};

use super::{Tied, tied_incomplete_ref::TiedIRef};
use crate::{add_bool, sort_using};

/// An order with possible ties.
#[derive(Debug, PartialEq, Eq, Default, PartialOrd, serde::Deserialize, serde::Serialize)]
pub struct TiedI {
    pub(crate) elements: usize,
    pub(crate) order: Vec<usize>,
    pub(crate) tied: Vec<bool>,
}

impl Clone for TiedI {
    fn clone(&self) -> Self {
        Self { elements: self.elements, order: self.order.clone(), tied: self.tied.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.elements = source.elements;
        self.order.clone_from(&source.order);
        self.tied.clone_from(&source.tied);
    }
}

impl<'a> TiedI {
    pub fn new(elements: usize, order: Vec<usize>, tied: Vec<bool>) -> Self {
        assert!(tied.len() + 1 == order.len() || tied.is_empty() && order.is_empty());
        TiedI { elements, order, tied }
    }

    pub unsafe fn new_unchecked(elements: usize, order: Vec<usize>, tied: Vec<bool>) -> Self {
        TiedI { elements, order, tied }
    }

    pub fn new_tied_from_slice(elements: usize, order: &[usize]) -> Self {
        let tie_len = order.len().saturating_sub(1);
        let tied = vec![true; tie_len];
        TiedI::new(elements, order.to_vec(), tied)
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    pub fn tied(&self) -> &[bool] {
        &self.tied
    }

    /// Clones from `source` to `self`, similar to [`Clone::clone_from`].
    pub fn clone_from_ref(&mut self, source: TiedIRef) {
        self.order.clear();
        self.order.extend(source.order());
        self.tied.clear();
        self.tied.extend(source.tied());
        self.elements = source.elements;
    }

    /// Create a `TiedI`, from groups of equal elements.
    pub fn from_slices(elements: usize, groups: &[&[usize]]) -> Self {
        let mut orders = Vec::with_capacity(groups.len());
        let mut tied = Vec::with_capacity(groups.len() - 1);
        let mut first = true;
        for group in groups {
            orders.extend_from_slice(group);
            if !first {
                tied.push(false);
            } else {
                first = false;
            }
            tied.extend(repeat_n(true, group.len().saturating_sub(1)));
        }
        TiedI::new(elements, orders, tied)
    }

    pub fn as_ref(&'a self) -> TiedIRef<'a> {
        TiedIRef::new(self.elements, &self.order[..], &self.tied[..])
    }

    /// Return the number of ordered elements.
    ///
    /// ```
    /// use orders::{OrderOwned, tied::{TiedI, Tied}};
    ///
    /// let empty = TiedI::new_zero();
    /// assert_eq!(empty.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn increase_elements(&mut self, elements: usize) {
        assert!(self.elements <= elements);
        self.elements = elements;
    }

    pub fn single(elements: usize, n: usize) -> TiedI {
        debug_assert!(n < elements);
        let order = vec![n];
        let tied = Vec::new();
        TiedI::new(elements, order, tied)
    }

    /// Make the order into a ranking which ranks all `elements`. Use
    /// `tied_last` to decide if the newly added elements should be tied
    /// with the last ranking element in the order.
    #[must_use]
    pub fn make_complete(mut self, tied_last: bool) -> Tied {
        let empty_first = self.is_empty();
        if self.order.len() != self.elements {
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
            self.tied.resize(self.elements - 1, true);
            debug_assert!(self.order.len() == self.elements);
        }
        Tied::new(self.order, self.tied)
    }

    pub fn from_score(elements: usize, mut order: Vec<usize>, score: &mut [usize]) -> TiedI {
        assert!(order.len() == score.len());
        sort_using(&mut order, score);
        let tied = score.windows(2).map(|w| w[0] == w[1]).collect();
        TiedI::new(elements, order, tied)
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
        TiedI::new(0, Vec::new(), Vec::new())
    }

    /// Generate a random tied ranking of `elements`.
    pub fn random<R: Rng>(rng: &mut R, elements: usize) -> Self {
        if elements == 0 {
            return TiedI::new_zero();
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
        TiedI::new(elements, order, tied)
    }

    /// Turn a tied order into a random tied order, reusing allocations.
    pub fn into_random<R: Rng>(&mut self, rng: &mut R, elements: usize) {
        self.order.clear();
        self.tied.clear();
        if elements != 0 {
            let order_len = rng.sample(Uniform::new(0, elements).unwrap());
            for i in 0..elements {
                self.order.push(i);
            }
            // Surely this can be done more effectively
            self.order.shuffle(rng);
            self.order.drain(order_len..);

            let tied_len = order_len.saturating_sub(1);
            if let Some(more_els) = tied_len.checked_sub(self.tied.capacity()) {
                self.tied.reserve(more_els);
            }
            add_bool(rng, &mut self.tied, tied_len);
        }
        self.elements = elements;
    }

    /// Normalize the inner representation of `self`, i.e. sorting the tied
    /// groups.
    ///
    /// ```
    /// use orders::tied::TiedI;
    ///
    /// let a = TiedI::new(3, vec![0, 1, 2], vec![true, true]);
    /// let mut b = TiedI::new(3, vec![2, 1, 0], vec![true, true]);
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

    pub fn remove(mut self, n: usize) -> Self {
        assert!(n < self.elements);
        if self.elements == 1 {
            self.order.clear();
            self.tied.clear();
            self.elements = 0;
            return self;
        }
        let mut skipped = false;
        for i in 0..self.len() {
            if skipped {
                let res = match self.order[i].cmp(&n) {
                    std::cmp::Ordering::Less => self.order[i],
                    std::cmp::Ordering::Equal => {
                        unreachable!();
                    }
                    std::cmp::Ordering::Greater => self.order[i] - 1,
                };
                self.order[i - 1] = res;
            } else {
                let res = match self.order[i].cmp(&n) {
                    std::cmp::Ordering::Less => self.order[i],
                    std::cmp::Ordering::Equal => {
                        skipped = true;
                        continue;
                    }
                    std::cmp::Ordering::Greater => self.order[i] - 1,
                };
                self.order[i] = res;
            }
        }
        if skipped {
            self.order.pop();
            self.tied.clear();
            self.tied.extend(self.order.windows(2).map(|w| w[0] == w[1]));
        }
        self.elements -= 1;
        self
    }

    pub fn random_total<R: Rng>(rng: &mut R, elements: usize, order: &[usize]) -> TiedI {
        let mut v = order.to_vec();
        v.shuffle(rng);
        let tied_len = v.len().saturating_sub(1);
        let tied = vec![false; tied_len];
        TiedI::new(elements, v, tied)
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::{Order, tests::std_rng};

    impl Arbitrary for TiedI {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let elements = <usize as Arbitrary>::arbitrary(g) % g.size();
            TiedI::random(&mut std_rng(g), elements)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1))).rev().map(move |i| {
                TiedI::new(
                    x.elements,
                    x.order[0..i].to_vec(),
                    x.tied[0..(i.saturating_sub(1))].to_vec(),
                )
            });
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn reverse_involution(before: TiedI) -> bool {
        let mut after = before.clone();
        after.reverse();
        after.reverse();
        before == after
    }

    #[quickcheck]
    fn owned(rank: TiedI) -> bool {
        rank == rank.as_ref().owned()
    }

    #[test]
    fn iter_groups_zero() {
        let rank = TiedI::new_zero();
        let first_group = rank.as_ref().iter_groups().next();
        assert!(first_group.is_none());
    }

    #[quickcheck]
    fn iter_groups_len(rank: TiedI) -> bool {
        let calc_len = rank.as_ref().iter_groups().map(|g| g.len()).sum::<usize>();
        rank.len() == calc_len
    }

    #[quickcheck]
    fn top_len(rank: TiedI, n: usize) -> bool {
        let values = if rank.len() == 0 { 0 } else { n % rank.len() };
        let l = rank.as_ref().top(values).len();
        values <= l && l <= rank.len()
    }

    #[quickcheck]
    fn make_complete_len(rank: TiedI, tied_last: bool) -> bool {
        let els = rank.elements;
        let rank = rank.make_complete(tied_last);
        rank.len() == els
    }

    #[test]
    fn tied_remove_last() {
        let mut r: TiedI = Tied::new_tied(20).into();
        r.remove_last();
        assert!(r.len() == 0);
    }

    #[quickcheck]
    fn top_idempotent(rank: TiedI, n: usize) -> bool {
        let values = if rank.len() == 0 { 0 } else { n % rank.len() };
        let first = rank.as_ref().top(values);
        let second = first.top(values);
        first == second
    }

    #[quickcheck]
    fn remove_last_complete(rank: TiedI) -> bool {
        let before = rank.clone();
        let mut comp1: TiedI = before.make_complete(true).into();
        let mut after = comp1.clone();
        after.remove_last();
        let mut comp2: TiedI = after.make_complete(false).into();

        comp1.normalize();
        comp2.normalize();

        comp1 == comp2
    }

    #[quickcheck]
    fn keep_top_n_threshold(mut rank: TiedI, i: usize) -> bool {
        let n = if rank.len() == 0 { 0 } else { i % rank.len() };
        let (order_group, tied_group) = rank.top_n_threshold(n);
        let o = order_group.to_vec();
        let t = tied_group.to_vec();
        rank.keep_top(n);
        let (new_order_group, new_tied_group) = rank.top_n_threshold(n);

        o == new_order_group && t == new_tied_group
    }

    #[quickcheck]
    fn keep_top_n_len(mut rank: TiedI, i: usize) -> bool {
        let n = if rank.len() == 0 { 0 } else { i % rank.len() };
        let l1 = rank.len();
        rank.keep_top(n);
        let l2 = rank.len();
        n <= l2 && l2 <= l1
    }
}
