use std::cmp::Ordering;

use bool_matrix::MatrixBool;

use super::Order;

mod bool_matrix;

#[derive(Debug)]
pub struct PartialOrder {
    // 2D matrix of length n*n, order[a*len + b] is `true` if a ≤ b
    matrix: MatrixBool,
}

impl Clone for PartialOrder {
    fn clone(&self) -> Self {
        Self { matrix: self.matrix.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.matrix.clone_from(&source.matrix);
    }
}

impl PartialOrder {
    pub(crate) fn valid(&self) -> bool {
        self.matrix.is_partial_order()
    }

    pub fn new(order: Vec<bool>, elements: usize) -> Self {
        let matrix = MatrixBool::from_vec(order, elements);
        assert!(matrix.is_partial_order());
        Self { matrix }
    }

    pub fn new_empty(n: usize) -> Self {
        let mut matrix = MatrixBool::new(n);
        for i in 0..n {
            matrix[(i, i)] = true;
        }
        Self { matrix }
    }

    pub unsafe fn new_unchecked(order: Vec<bool>, elements: usize) -> Self {
        Self { matrix: MatrixBool::from_vec(order, elements) }
    }

    /// Returns true if and only if `a ≤ b`.
    #[must_use]
    pub fn le(&self, a: usize, b: usize) -> bool {
        assert!(a < self.elements() && b < self.elements());
        self.matrix[(a, b)]
    }

    pub fn eq(&self, a: usize, b: usize) -> bool {
        assert!(a < self.elements() && b < self.elements());
        a == b || self.le(a, b) && self.le(b, a)
    }

    pub fn add(&mut self, x: usize) {
        if x == 0 {
            return;
        }
        let orig_len = self.elements();
        self.matrix = self.matrix.add_rows(x);
        for i in orig_len..(orig_len + x) {
            self.matrix[(i, i)] = true;
        }
    }

    pub fn remove(&mut self, x: usize) {
        assert!(x < self.elements());
        self.matrix = self.matrix.remove_rows(x);
    }

    pub fn remove_subset(&mut self, x: &[usize]) {
        self.matrix = self.matrix.remove_rows_set(x);
    }

    /// Set `i ≤ j` and any transitive relations.
    pub fn set(&mut self, i: usize, j: usize) {
        assert!(i < self.elements() && j < self.elements());
        // Already done?
        if self.le(i, j) {
            return;
        }

        self.matrix[(i, j)] = true;
        // The transitive part
        // TODO: This feels wrong
        for ii in 0..self.elements() {
            for jj in 0..self.elements() {
                if self.le(ii, i) && self.le(j, jj) {
                    self.matrix[(ii, jj)] = true;
                }
            }
        }
    }

    pub fn ord(&self, i: usize, j: usize) -> Option<Ordering> {
        assert!(i < self.elements() && j < self.elements());
        if self.eq(i, j) {
            Some(Ordering::Equal)
        } else if self.le(i, j) {
            Some(Ordering::Less)
        } else if self.le(j, i) {
            Some(Ordering::Greater)
        } else {
            None
        }
    }

    pub fn set_ord(&mut self, i: usize, j: usize, o: Ordering) {
        assert!(i < self.elements() && j < self.elements());
        match o {
            Ordering::Less => self.set(i, j),
            Ordering::Equal => {
                self.set(i, j);
                self.set(j, i);
            }
            Ordering::Greater => self.set(j, i),
        }
    }

    #[must_use]
    pub fn combine(po1: &Self, po2: &Self) -> Self {
        assert!(po1.elements() == po2.elements());
        let mut po3 = po1.clone();
        po3.and_mut(po2);
        po3
    }

    pub fn and_mut(&mut self, other: &Self) {
        for i in 0..self.elements() {
            for j in 0..self.elements() {
                let v: bool = self.le(i, j) && other.le(i, j);
                self.matrix[(i, j)] = v;
            }
        }
    }

    // Partition the partial order into (at most) `x` categories, so that "larger"
    // values are in the earlier categories
    #[must_use]
    pub fn categorize(&self, x: usize) -> Vec<Vec<usize>> {
        if self.elements() == 0 || x == 0 {
            return Vec::new();
        }
        let category_size = self.elements().div_ceil(x);

        let mut objs: Vec<usize> = (0..self.elements()).collect();
        objs.sort_by(|&a, &b| self.ord(a, b).unwrap_or(Ordering::Equal));
        let mut switches = Vec::new();
        let mut i = 0;
        for xx in objs.windows(2) {
            i += 1;
            let a = xx[0];
            let b = xx[1];
            match self.ord(a, b).unwrap_or(Ordering::Equal) {
                Ordering::Greater => unreachable!(),
                Ordering::Equal => {}
                Ordering::Less => {
                    switches.push(i);
                }
            }
        }
        let mut category_ranges: Vec<(usize, usize)> = Vec::new();
        let mut curr_start = 0;
        for yy in switches.windows(2) {
            let aa = yy[0];
            let bb = yy[1];
            debug_assert!(aa < bb);
            debug_assert!(curr_start <= aa);
            debug_assert!(curr_start <= bb);
            let a_size = aa - curr_start;
            let b_size = bb - curr_start;

            // false = a, true = b
            let choose_b: bool = match (a_size.cmp(&category_size), b_size.cmp(&category_size)) {
                (Ordering::Less, Ordering::Less) => continue,
                (Ordering::Equal, Ordering::Equal) => unreachable!(),
                (Ordering::Greater, Ordering::Equal) => unreachable!(),
                (Ordering::Equal, Ordering::Less) => unreachable!(),
                (Ordering::Greater, Ordering::Less) => unreachable!(),
                (Ordering::Equal, Ordering::Greater) => false,
                (Ordering::Less, Ordering::Equal) => true,
                (Ordering::Greater, Ordering::Greater) => {
                    debug_assert!(a_size < b_size);
                    false
                }
                (Ordering::Less, Ordering::Greater) => {
                    // Which am I closer too?
                    let a_dist = category_size - a_size;
                    let b_dist = b_size - category_size;
                    match a_dist.cmp(&b_dist) {
                        Ordering::Less => false,
                        Ordering::Equal => false,
                        Ordering::Greater => true,
                    }
                }
            };
            if choose_b && curr_start != bb {
                category_ranges.push((curr_start, bb));
                curr_start = bb;
            } else if curr_start != aa {
                category_ranges.push((curr_start, aa));
                curr_start = aa;
            }
            if category_ranges.len() == x {
                break;
            }
        }

        if category_ranges.len() < x && curr_start != objs.len() {
            category_ranges.push((curr_start, objs.len()));
        }

        category_ranges.into_iter().map(|(start, end)| objs[start..end].to_vec()).collect()
    }
}

impl Order for PartialOrder {
    fn elements(&self) -> usize {
        self.matrix.dim
    }

    fn len(&self) -> usize {
        self.matrix.dim
    }

    fn to_partial(self) -> PartialOrder {
        self
    }
}

/// Like `PartialOrder` but transitive relations may not be set. Created using
/// [`PartialOrder::to_manual`].
pub(crate) struct PartialOrderManual {
    matrix: MatrixBool,
}

impl PartialOrderManual {
    pub(crate) fn elements(&self) -> usize {
        self.matrix.dim
    }

    pub(crate) fn new(n: usize) -> Self {
        let mut matrix = MatrixBool::new(n);
        for i in 0..n {
            matrix[(i, i)] = true;
        }
        Self { matrix }
    }

    /// Set only `i ≤ j`, without setting transitive relations.
    pub(crate) fn set(&mut self, i: usize, j: usize) {
        assert!(i < self.elements() && j < self.elements());
        self.matrix[(i, j)] = true;
    }

    pub fn set_ord(&mut self, i: usize, j: usize, o: Ordering) {
        assert!(i < self.elements() && j < self.elements());
        match o {
            Ordering::Less => self.set(i, j),
            Ordering::Equal => {
                self.set(i, j);
                self.set(j, i);
            }
            Ordering::Greater => self.set(j, i),
        }
    }

    pub(crate) fn finish(mut self) -> PartialOrder {
        let mut updated = true;
        while updated {
            updated = false;
            for i in 0..self.elements() {
                for k in 0..self.elements() {
                    for j in 0..self.elements() {
                        if self.matrix[(i, j)] && self.matrix[(j, k)] && !self.matrix[(i, k)] {
                            self.matrix[(i, k)] = true;
                            updated = true;
                        }
                    }
                }
            }
        }
        PartialOrder { matrix: self.matrix }
    }

    /// Convert to `PartialOrder`.
    ///
    /// # Safety
    ///
    /// All transitive relations have to be set.
    pub(crate) unsafe fn finish_unchecked(self) -> PartialOrder {
        PartialOrder { matrix: self.matrix }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use quickcheck::Arbitrary;

    use super::{PartialOrder, PartialOrderManual};
    use crate::Order;

    impl Arbitrary for PartialOrder {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut po = PartialOrderManual::new(g.size());
            for i in 0..po.elements() {
                for j in 0..po.elements() {
                    if Arbitrary::arbitrary(g) {
                        po.set(i, j);
                    }
                }
            }
            po.finish()
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let pp = self.clone();
            Box::new((0..self.elements()).map(move |x| {
                let mut co = pp.clone();
                let v: Vec<usize> = (0..=x).collect();
                co.remove_subset(&v);
                co
            }))
        }
    }

    #[test]
    fn empty_equal() {
        let po = PartialOrder::new_empty(123);
        for i in 0..po.elements() {
            match po.ord(i, i) {
                Some(Ordering::Equal) => {}
                _ => panic!(),
            }
        }
    }

    #[quickcheck]
    fn po_valid_gen(po: PartialOrder) -> bool {
        po.valid()
    }

    #[quickcheck]
    fn po_add_combine(mut po1: PartialOrder, mut po2: PartialOrder) -> bool {
        let l1 = po1.elements();
        let l2 = po2.elements();
        match l1.cmp(&l2) {
            Ordering::Less => {
                po1.add(l2 - l1);
                if !po1.valid() {
                    return false;
                }
            }
            Ordering::Greater => {
                po2.add(l1 - l2);
                if !po2.valid() {
                    return false;
                }
            }
            Ordering::Equal => {}
        }
        let po3 = PartialOrder::combine(&po1, &po2);
        po3.valid()
    }

    #[quickcheck]
    fn po_categorize(po: PartialOrder, x: usize) -> bool {
        let cats = x % po.elements();
        let vv = po.categorize(cats);
        vv.len() <= cats
    }

    #[quickcheck]
    fn add_remove(po: PartialOrder, x: usize) -> bool {
        if po.elements() == 0 {
            return true;
        }
        let mut poc = po.clone();
        let a = x % poc.elements();
        poc.add(a);
        if !poc.valid() {
            return false;
        }
        poc.remove(a);
        poc.valid()
    }

    // FIXME
    #[quickcheck]
    fn po_categorize_one(po: PartialOrder) -> bool {
        if po.elements() == 0 {
            return true;
        }
        let vv = po.categorize(1);
        vv.len() == 1 && vv[0].len() == po.elements()
    }
}
