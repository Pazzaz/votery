use super::{groups::GroupIterator, split_ref::SplitRef, tied_incomplete::TiedI};
use crate::unique_and_bounded;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TiedIRef<'a> {
    /// The total number of elements this ranking concerns, some of them may
    /// not actually be part of the ranking.
    pub(crate) elements: usize,
    order_tied: SplitRef<'a>,
}

impl<'a> TiedIRef<'a> {
    pub fn new(elements: usize, order: &'a [usize], tied: &'a [bool]) -> Self {
        assert!(tied.len() + 1 == order.len() || order.is_empty() && tied.is_empty());
        assert!(unique_and_bounded(elements, order));
        let order_tied = SplitRef::new(order, tied);
        TiedIRef { elements, order_tied }
    }

    #[inline]
    pub fn order(self: &TiedIRef<'a>) -> &'a [usize] {
        self.order_tied.a()
    }

    #[inline]
    pub fn tied(self: &TiedIRef<'a>) -> &'a [bool] {
        self.order_tied.b()
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    // TODO: Which ones of these...
    pub fn cardinal_uniform(&self, c: &mut [usize], min: usize, max: usize) {
        debug_assert!(c.len() == self.elements);
        debug_assert!(min <= max);
        let groups = self.iter_groups().count();
        for (i, group) in self.iter_groups().enumerate() {
            let mapped = (groups - 1 - i) * (max - min) / self.elements + min;
            for e in group {
                c[*e] = mapped;
            }
        }
    }

    // ...makes sense? Both?
    pub fn cardinal_high(&self, c: &mut [usize], min: usize, max: usize) {
        debug_assert!(c.len() == self.elements);
        debug_assert!(min <= max);
        for (i, group) in self.iter_groups().enumerate() {
            let mapped = (self.elements - 1 - i) * (max - min) / self.elements + min;
            for e in group {
                c[*e] = mapped;
            }
        }
    }

    pub fn increase_elements(&mut self, elements: usize) {
        debug_assert!(self.elements <= elements);
        self.elements = elements;
    }

    /// Return an empty ranking of zero elements.
    pub fn new_zero() -> Self {
        TiedIRef::new(0, &[], &[])
    }

    /// Return an empty ranking of `elements`.
    pub fn new_zero_c(elements: usize) -> Self {
        let mut rank = TiedIRef::new_zero();
        rank.increase_elements(elements);
        rank
    }

    /// Return an empty ranking of the same `elements` as `self`.
    pub fn zeroed(&self) -> Self {
        TiedIRef::new(self.elements, &[], &[])
    }

    /// Return a ranking of the top `n` elements. The ranking will be larger
    /// than `n` if ties prevent us from saying which ones are ranked
    /// higher.
    #[must_use]
    pub fn top(&self, n: usize) -> Self {
        if n == 0 {
            return self.zeroed();
        }
        debug_assert!(n <= self.order().len());
        let mut i = n;
        while i < self.order().len() {
            if self.tied()[i - 1] {
                i += 1;
            } else {
                break;
            }
        }
        TiedIRef::new(self.elements, &self.order()[0..i], &self.tied()[0..(i.saturating_sub(1))])
    }

    pub fn len(&self) -> usize {
        self.order().len()
    }

    pub fn owned(self) -> TiedI {
        TiedI::new(self.elements, self.order().to_vec(), self.tied().to_vec())
    }

    pub fn iter_groups(&self) -> GroupIterator<'a> {
        GroupIterator { order: *self }
    }

    pub fn group(&self, n: usize) -> Option<&[usize]> {
        self.iter_groups().nth(n)
    }

    /// Returns group of element `c`. 0 is highest rank. Takes `O(n)` time
    pub fn group_of(&self, c: usize) -> Option<usize> {
        let mut group = 0;
        for i in 0..self.len() {
            if self.order()[i] == c {
                return Some(group);
            }
            if !self.tied()[i] {
                group += 1;
            }
        }
        None
    }

    pub fn winners(&self) -> &'a [usize] {
        let i = self.tied().iter().take_while(|x| **x).count();
        &self.order()[0..=i]
    }

    pub fn is_empty(&self) -> bool {
        self.order().is_empty()
    }

    /// Returns a list of all elements with the top rank, and a ranking of the
    /// rest
    pub fn split_winner_group(&self) -> (&'a [usize], TiedIRef<'a>) {
        if self.is_empty() {
            return (&[], *self);
        }
        let mut values = 1;
        for k in self.tied() {
            if *k {
                values += 1;
            } else {
                break;
            }
        }
        let (out, rest_order, rest_tied): (&[usize], &[usize], &[bool]) = if values == self.len() {
            (self.order(), &[], &[])
        } else {
            let (_, rest_tied) = self.tied().split_at(values);
            let (out, rest_order) = self.order().split_at(values);
            (out, rest_order, rest_tied)
        };
        (out, TiedIRef::new(self.elements, rest_order, rest_tied))
    }
}
