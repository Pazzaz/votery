use rand::seq::SliceRandom;

/// SOC - Strict Orders - Complete List
///
/// A packed list of complete strict orders, with related methods. Each order is
/// a permutation of the elements
#[derive(Clone, Debug)]
pub struct StrictOrdersComplete {
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
}

impl StrictOrdersComplete {
    pub fn new(elements: usize) -> Self {
        StrictOrdersComplete { orders: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn add(&mut self, order: &[usize]) {
        debug_assert!(order.len() == self.elements);
        self.orders.reserve(self.elements);
        let mut seen = vec![false; self.elements];
        for &i in order {
            debug_assert!(i < self.elements || !seen[i]);
            seen[i] = true;
            self.orders.push(i);
        }
        debug_assert!(self.valid());
    }

    pub fn orders(&self) -> usize {
        debug_assert!(self.orders.len() % self.elements == 0);
        self.orders.len() / self.elements
    }

    /// Return true if it was a valid order.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut order = Vec::with_capacity(self.elements);
        let mut seen = vec![false; self.elements];
        for number in s.split(',') {
            let i: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if i >= self.elements || seen[i] {
                return false;
            }
            seen[i] = true;
            order.push(i);
        }
        if order.len() != self.elements {
            return false;
        }
        self.add(&order);
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        for order in self {
            let mut seen = vec![false; self.elements];
            for &i in order {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    pub fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 {
            return;
        }
        let v: &mut [usize] = &mut (0..self.elements).collect::<Vec<usize>>();
        self.orders.reserve(self.elements * new_orders);
        for _ in 0..new_orders {
            v.shuffle(rng);
            for &el in &*v {
                self.orders.push(el);
            }
        }
        debug_assert!(self.valid());
    }
}

impl<'a> IntoIterator for &'a StrictOrdersComplete {
    type Item = &'a [usize];
    type IntoIter = StrictOrdersCompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StrictOrdersCompleteIterator { orig: self, i: 0 }
    }
}

pub struct StrictOrdersCompleteIterator<'a> {
    orig: &'a StrictOrdersComplete,
    i: usize,
}

impl<'a> Iterator for StrictOrdersCompleteIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.orig.elements;
        let start = self.i * self.orig.elements;
        let order = &self.orig.orders[start..(start + len)];
        self.i += 1;
        Some(order)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.orders() - self.i;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for StrictOrdersCompleteIterator<'_> {}
