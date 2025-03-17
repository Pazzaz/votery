use std::{
    cmp::Ordering,
    fmt::{self, Display},
    io::BufRead,
    ops::RangeBounds,
};

use rand::distr::{Distribution, Uniform};

use super::CardinalRef;
use crate::{DenseOrders, binary::BinaryDense, pairwise_lt, remove_newline};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardinalDense {
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
    pub(crate) min: usize,
    pub(crate) max: usize,
}

impl CardinalDense {
    pub fn new<R: RangeBounds<usize>>(elements: usize, range: R) -> CardinalDense {
        let min = match range.start_bound() {
            std::ops::Bound::Included(&x) => x,
            std::ops::Bound::Excluded(&x) => x + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let max = match range.end_bound() {
            std::ops::Bound::Included(&x) => x,
            std::ops::Bound::Excluded(&x) => x - 1,
            std::ops::Bound::Unbounded => usize::MAX,
        };
        debug_assert!(min <= max);
        CardinalDense { orders: Vec::new(), elements, min, max }
    }

    pub fn count(&self) -> usize {
        if self.elements == 0 { 0 } else { self.orders.len() / self.elements }
    }

    pub fn min(&self) -> usize {
        self.min
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn get(&self, i: usize) -> Option<CardinalRef> {
        if i < self.count() {
            let start = i * self.elements;
            let end = (i + 1) * self.elements;
            let s = &self.orders[start..end];
            Some(CardinalRef::new(s))
        } else {
            None
        }
    }

    pub(crate) fn valid(&self) -> bool {
        if self.elements == 0 {
            self.orders.len() == 0
        } else if self.orders.len() % self.elements != 0 {
            false
        } else {
            for i in 0..self.count() {
                for j in 0..self.elements {
                    let v = self.orders[self.elements * i + j];
                    if v < self.min || v > self.max {
                        return false;
                    }
                }
            }
            true
        }
    }

    /// Multiply each order score with constant `a`, changing the `min` and
    /// `max` score.
    pub fn mul(&mut self, a: usize) {
        if a == 1 {
            return;
        }
        let new_min = self.min.checked_mul(a).unwrap();
        let new_max = self.max.checked_mul(a).unwrap();
        for v in &mut self.orders {
            *v *= a;
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
        for v in &mut self.orders {
            *v += a;
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
        for v in &mut self.orders {
            *v -= a;
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
            match count.cmp(&self.elements) {
                Ordering::Greater => return Err("Too many elements listed in order"),
                Ordering::Less => return Err("Too few elements listed in order"),
                Ordering::Equal => {}
            }
        }
        debug_assert!(self.valid());
        Ok(())
    }

    /// Number of valid values
    pub fn values(&self) -> usize {
        self.max - self.min + 1
    }

    /// The Kotze-Pereira transformation
    pub fn kp_tranform(&self) -> Result<BinaryDense, &'static str> {
        let mut binary_orders: Vec<bool> = Vec::new();
        let orders_size = self
            .elements
            .checked_mul(self.count())
            .ok_or("Number of orders would be too large")?
            .checked_mul(self.values() - 1)
            .ok_or("Number of orders would be too large")?;
        binary_orders.try_reserve_exact(orders_size).or(Err("Could not allocate"))?;
        for i in 0..self.count() {
            let order = &self.orders[i * self.elements..(i + 1) * self.elements];
            for lower in self.min..self.max {
                for &j in order {
                    binary_orders.push(j > lower);
                }
            }
        }
        Ok(BinaryDense::new_from_parts(binary_orders, self.elements))
    }

    /// Turn every order into a binary order, where every value larger or equal
    /// to `n` becomes an approval.
    ///
    /// # Panics
    ///
    /// Will panic if n is not contained in `self.min..=self.max`.
    pub fn to_binary_cutoff(&self, n: usize) -> Result<BinaryDense, &'static str> {
        debug_assert!(self.min <= n && n <= self.max);
        let mut binary_orders: Vec<bool> = Vec::new();
        binary_orders
            .try_reserve_exact(self.elements * self.count())
            .or(Err("Could not allocate"))?;
        binary_orders.extend(self.orders.iter().map(|x| *x >= n));
        Ok(BinaryDense::new_from_parts(binary_orders, self.elements))
    }

    pub fn iter(&self) -> impl Iterator<Item = CardinalRef> {
        (0..self.count()).map(|i| self.get(i).unwrap())
    }

    /// Fill the given preference matrix for the elements listed in `keep`.
    ///
    /// The middle row in the matrix will always be zero
    pub fn fill_preference_matrix(&self, keep: &[usize], matrix: &mut [usize]) {
        let l = keep.len();
        debug_assert!(l * l == matrix.len());
        for v in self.iter() {
            for i in 0..l {
                let ci = v.values[keep[i]];
                for j in (i + 1)..l {
                    let cj = v.values[keep[j]];

                    // TODO: What should the orientation of the matrix be?
                    match ci.cmp(&cj) {
                        Ordering::Greater => matrix[i * l + j] += 1,
                        Ordering::Less => matrix[j * l + i] += 1,
                        Ordering::Equal => {}
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
        for v in self.iter() {
            match v.values[a].cmp(&v.values[b]) {
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
        for v in self.iter() {
            if v.values[a] == value {
                a_v += 1;
            }
            if v.values[b] == value {
                b_v += 1;
            }
        }
        a_v.cmp(&b_v)
    }
}

impl Display for CardinalDense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.count() {
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

impl<'a> DenseOrders<'a> for CardinalDense {
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
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.count() {
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
        self.orders.truncate(self.count() * new_elements);
        self.elements = new_elements;
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 || new_orders == 0 {
            return;
        }

        self.orders.reserve(new_orders);
        let dist = Uniform::new_inclusive(self.min, self.max).unwrap();
        for _ in 0..new_orders {
            for _ in 0..self.elements {
                let i = dist.sample(rng);
                self.orders.push(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for CardinalDense {
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

            let mut orders = CardinalDense::new(elements, min..=max);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            orders
        }
    }

    #[quickcheck]
    fn kp_tranform_orders(cv: CardinalDense) -> bool {
        match cv.kp_tranform() {
            Ok(bv) => bv.count() == cv.count() * (cv.values() - 1),
            Err(_) => true,
        }
    }
}
