use rand::{
    Rng,
    distr::{Distribution, Uniform},
};

use crate::{DenseOrders, pairwise_lt};

/// A collection of elements.
///
/// Collection of orders where every order is a specific element. Some would say
/// that this isn't an order at all, but it's useful to model a collection of
/// votes in voting theory.
///
/// ```
/// use orders::{DenseOrders, specific::SpecificDense};
///
/// let mut orders = SpecificDense::from_vec(5, vec![4, 3, 4, 2, 4]);
///
/// assert_eq!(orders.majority(), Some(4));
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct SpecificDense {
    // number of orders = orders.len()
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
}

impl Clone for SpecificDense {
    fn clone(&self) -> Self {
        Self { orders: self.orders.clone(), elements: self.elements }
    }

    fn clone_from(&mut self, source: &Self) {
        self.orders.clone_from(&source.orders);
        self.elements = source.elements;
    }
}

impl SpecificDense {
    pub fn new(elements: usize) -> Self {
        SpecificDense { orders: Vec::new(), elements }
    }

    /// Create a `SpecificDense` from a list of elements.
    ///
    /// # Panics
    ///
    /// Panics if any of the elements is not a valid.
    pub fn from_vec(elements: usize, orders: Vec<usize>) -> Self {
        Self::try_from_vec(elements, orders).unwrap()
    }

    /// Create a `SpecificDense` from a list of elements. Returns `None` if any
    /// of the elements is not valid.
    pub fn try_from_vec(elements: usize, orders: Vec<usize>) -> Option<Self> {
        if orders.iter().all(|&x| x < elements) { Some(Self { orders, elements }) } else { None }
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> {
        self.orders.iter().copied()
    }

    /// Return the element that the majority of orders consists of. Returns
    /// `None` if no element is the majority.
    pub fn majority(&self) -> Option<usize> {
        if self.elements == 1 {
            return Some(0);
        }
        let mut score = vec![0; self.elements];
        for i in &self.orders {
            score[*i] += 1;
        }
        (0..self.elements).find(|&i| score[i] > (self.orders.len() / 2))
    }

    // Checks if all invariants of the format are valid, used in debug_asserts and
    // tests
    fn valid(&self) -> bool {
        if self.elements == 0 && !self.orders.is_empty() {
            return false;
        }

        for v in &self.orders {
            if *v >= self.elements {
                return false;
            }
        }
        true
    }

    /// Set the number of elements to a larger amount.
    pub fn set_elements(&mut self, elements: usize) {
        assert!(self.elements <= elements);
        self.elements = elements;
    }
}

impl DenseOrders<'_> for SpecificDense {
    type Order = usize;
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        self.orders.len()
    }

    fn try_get(&self, i: usize) -> Option<Self::Order> {
        self.orders.get(i).copied()
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        if v < self.elements {
            self.orders.try_reserve(1).or(Err("Could not add order"))?;
            self.orders.push(v);
            Ok(())
        } else {
            Err("Invalid element")
        }
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        let mut j = 0;
        for i in 0..self.orders.len() {
            let v = self.orders[i];
            if let Err(offset) = targets.binary_search(&v) {
                self.orders[j] = v - offset;
                j += 1;
            }
        }
        self.orders.truncate(j);
        self.elements = new_elements;
        debug_assert!(self.valid());
        Ok(())
    }

    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 || new_orders == 0 {
            return;
        }

        self.orders.reserve(new_orders);
        let dist = Uniform::new(0, self.elements).unwrap();
        for _ in 0..new_orders {
            let i = dist.sample(rng);
            self.orders.push(i);
        }
        debug_assert!(self.valid());
    }
}

impl FromIterator<usize> for SpecificDense {
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        let ii = iter.into_iter();
        let (min_len, _) = ii.size_hint();
        let mut orders = Vec::with_capacity(min_len);
        let mut max = 0;
        for v in ii {
            orders.push(v);
            if v > max {
                max = v;
            }
        }
        SpecificDense { orders, elements: max + 1 }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for SpecificDense {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut orders_count, mut elements): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            orders_count = orders_count % g.size();
            elements = elements % g.size();

            let mut orders = SpecificDense::new(elements);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            debug_assert!(orders.valid());
            orders
        }
    }

    #[quickcheck]
    fn majority_bound(orders: SpecificDense) -> bool {
        let major = orders.majority();
        eprintln!("{:?}", major);
        match major {
            Some(i) => i < orders.elements,
            None => true,
        }
    }
}
