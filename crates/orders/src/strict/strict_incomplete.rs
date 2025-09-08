use rand::{
    Rng,
    seq::{IteratorRandom, SliceRandom},
};

use super::{strict::Total, strict_incomplete_ref::ChainRef};
use crate::{
    Order, OrderOwned, OrderRef,
    partial_order::{PartialOrder, PartialOrderManual},
    unique_and_bounded,
};

/// A possibly incomplete order without any ties, owned version of [`ChainRef`]
#[derive(Debug, PartialEq, Eq)]
pub struct Chain {
    pub(crate) elements: usize,
    pub(crate) order: Vec<usize>,
}

impl Clone for Chain {
    fn clone(&self) -> Self {
        Self { elements: self.elements, order: self.order.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.elements = source.elements;
        self.order.clone_from(&source.order);
    }
}

impl Chain {
    pub fn new(elements: usize, order: Vec<usize>) -> Self {
        Self::try_new(elements, order).unwrap()
    }

    pub fn try_new(elements: usize, order: Vec<usize>) -> Option<Self> {
        if unique_and_bounded(elements, &order) { Some(Chain { elements, order }) } else { None }
    }

    pub unsafe fn new_unchecked(elements: usize, order: Vec<usize>) -> Self {
        Chain { elements, order }
    }

    /// Clones from `source` to `self`, similar to [`Clone::clone_from`].
    pub fn clone_from_ref(&mut self, source: ChainRef) {
        self.order.clone_from_slice(source.order);
        self.elements = source.elements;
    }

    pub fn random<R: Rng>(rng: &mut R, elements: usize) -> Chain {
        if elements == 0 {
            Chain { order: Vec::new(), elements }
        } else {
            let len = rng.random_range(0..elements);

            let mut order = (0..elements).choose_multiple(rng, len);
            order.shuffle(rng);
            Chain { order, elements }
        }
    }
}

impl TryFrom<Chain> for Total {
    type Error = ();

    /// Convert to total order. Returns `Err` if not all elements are ranked.
    fn try_from(Chain { elements, order }: Chain) -> Result<Self, Self::Error> {
        if elements == order.len() { Ok(Total { order }) } else { Err(()) }
    }
}

impl Order for Chain {
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn to_partial(self) -> PartialOrder {
        let mut manual = PartialOrderManual::new(self.elements());
        for (i1, e1) in self.order.iter().enumerate() {
            for e2 in &self.order[(i1 + 1)..] {
                manual.set(*e2, *e1);
            }
        }
        // SAFETY: We set the relations in `self.order`, including transitive relations.
        // The elements in `rest` have no relations with eachother, or the
        // non-ordered elements.
        unsafe { manual.finish_unchecked() }
    }
}

impl<'a> OrderOwned<'a> for Chain {
    type Ref = ChainRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        ChainRef { elements: self.elements, order: &self.order }
    }
}

impl OrderRef for ChainRef<'_> {
    type Owned = Chain;

    fn to_owned(self) -> Self::Owned {
        Chain::new(self.elements, self.order.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for Chain {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let elements = <usize as Arbitrary>::arbitrary(g) % g.size();
            Chain::random(&mut std_rng(g), elements)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1)))
                .rev()
                .map(move |i| Chain::new(x.elements, x.order[0..i].to_vec()));
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn as_partial(b: Chain) -> bool {
        let po = b.to_partial();
        po.valid()
    }

    #[quickcheck]
    fn as_partial_correct(b: Chain) -> bool {
        let po = b.clone().to_partial();
        if !po.valid() {
            return false;
        }
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
                if po.le(q, p) || po.le(p, q) {
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
    fn len(b: Chain) -> bool {
        b.len() <= b.elements()
    }
}
