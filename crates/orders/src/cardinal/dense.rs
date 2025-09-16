use std::{cmp::Ordering, iter::repeat_n, ops::RangeBounds};

use rand::distr::{Distribution, Uniform};

use super::{Cardinal, CardinalRef};
use crate::{DenseOrders, binary::BinaryDense, pairwise_lt};

#[derive(Debug, PartialEq, Eq)]
pub struct CardinalDense {
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
    pub(crate) min: usize,
    pub(crate) max: usize,
}

impl Clone for CardinalDense {
    fn clone(&self) -> Self {
        Self { orders: self.orders.clone(), elements: self.elements, min: self.min, max: self.max }
    }

    fn clone_from(&mut self, source: &Self) {
        self.orders.clone_from(&source.orders);
        self.elements = source.elements;
        self.min = source.min;
        self.max = source.max;
    }
}

pub enum MapError {
    Overflow,
    Underflow,
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

    pub fn min(&self) -> usize {
        self.min
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    #[cfg(test)]
    pub(crate) fn valid(&self) -> bool {
        if self.elements == 0 {
            self.orders.is_empty()
        } else if self.orders.len() % self.elements != 0 {
            false
        } else {
            for i in 0..self.len() {
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
    pub fn map_mul(&mut self, a: usize) -> Result<(), MapError> {
        if a == 1 {
            return Ok(());
        }
        let new_min = self.min.checked_mul(a).ok_or(MapError::Underflow)?;
        let new_max = self.max.checked_mul(a).ok_or(MapError::Overflow)?;
        for v in &mut self.orders {
            *v *= a;
        }
        self.min = new_min;
        self.max = new_max;
        Ok(())
    }

    /// Add to each order score a constant `a`, changing the `min` and `max`
    /// score.
    pub fn map_add(&mut self, a: usize) -> Result<(), MapError> {
        if a == 0 {
            return Ok(());
        }
        let new_min = self.min.checked_add(a).ok_or(MapError::Underflow)?;
        let new_max = self.max.checked_add(a).ok_or(MapError::Overflow)?;
        for v in &mut self.orders {
            *v += a;
        }
        self.min = new_min;
        self.max = new_max;
        Ok(())
    }

    /// Subtracts from each order score a constant `a`, changing the `min` and
    /// `max` score.
    pub fn map_sub(&mut self, a: usize) -> Result<(), MapError> {
        if a == 0 {
            return Ok(());
        }
        let new_min = self.min.checked_sub(a).ok_or(MapError::Underflow)?;
        let new_max = self.max.checked_sub(a).ok_or(MapError::Overflow)?;
        for v in &mut self.orders {
            *v -= a;
        }
        self.min = new_min;
        self.max = new_max;
        Ok(())
    }

    /// Number of valid values
    pub fn values(&self) -> usize {
        self.max - self.min + 1
    }

    /// The [Kotze-Pereira transformation](https://electowiki.org/wiki/Kotze-Pereira_transformation).
    #[doc(alias = "kotze")]
    pub fn kp_transform(&self) -> Result<BinaryDense, &'static str> {
        let mut binary_orders: Vec<bool> = Vec::new();
        let orders_size = self
            .elements
            .checked_mul(self.len())
            .ok_or("Number of orders would be too large")?
            .checked_mul(self.values() - 1)
            .ok_or("Number of orders would be too large")?;
        binary_orders.try_reserve_exact(orders_size).or(Err("Could not allocate"))?;
        for order in self.iter() {
            for i in self.min..self.max {
                for &j in order.values {
                    binary_orders.push(j > i);
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
    /// Will panic if `n` is not contained in `self.min..=self.max`.
    pub fn to_binary_cutoff(&self, n: usize) -> Result<BinaryDense, &'static str> {
        debug_assert!(self.min <= n && n <= self.max);
        let mut binary_orders: Vec<bool> = Vec::new();
        binary_orders
            .try_reserve_exact(self.elements * self.len())
            .or(Err("Could not allocate"))?;
        binary_orders.extend(self.orders.iter().map(|x| *x >= n));
        Ok(BinaryDense::new_from_parts(binary_orders, self.elements))
    }

    pub fn iter(&self) -> impl Iterator<Item = CardinalRef<'_>> {
        (0..self.len()).map(|i| self.get(i))
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
        assert!(a < self.elements && b < self.elements);
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
        assert!(a < self.elements && b < self.elements);
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

    pub fn sum(&self) -> Result<Cardinal, SumError> {
        let mut out: Vec<usize> = Vec::new();
        if out.try_reserve(self.elements).is_err() {
            return Err(SumError::Alloc);
        }
        out.extend(repeat_n(0, self.elements));
        if self.max.checked_mul(self.len()).is_none() {
            // If there's a chance that we overflow we'll have to check for it every
            // iteration.
            for order in self.iter() {
                debug_assert!(order.len() == self.elements);
                for (i, &v) in order.values().iter().enumerate() {
                    if let Some(res) = out[i].checked_add(v) {
                        out[i] = res;
                    } else {
                        return Err(SumError::Overflow);
                    }
                }
            }
        } else {
            for order in self.iter() {
                debug_assert!(order.len() == self.elements);
                for (i, &v) in order.values().iter().enumerate() {
                    out[i] += v;
                }
            }
        }

        Ok(Cardinal::new(out))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SumError {
    Alloc,
    Overflow,
}

impl<'a> DenseOrders<'a> for CardinalDense {
    type Order = CardinalRef<'a>;
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
            Some(CardinalRef::new(s))
        } else {
            None
        }
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
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
    fn kp_transform_orders(cv: CardinalDense) -> bool {
        match cv.kp_transform() {
            Ok(bv) => bv.len() == cv.len() * (cv.values() - 1),
            Err(_) => true,
        }
    }
}
