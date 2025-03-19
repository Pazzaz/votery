//! Different orders of elements
//!
//! There are two main types of orders:
//! - [`Rank`] - An order of elements without ties, where earlier elements are
//!   ranked higher. There are also reference versions which don't own the data:
//!   [`RankRef`]
//! - [`TiedRank`] - An order of elements with ties,  where earlier elements are
//!   ranked higher and where some elements can be tied with others. There are
//!   also reference versions which don't own the data: [`TiedRankRef`].

use rand::{
    Rng,
    seq::{IteratorRandom, SliceRandom},
};

use super::{strict::Strict, strict_incomplete_ref::StrictIRef};
use crate::{
    Order, OrderOwned, OrderRef,
    partial_order::{PartialOrder, PartialOrderManual},
    unique_and_bounded,
};

/// A possibly incomplete order without any ties, owned version of [`RankRef`]
#[derive(Debug, PartialEq, Eq)]
pub struct StrictI {
    pub(crate) elements: usize,
    pub(crate) order: Vec<usize>,
}

impl Clone for StrictI {
    fn clone(&self) -> Self {
        Self { elements: self.elements, order: self.order.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.elements = source.elements;
        self.order.clone_from(&source.order);
    }
}

impl StrictI {
    pub fn new(elements: usize, order: Vec<usize>) -> Self {
        Self::try_new(elements, order).unwrap()
    }

    pub fn try_new(elements: usize, order: Vec<usize>) -> Option<Self> {
        if unique_and_bounded(elements, &order) { Some(StrictI { elements, order }) } else { None }
    }

    pub unsafe fn new_unchecked(elements: usize, order: Vec<usize>) -> Self {
        StrictI { elements, order }
    }

    pub fn parse_order(elements: usize, s: &str) -> Option<Self> {
        let mut order: Vec<usize> = Vec::with_capacity(elements);
        for number in s.split(',') {
            let n: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return None,
            };
            if n >= elements {
                return None;
            }
            order.push(n);
        }

        Some(StrictI::new(elements, order))
    }

    pub fn random<R: Rng>(rng: &mut R, elements: usize) -> StrictI {
        if elements == 0 {
            StrictI { order: Vec::new(), elements }
        } else {
            let len = rng.random_range(0..elements);

            let mut order = (0..elements).choose_multiple(rng, len);
            order.shuffle(rng);
            StrictI { order, elements }
        }
    }
}

impl TryFrom<StrictI> for Strict {
    type Error = ();

    /// Converts to complete ranking. Panics if not all elements are ranked.
    fn try_from(StrictI { elements, order }: StrictI) -> Result<Self, Self::Error> {
        if elements == order.len() { Ok(Strict { order }) } else { Err(()) }
    }
}

impl Order for StrictI {
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn to_partial(self) -> PartialOrder {
        let mut manual = PartialOrderManual::new(self.elements());
        let seen: &mut [bool] = &mut vec![false; self.elements()];
        for (i1, e1) in self.order.iter().enumerate() {
            seen[*e1] = true;
            for e2 in &self.order[(i1 + 1)..] {
                manual.set(*e2, *e1);
            }
        }
        let rest: Vec<usize> = (*seen)
            .iter()
            .enumerate()
            .filter_map(|(i, b)| if !b { Some(i) } else { None })
            .collect();

        for &upper in &self.order {
            for &lower in &rest {
                manual.set(lower, upper);
            }
        }

        // SAFETY: We set the relations in `self.order`, including transitive relations,
        // and every element in `self.order` is larger than the rest. The
        // elements in `rest` have no relations with eachother.
        let out = unsafe { manual.finish_unchecked() };
        debug_assert!(out.valid());
        out
    }
}

impl<'a> OrderOwned<'a> for StrictI {
    type Ref = StrictIRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        StrictIRef { elements: self.elements, order: &self.order }
    }
}

impl OrderRef for StrictIRef<'_> {
    type Owned = StrictI;

    fn to_owned(self) -> Self::Owned {
        StrictI::new(self.elements, self.order.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for StrictI {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let elements = <usize as Arbitrary>::arbitrary(g) % g.size();
            StrictI::random(&mut std_rng(g), elements)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1)))
                .rev()
                .map(move |i| StrictI::new(x.elements, x.order[0..i].to_vec()));
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn as_partial(b: StrictI) -> bool {
        let po = b.to_partial();
        po.valid()
    }

    #[quickcheck]
    fn as_partial_correct(b: StrictI) -> bool {
        let po = b.clone().to_partial();
        for (i, vi) in b.order.iter().enumerate() {
            for (j, vj) in b.order.iter().enumerate() {
                let index_cmp = j.cmp(&i);
                if let Some(value_cmp) = po.ord(*vi, *vj) {
                    if index_cmp != value_cmp {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        let mut values = b.order;
        values.sort();
        let rest: Vec<usize> =
            (0..b.elements).filter(|x| !values.binary_search(x).is_ok()).collect();
        for &p in &values {
            for &q in &rest {
                if !po.le(q, p) {
                    return false;
                }
            }
        }
        for &r1 in &rest {
            for &r2 in &rest {
                if r1 == r2 {
                    if !po.eq(r1, r2) {
                        return false;
                    }
                } else if po.le(r1, r2) {
                    return false;
                }
            }
        }
        po.valid()
    }

    #[quickcheck]
    fn len(b: StrictI) -> bool {
        b.len() <= b.elements()
    }
}
