mod dense;

pub use dense::BinaryDense;
use rand::{Rng, distr::StandardUniform};

use super::{Order, OrderOwned, OrderRef, partial_order::PartialOrder};
use crate::partial_order::PartialOrderManual;

#[derive(Debug)]
pub struct Binary {
    values: Vec<bool>,
}

impl Clone for Binary {
    fn clone(&self) -> Self {
        Self { values: self.values.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.values.clone_from(&source.values);
    }
}

impl Binary {
    pub fn new(v: Vec<bool>) -> Self {
        Binary { values: v }
    }

    /// Clones from `source` to `self`, similar to [`Clone::clone_from`].
    pub fn clone_from_ref(&mut self, source: BinaryRef) {
        self.values.clone_from_slice(source.values);
    }

    pub fn random<R: Rng>(rng: &mut R, elements: usize) -> Binary {
        let values = rng.sample_iter(StandardUniform).take(elements).collect();
        Binary { values }
    }

    pub fn into_inner(self) -> Vec<bool> {
        self.values
    }
}

pub struct BinaryRef<'a> {
    values: &'a [bool],
}

impl<'a> BinaryRef<'a> {
    pub fn new(v: &'a [bool]) -> Self {
        BinaryRef { values: v }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn values(&self) -> &'a [bool] {
        self.values
    }
}

impl Order for Binary {
    fn elements(&self) -> usize {
        self.values.len()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    /// Convert to `PartialOrder`. We only know that `true` values are above
    /// `false` values, so those are the only relations that will be
    /// included in the result.
    fn to_partial(self) -> PartialOrder {
        let mut tmp = PartialOrderManual::new(self.elements());
        for (i1, b1) in self.values.iter().enumerate() {
            for (i2, b2) in self.values.iter().enumerate().skip(i1 + 1) {
                match (b1, b2) {
                    (true, false) => tmp.set(i2, i1),
                    (false, true) => tmp.set(i1, i2),
                    (true, true) | (false, false) => {}
                }
            }
        }
        // SAFETY: There won't be any transitive relations between elements, and we
        // iterated through every pair of elements, so we've set every
        // relation.
        let out = unsafe { tmp.finish_unchecked() };
        debug_assert!(out.valid());
        out
    }
}

impl<'a> OrderOwned<'a> for Binary {
    type Ref = BinaryRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        BinaryRef { values: &self.values }
    }
}

impl OrderRef for BinaryRef<'_> {
    type Owned = Binary;

    fn to_owned(self) -> Self::Owned {
        Binary { values: self.values.to_vec() }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for Binary {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let elements = <usize as Arbitrary>::arbitrary(g) % g.size();
            Binary::random(&mut std_rng(g), elements)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1)))
                .rev()
                .map(move |i| Binary::new(x.values[0..i].to_vec()));
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn as_partial(b: Binary) -> bool {
        let po = b.to_partial();
        po.valid()
    }

    #[quickcheck]
    fn as_partial_correct(b: Binary) -> bool {
        let po = b.clone().to_partial();
        for i in 0..b.elements() {
            for j in 0..b.elements() {
                let goal = if i == j {
                    Some(Ordering::Equal)
                } else {
                    match (b.values[i], b.values[j]) {
                        (false, true) => Some(Ordering::Less),
                        (true, false) => Some(Ordering::Greater),
                        (true, true) | (false, false) => None,
                    }
                };
                if po.ord(i, j) != goal {
                    return false;
                }
            }
        }
        po.valid()
    }

    #[quickcheck]
    fn complete(b: Binary) -> bool {
        b.len() == b.elements()
    }
}
