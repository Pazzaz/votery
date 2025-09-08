use rand::{
    distr::{Distribution, Uniform},
    seq::SliceRandom,
};

use super::TotalDense;
use crate::{DenseOrders, strict::ChainRef};

/// SOI - Strict Orders - Incomplete List
///
/// A packed list of (possibly incomplete) strict orders, with related methods.
#[derive(Debug)]
pub struct ChainDense {
    pub(crate) orders: Vec<usize>,

    // End position of order
    pub(crate) order_end: Vec<usize>,
    pub(crate) elements: usize,
}

impl Clone for ChainDense {
    fn clone(&self) -> Self {
        Self {
            orders: self.orders.clone(),
            order_end: self.order_end.clone(),
            elements: self.elements,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.orders.clone_from(&source.orders);
        self.order_end.clone_from(&source.order_end);
        self.elements = source.elements;
    }
}

impl ChainDense {
    pub fn new(elements: usize) -> Self {
        ChainDense { orders: Vec::new(), order_end: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    #[cfg(test)]
    fn valid(&self) -> bool {
        let mut seen = vec![false; self.elements];
        for v in self.iter() {
            seen.fill(false);
            for &i in v.order {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        for &o in &self.order_end {
            if o > self.orders.len() {
                return false;
            }
        }
        for o in self.order_end.windows(2) {
            if o[0] > o[1] {
                return false;
            }
        }
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = ChainRef<'_>> {
        (0..self.len()).map(|i| self.get(i))
    }
}

impl<'a> DenseOrders<'a> for ChainDense {
    type Order = ChainRef<'a>;

    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        self.order_end.len()
    }

    fn try_get(&'a self, i: usize) -> Option<Self::Order> {
        if i < self.len() {
            let start: usize = if i == 0 { 0 } else { self.order_end[i - 1] };
            let end = self.order_end[i];
            Some(ChainRef::new(self.elements, &self.orders[start..end]))
        } else {
            None
        }
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        assert!(v.elements == self.elements);
        self.orders.reserve(v.len());
        let start = self.order_end.last().unwrap_or(&0);
        self.order_end.push(*start + v.len());
        self.orders.extend_from_slice(v.order);
        Ok(())
    }

    fn remove_element(&mut self, _target: usize) -> Result<(), &'static str> {
        todo!();
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 {
            return;
        }
        let v: &mut [usize] = &mut (0..self.elements).collect::<Vec<usize>>();
        self.orders.reserve(self.elements * new_orders);
        let range = Uniform::new(0, self.elements).unwrap();
        for _ in 0..new_orders {
            let elements = range.sample(rng) + 1;
            v.shuffle(rng);
            for &el in &v[..elements] {
                self.orders.push(el);
            }
            let start = self.order_end.last().unwrap_or(&0);
            self.order_end.push(*start + elements);
        }
    }
}

impl From<TotalDense> for ChainDense {
    fn from(value: TotalDense) -> Self {
        let orders: usize = value.len();
        let order_end = (0..orders).map(|i| (i + 1) * value.elements).collect();
        ChainDense { orders: value.orders, order_end, elements: value.elements }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::{OrderOwned, OrderRef, strict::Chain, tests::std_rng};

    impl Arbitrary for ChainDense {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut orders_count, mut elements): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            orders_count = orders_count % g.size();
            elements = elements % g.size();

            let mut orders = ChainDense::new(elements);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            orders
        }
    }

    #[quickcheck]
    fn arbitrary(orders: ChainDense) -> bool {
        orders.valid()
    }

    #[quickcheck]
    fn iter_collect(orders: ChainDense) -> bool {
        let orig = orders.clone();
        let parts: Vec<Chain> = orders.iter().map(|x| x.to_owned()).collect();
        for i in 0..orders.len() {
            if parts[i].as_ref() != orig.get(i) {
                return false;
            }
        }
        true
    }
}
