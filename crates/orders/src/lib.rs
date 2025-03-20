//! This is a library of different representations of orders. The most general
//! order type is [`PartialOrder`](partial_order::PartialOrder), but we can
//! represent orders more effectively if we use a type for a smaller set of
//! orders. Every order is finite, and stores how many elements the ordered set
//! has&nbsp;(see [`Order::elements`]).
//!
//! The different types of orders are
//! - [`Binary`](binary), a ranked order where every element either has a high
//!   rank or a low rank.
//! - [`Cardinal`](cardinal), a ranked order where every element is assigned
//!   some number.
//! - `PartialOrder`,
//! - [`Rank`](rank), a linear order containing every element.
//! - [`TiedRank`](tied_rank), a linear order containing every element, where
//!   some elements can be tied.
//!
//! There are also variants of the orders which don't store all elements. Stored
//! elements are considered higher in the poset than non-stored elements.
//!
//! There are also custom collections of orders. These are more effective than
//! just using a [`Vec`] of orders, as the orders themselves often contain a
//! `Vec`. By using custom containers it's possible to store them in a more
//! compact form and avoid nested containers.

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod binary;
pub mod cardinal;
pub mod partial_order;
pub mod specific;
pub mod strict;
pub mod tied;

fn pairwise_lt(v: &[usize]) -> bool {
    if v.len() >= 2 {
        for i in 0..(v.len() - 1) {
            if v[i] >= v[i + 1] {
                return false;
            }
        }
    }
    true
}

fn get_order<T: Ord>(v: &[T], reverse: bool) -> Vec<usize> {
    if v.is_empty() {
        return Vec::new();
    } else if v.len() == 1 {
        return vec![0];
    }

    let mut tmp: Vec<(usize, &T)> = Vec::with_capacity(v.len());
    for (i, el) in v.iter().enumerate() {
        tmp.push((i, el));
    }
    tmp.sort_by(|a, b| (*a.1).cmp(b.1));
    if reverse {
        tmp.reverse();
    }
    let mut out = vec![0; v.len()];
    if let Some((b, bs)) = tmp.split_first_mut() {
        let mut current: &T = b.1;
        let mut i: usize = 0;
        for x in bs.iter_mut() {
            if *x.1 != *current {
                current = x.1;
                i += 1;
            }
            out[x.0] = i;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};
    use rand::{SeedableRng, rngs::StdRng};

    // `Gen` contains a rng, but it's a private member so this method is used to get
    // a standard rng generated from `Gen`
    pub fn std_rng(g: &mut Gen) -> StdRng {
        let mut seed = [0u8; 32];
        for i in 0..32 {
            seed[i] = Arbitrary::arbitrary(g);
        }
        StdRng::from_seed(seed)
    }
}

// Sort two arrays, sorted according to the values in `b`.
// Uses insertion sort
pub(crate) fn sort_using<A, B>(a: &mut [A], b: &mut [B])
where
    B: PartialOrd,
{
    debug_assert!(a.len() == b.len());
    let mut i: usize = 1;
    while i < b.len() {
        let mut j = i;
        while j > 0 && b[j - 1] > b[j] {
            a.swap(j, j - 1);
            b.swap(j, j - 1);
            j -= 1;
        }
        i += 1;
    }
}

pub trait Order {
    /// The number of elements that can be in this order.
    fn elements(&self) -> usize;

    /// The number of elements currently part of this order.
    fn len(&self) -> usize;

    /// Shorthand for `self.len() == 0`
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn to_partial(self) -> partial_order::PartialOrder;
}

pub trait OrderOwned<'a> {
    type Ref;
    fn as_ref(&'a self) -> Self::Ref;
}

pub trait OrderRef {
    type Owned;
    fn to_owned(self) -> Self::Owned;
}

use rand::Rng;

// Lifetime needed because `Order` may be a reference which then needs a
// lifetime
pub trait DenseOrders<'a> {
    type Order;
    /// Number of elements
    fn elements(&self) -> usize;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str>;

    fn try_get(&'a self, i: usize) -> Option<Self::Order>;

    fn get(&'a self, i: usize) -> Self::Order {
        self.try_get(i).unwrap()
    }

    /// Removes element from the orders, offsetting the other elements to
    /// take their place.
    fn remove_element(&mut self, target: usize) -> Result<(), &'static str>;

    /// Sample and add `new_orders` uniformly random orders for this format,
    /// using random numbers from `rng`.
    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_orders: usize);
}

fn unique_and_bounded(elements: usize, order: &[usize]) -> bool {
    for (i, &a) in order.iter().enumerate() {
        if a >= elements {
            return false;
        }
        for (j, &b) in order.iter().enumerate() {
            if i == j {
                continue;
            }
            if a == b {
                return false;
            }
        }
    }
    true
}
