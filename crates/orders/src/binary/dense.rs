use rand::{
    Rng,
    distr::{Bernoulli, Distribution},
};

use super::BinaryRef;
use crate::{DenseOrders, cardinal::CardinalDense, pairwise_lt};

#[derive(Debug, PartialEq, Eq)]
pub struct BinaryDense {
    pub orders: Vec<bool>,
    pub(crate) elements: usize,
}

impl Clone for BinaryDense {
    fn clone(&self) -> Self {
        Self { orders: self.orders.clone(), elements: self.elements }
    }

    fn clone_from(&mut self, source: &Self) {
        self.orders.clone_from(&source.orders);
        self.elements = source.elements;
    }
}

impl BinaryDense {
    pub fn new(elements: usize) -> BinaryDense {
        BinaryDense { orders: Vec::new(), elements }
    }

    pub fn new_from_parts(orders: Vec<bool>, elements: usize) -> BinaryDense {
        assert!(orders.is_empty() && elements == 0 || orders.len().is_multiple_of(elements));
        BinaryDense { orders, elements }
    }

    pub unsafe fn new_from_parts_unchecked(orders: Vec<bool>, elements: usize) -> BinaryDense {
        BinaryDense { orders, elements }
    }

    #[cfg(test)]
    pub(crate) fn valid(&self) -> bool {
        self.elements == 0 && self.orders.is_empty() || self.orders.len() % self.elements == 0
    }

    /// Sample and add `new_orders` new orders, where each elements has a
    /// chance of `p` to be chosen, where 0.0 <= `p` <= 1.0
    pub fn bernoulli<R: Rng>(data: &mut Self, rng: &mut R, new_orders: usize, p: f64) {
        if data.elements == 0 || new_orders == 0 {
            return;
        }

        data.orders.reserve(new_orders * data.elements);
        let dist = Bernoulli::new(p).unwrap();
        for _ in 0..new_orders {
            for _ in 0..data.elements {
                let b: bool = dist.sample(rng);
                data.orders.push(b);
            }
        }
    }
}

impl TryFrom<&BinaryDense> for CardinalDense {
    type Error = &'static str;

    /// Convert each order to a cardinal order, with an approval being 1 and
    /// disapproval 0.
    ///
    /// Returns `Err` if it failed to allocate.
    fn try_from(value: &BinaryDense) -> Result<Self, Self::Error> {
        let mut orders: Vec<usize> = Vec::new();
        orders.try_reserve_exact(value.elements * value.len()).or(Err("Could not allocate"))?;
        orders.extend(value.orders.iter().map(|x| if *x { 1 } else { 0 }));
        Ok(CardinalDense { orders, elements: value.elements, min: 0, max: 1 })
    }
}

impl<'a> DenseOrders<'a> for BinaryDense {
    type Order = BinaryRef<'a>;
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        if self.elements == 0 { 0 } else { self.orders.len() / self.elements }
    }

    fn try_get(&'a self, i: usize) -> Option<Self::Order> {
        if i < self.len() {
            let start = i * self.elements;
            let end = (i + 1) * self.elements;
            let s = &self.orders[start..end];
            Some(BinaryRef::new(s))
        } else {
            None
        }
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        if v.len() != self.elements {
            return Err("Order must contains all elements");
        }
        self.orders.try_reserve(self.elements).or(Err("Could not add order"))?;
        self.orders.extend_from_slice(v.values);
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.len() {
            let mut t_i = 0;
            let mut offset = 0;
            for j in 0..self.elements {
                if targets[t_i] == j {
                    t_i += 1;
                    offset += 1;
                } else {
                    let old_index = i * self.elements + j;
                    let new_index = i * new_elements + (j - offset);
                    debug_assert!(new_index <= old_index);
                    self.orders[new_index] = self.orders[old_index];
                }
            }
        }
        self.orders.truncate(self.len() * new_elements);
        self.elements = new_elements;
        Ok(())
    }

    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_orders: usize) {
        BinaryDense::bernoulli(self, rng, new_orders, 0.5);
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for BinaryDense {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut orders_count, mut elements): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            orders_count = orders_count % g.size();
            elements = elements % g.size();

            let mut orders = BinaryDense::new(elements);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            debug_assert!(orders.valid());
            orders
        }
    }

    #[quickcheck]
    fn to_cardinal(orders: BinaryDense) -> bool {
        let cardinal: CardinalDense = (&orders).try_into().unwrap();
        let around: BinaryDense = cardinal.to_binary_cutoff(1).unwrap();
        around == orders
    }
}
