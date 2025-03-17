use std::{fmt, fmt::Display, io::BufRead};

use rand::{
    Rng,
    distr::{Distribution, Uniform},
};

use crate::{pairwise_lt, remove_newline, DenseOrders};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecificDense {
    // number of orders = orders.len()
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
}

impl SpecificDense {
    pub fn new(elements: usize) -> Self {
        SpecificDense { orders: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn orders_count(&self) -> &[usize] {
        &self.orders
    }

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

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.elements == 0 {
            return Ok(());
        }

        // Now we start parsing the actual orders, consisting of a
        // number < elements. We don't use `std::io::Lines`, because we want to
        // reuse `buf` for performance reasons.
        let mut buf = String::with_capacity(20);
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of order"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            let order: usize = buf.parse().or(Err("Order is not a number"))?;
            if order >= self.elements {
                return Err("Order assigned to non-existing candidate");
            }
            self.orders.push(order);
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Set the number of elements to a larger amount
    pub fn set_elements(&mut self, elements: usize) {
        debug_assert!(self.elements <= elements);
        self.elements = elements;
    }
}

impl Display for SpecificDense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for v in &self.orders {
            writeln!(f, "{}", v)?;
        }
        Ok(())
    }
}

impl DenseOrders<'_> for SpecificDense {
    type Order = usize;
    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        // TODO: check
        self.orders.try_reserve(1).or(Err("Could not add order"))?;
        self.orders.push(v);
        Ok(())
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

        // We shrink both the number of elements, and the votes.
        // fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        //     let c = self.elements;
        //     let elements: Vec<usize> = (0..c).collect();
        //     Box::new(self.votes.shrink().zip(elements.shrink()).map(
        //         move |(shrink_votes, shrink_elements)| {
        //             let mut new_votes = Specific { votes: shrink_votes, elements: c
        // };
        // new_votes.remove_elements(&shrink_elements).unwrap();
        // debug_assert!(new_votes.valid());             new_votes
        //         },
        //     ))
        // }
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
