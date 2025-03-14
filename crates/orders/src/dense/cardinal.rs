use std::{
    cmp::Ordering,
    fmt::{self, Display},
    io::BufRead,
    slice::Chunks,
};

use rand::distributions::{Distribution, Uniform};

use super::{Binary, DenseOrders, remove_newline, toi::TiedOrdersIncomplete};
use crate::pairwise_lt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cardinal {
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
    pub(crate) orders_count: usize,
    pub min: usize,
    pub max: usize,
}

impl Cardinal {
    pub fn new(elements: usize, min: usize, max: usize) -> Cardinal {
        debug_assert!(min <= max);
        Cardinal { orders: Vec::new(), elements, orders_count: 0, min, max }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub(crate) fn valid(&self) -> bool {
        if self.elements == 0 && (self.orders_count != 0 || !self.orders.is_empty())
            || self.orders.len() != self.orders_count * self.elements
        {
            return false;
        }
        for i in 0..self.orders_count {
            for j in 0..self.elements {
                let v = self.orders[self.elements * i + j];
                if v < self.min || v > self.max {
                    return false;
                }
            }
        }
        true
    }

    /// Multiply each order score with constant `a`, changing the `min` and `max`
    /// score.
    pub fn mul(&mut self, a: usize) {
        if a == 1 {
            return;
        }
        let new_min = self.min.checked_mul(a).unwrap();
        let new_max = self.max.checked_mul(a).unwrap();
        for i in 0..self.orders_count {
            for j in 0..self.elements {
                self.orders[i * self.elements + j] *= a;
            }
        }
        self.min = new_min;
        self.max = new_max;
        debug_assert!(self.valid());
    }

    /// Add to each order score a constant `a`, changing the `min` and `max`
    /// score.
    pub fn add_constant(&mut self, a: usize) {
        if a == 0 {
            return;
        }
        let new_min = self.min.checked_add(a).unwrap();
        let new_max = self.max.checked_add(a).unwrap();
        for i in 0..self.orders_count {
            for j in 0..self.elements {
                self.orders[i * self.elements + j] += a;
            }
        }
        self.min = new_min;
        self.max = new_max;
        debug_assert!(self.valid());
    }

    /// Subtracts from each order score a constant `a`, changing the `min` and
    /// `max` score.
    pub fn sub(&mut self, a: usize) {
        if a == 0 {
            return;
        }
        let new_min = self.min.checked_sub(a).unwrap();
        let new_max = self.max.checked_sub(a).unwrap();
        for i in 0..self.orders_count {
            for j in 0..self.elements {
                self.orders[i * self.elements + j] -= a;
            }
        }
        self.min = new_min;
        self.max = new_max;
        debug_assert!(self.valid());
    }

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.elements == 0 {
            return Ok(());
        }
        // The smallest each order can be is all '0' seperated by ','
        let mut buf = String::with_capacity(self.elements * 2);
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of order"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            let mut count = 0;
            for s in buf.split(',') {
                count += 1;
                let v: usize = s.parse().or(Err("Order is not a number"))?;
                if v > self.max {
                    return Err("Cardinal order is larger than max value");
                } else if v < self.min {
                    return Err("Cardinal order is smaller than min value");
                }
                self.orders.push(v);
            }
            if count > self.elements {
                return Err("Too many elements listed in order");
            } else if count < self.elements {
                return Err("Too few elements listed in order");
            }
            self.orders_count += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Number of valid values
    pub fn values(&self) -> usize {
        self.max - self.min + 1
    }

    /// The Kotze-Pereira transformation
    pub fn kp_tranform(&self) -> Result<Binary, &'static str> {
        let mut binary_orders: Vec<bool> = Vec::new();
        let orders_size = self
            .elements
            .checked_mul(self.orders_count)
            .ok_or("Number of orders would be too large")?
            .checked_mul(self.values() - 1)
            .ok_or("Number of orders would be too large")?;
        binary_orders.try_reserve_exact(orders_size).or(Err("Could not allocate"))?;
        for i in 0..self.orders_count {
            let order = &self.orders[i * self.elements..(i + 1) * self.elements];
            for lower in self.min..self.max {
                for &j in order {
                    binary_orders.push(j > lower);
                }
            }
        }
        let orders = Binary {
            orders: binary_orders,
            elements: self.elements,
            orders_count: self.orders_count * (self.values() - 1),
        };
        debug_assert!(orders.valid());
        Ok(orders)
    }

    /// Turn every order into a binary order, where every value larger or equal to
    /// `n` becomes an approval.
    ///
    /// # Panics
    /// Will panic if n is not contained in `self.min..=self.max`.
    pub fn to_binary_cutoff(&self, n: usize) -> Result<Binary, &'static str> {
        debug_assert!(self.min <= n && n <= self.max);
        let mut binary_orders: Vec<bool> = Vec::new();
        binary_orders
            .try_reserve_exact(self.elements * self.orders_count)
            .or(Err("Could not allocate"))?;
        binary_orders.extend(self.orders.iter().map(|x| *x >= n));
        let orders =
            Binary { orders: binary_orders, elements: self.elements, orders_count: self.orders_count };
        debug_assert!(orders.valid());
        Ok(orders)
    }

    pub fn iter(&self) -> Chunks<usize> {
        self.orders.chunks(self.elements)
    }

    /// Fill the given preference matrix for the elements listed in `keep`.
    ///
    /// The middle row in the matrix will always be zero
    pub fn fill_preference_matrix(&self, keep: &[usize], matrix: &mut [usize]) {
        let l = keep.len();
        debug_assert!(l * l == matrix.len());
        for order in self.iter() {
            for i in 0..l {
                let ci = order[keep[i]];
                for j in (i + 1)..l {
                    let cj = order[keep[j]];

                    // TODO: What should the orientation of the matrix be?
                    if ci > cj {
                        matrix[i * l + j] += 1;
                    } else if cj > ci {
                        matrix[j * l + i] += 1;
                    }
                }
            }
        }
    }

    // Return whether element `a` was rated higher more times than `b`
    pub fn compare(&self, a: usize, b: usize) -> Ordering {
        debug_assert!(a < self.elements && b < self.elements);
        let mut a_v = 0;
        let mut b_v = 0;
        for order in self.iter() {
            match order[a].cmp(&order[b]) {
                Ordering::Greater => a_v += 1,
                Ordering::Less => b_v += 1,
                Ordering::Equal => {}
            }
        }
        a_v.cmp(&b_v)
    }

    // Return whether element `a` was rated `value` more times than `b`
    pub fn compare_specific(&self, a: usize, b: usize, value: usize) -> Ordering {
        debug_assert!(a < self.elements && b < self.elements);
        let mut a_v = 0;
        let mut b_v = 0;
        for order in self.iter() {
            if order[a] == value {
                a_v += 1;
            }
            if order[b] == value {
                b_v += 1;
            }
        }
        a_v.cmp(&b_v)
    }
}

impl Display for Cardinal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.orders_count {
            for j in 0..(self.elements - 1) {
                let v = self.orders[i * self.elements + j];
                write!(f, "{},", v)?;
            }
            let v_last = self.orders[i * self.elements + (self.elements - 1)];
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}

impl<'a> DenseOrders<'a> for Cardinal {
    type Order = &'a [usize];
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

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        unimplemented!();
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 || new_orders == 0 {
            return;
        }

        self.orders.reserve(new_orders);
        let dist = Uniform::from(self.min..=self.max);
        for _ in 0..new_orders {
            for _ in 0..self.elements {
                let i = dist.sample(rng);
                self.orders.push(i);
            }
        }
        self.orders_count += new_orders;
        debug_assert!(self.valid());
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use crate::tests::std_rng;

    use super::*;

    impl Arbitrary for Cardinal {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut orders_count, mut elements, mut min, mut max): (usize, usize, usize, usize) =
                Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            orders_count = orders_count % g.size();
            elements = elements % g.size();
            min = min % g.size();
            max = max % g.size();

            if min > max {
                std::mem::swap(&mut min, &mut max);
            }

            let mut orders = Cardinal::new(elements, min, max);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            orders
        }
    }

    #[quickcheck]
    fn kp_tranform_orders(cv: Cardinal) -> bool {
        match cv.kp_tranform() {
            Ok(bv) => bv.orders_count == cv.orders_count * (cv.values() - 1),
            Err(_) => true,
        }
    }
}
