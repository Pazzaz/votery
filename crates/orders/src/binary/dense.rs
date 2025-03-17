use std::{
    fmt::{self, Display},
    io::BufRead,
};

use rand::{
    Rng,
    distr::{Bernoulli, Distribution},
};

use crate::{cardinal::CardinalDense, pairwise_lt, remove_newline, DenseOrders};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryDense {
    pub orders: Vec<bool>,
    pub(crate) elements: usize,
    pub orders_count: usize,
}

impl BinaryDense {
    pub fn new(elements: usize) -> BinaryDense {
        BinaryDense { orders: Vec::new(), elements, orders_count: 0 }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub(crate) fn valid(&self) -> bool {
        !(self.elements == 0 && (self.orders_count != 0 || !self.orders.is_empty())
            || self.orders.len() != self.orders_count * self.elements)
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
        data.orders_count += new_orders;
        debug_assert!(data.valid());
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.elements == 0 {
            return Ok(());
        }

        // Should fit each line, including "\r\n"
        let mut buf = String::with_capacity(self.elements * 2 + 1);
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of order"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            let bbuf = buf.as_bytes();
            // Each order has a value for each element and a comma after every
            // element, except for the last element.
            // => len = element + element - 1
            if bbuf.len() == (self.elements * 2 - 1) {
                for i in 0..self.elements {
                    match bbuf[i * 2] {
                        b'0' => self.orders.push(false),
                        b'1' => self.orders.push(true),
                        _ => return Err("Invalid order"),
                    }
                    if i != self.elements - 1 && bbuf[i * 2 + 1] != b',' {
                        return Err("Invalid order");
                    }
                }
            } else {
                return Err("Invalid order");
            }
            self.orders_count += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Convert each order to a cardinal order, with an approval being 1 and
    /// disapproval 0.
    ///
    /// Returns `Err` if it failed to allocate
    pub fn to_cardinal(&self) -> Result<CardinalDense, &'static str> {
        let mut orders: Vec<usize> = Vec::new();
        orders
            .try_reserve_exact(self.elements * self.orders_count)
            .or(Err("Could not allocate"))?;
        orders.extend(self.orders.iter().map(|x| if *x { 1 } else { 0 }));
        let v = CardinalDense {
            orders,
            elements: self.elements,
            orders_count: self.orders_count,
            min: 0,
            max: 1,
        };
        debug_assert!(v.valid());
        Ok(v)
    }
}

impl Display for BinaryDense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.orders_count {
            for j in 0..(self.elements - 1) {
                let b = self.orders[i * self.elements + j];
                let v = if b { '1' } else { '0' };
                write!(f, "{},", v)?;
            }
            let b_last = self.orders[i * self.elements + (self.elements - 1)];
            let v_last = if b_last { '1' } else { '0' };
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}

impl<'a> DenseOrders<'a> for BinaryDense {
    type Order = &'a [bool];
    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        if v.len() != self.elements {
            return Err("Order must contains all elements");
        }
        self.orders.try_reserve(self.elements).or(Err("Could not add order"))?;
        for c in v {
            self.orders.push(*c);
        }
        self.orders_count += 1;
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.orders_count {
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
        self.orders.truncate(self.orders_count * new_elements);
        self.elements = new_elements;
        debug_assert!(self.valid());
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
        let around: BinaryDense = orders.to_cardinal().unwrap().to_binary_cutoff(1).unwrap();
        around == orders
    }
}
