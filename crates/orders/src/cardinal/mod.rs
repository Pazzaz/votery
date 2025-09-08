mod dense;

pub use dense::*;
use rand::{
    Rng,
    distr::{Distribution, Uniform},
};

use super::{
    Order, OrderOwned, OrderRef,
    binary::Binary,
    partial_order::{PartialOrder, PartialOrderManual},
};

#[derive(Debug)]
pub struct Cardinal {
    values: Vec<usize>,
}

impl Clone for Cardinal {
    fn clone(&self) -> Self {
        Self { values: self.values.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.values.clone_from(&source.values);
    }
}

impl Cardinal {
    pub fn new(v: Vec<usize>) -> Self {
        Cardinal { values: v }
    }

    pub fn remove(&mut self, n: usize) {
        self.values.remove(n);
    }

    /// Clones from `source` to `self`, similar to [`Clone::clone_from`].
    pub fn clone_from_ref(&mut self, source: CardinalRef) {
        self.values.clone_from_slice(source.values);
    }

    pub fn random<R: Rng>(rng: &mut R, elements: usize, min: usize, max: usize) -> Cardinal {
        assert!(min <= max);
        let dist = Uniform::new_inclusive(min, max).unwrap();
        let values = dist.sample_iter(rng).take(elements).collect();
        Cardinal { values }
    }
}

pub struct CardinalRef<'a> {
    values: &'a [usize],
}

impl<'a> CardinalRef<'a> {
    pub fn new(s: &'a [usize]) -> Self {
        CardinalRef { values: s }
    }

    /// Returns the number of elements in the order
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn values(&self) -> &'a [usize] {
        self.values
    }

    /// Convert to binary order, where any value less than `cutoff` becomes
    /// `false` and larger becomes `true`.
    pub fn to_binary(&self, cutoff: usize) -> Binary {
        let values = self.values.iter().map(|x| *x >= cutoff).collect();
        Binary::new(values)
    }
}

impl Order for Cardinal {
    fn elements(&self) -> usize {
        self.values.len()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    /// Converts `Cardinal` to a `PartialOrder`: if two elements `a` and `b`
    /// have cardinal values `f(a)` and `f(b)`, where `f(a) ≤ f(b)`, then
    /// the partial order will include `a ≤ b`. Equal cardinal values will
    /// not be considered equal in the partial order.
    fn to_partial(self) -> PartialOrder {
        let mut tmp = PartialOrderManual::new(self.elements());
        for (i, e1) in self.values.iter().enumerate() {
            for (j, e2) in self.values.iter().enumerate() {
                if e1 == e2 {
                    continue;
                }
                tmp.set_ord(i, j, e1.cmp(e2));
            }
        }
        tmp.finish()
    }
}

impl<'a> OrderOwned<'a> for Cardinal {
    type Ref = CardinalRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        CardinalRef { values: &self.values }
    }
}

impl OrderRef for CardinalRef<'_> {
    type Owned = Cardinal;

    fn to_owned(self) -> Self::Owned {
        Cardinal { values: self.values.to_owned() }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for Cardinal {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let elements = <usize as Arbitrary>::arbitrary(g) % g.size();
            let (a, b): (usize, usize) = Arbitrary::arbitrary(g);
            let (min, max) = if b < a { (b, a) } else { (a, b) };
            Cardinal::random(&mut std_rng(g), elements, min, max)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1)))
                .rev()
                .map(move |i| Cardinal::new(x.values[0..i].to_vec()));
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn as_partial(b: Cardinal) -> bool {
        let po = b.to_partial();
        po.valid()
    }

    #[quickcheck]
    fn as_partial_correct(b: Cardinal) -> bool {
        let po = b.clone().to_partial();
        for i in 0..b.elements() {
            for j in 0..b.elements() {
                let goal = if i == j {
                    Some(Ordering::Equal)
                } else {
                    match b.values[i].cmp(&b.values[j]) {
                        Ordering::Less => Some(Ordering::Less),
                        Ordering::Equal => None,
                        Ordering::Greater => Some(Ordering::Greater),
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
    fn complete(b: Cardinal) -> bool {
        b.len() == b.elements()
    }
}
