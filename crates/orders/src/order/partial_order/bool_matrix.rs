use std::ops::{Index, IndexMut};

#[derive(Debug, PartialEq, Eq, Default)]
pub(crate) struct MatrixBool {
    pub(crate) dim: usize,
    pub(crate) elements: Vec<bool>,
}

impl Clone for MatrixBool {
    fn clone(&self) -> Self {
        Self { dim: self.dim, elements: self.elements.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.dim = source.dim;
        self.elements.clone_from(&source.elements);
    }
}

impl MatrixBool {
    #[must_use]
    pub fn new(dim: usize) -> Self {
        Self { dim, elements: vec![false; dim * dim] }
    }

    pub fn from_vec(elements: Vec<bool>, dim: usize) -> Self {
        assert!(dim * dim == elements.len());
        Self { elements, dim }
    }

    pub fn add_rows(&self, x: usize) -> Self {
        let mut new_matrix = MatrixBool::new(self.dim + x);
        for y in 0..self.dim {
            for x in 0..self.dim {
                new_matrix[(x, y)] = self[(x, y)];
            }
        }
        new_matrix
    }

    pub fn remove_rows(&self, x: usize) -> Self {
        debug_assert!(x <= self.dim);
        let mut new_matrix = MatrixBool::new(self.dim - x);
        for y in 0..(self.dim - x) {
            for x in 0..(self.dim - x) {
                new_matrix[(x, y)] = self[(x, y)];
            }
        }
        new_matrix
    }

    // Remove has to be sorted
    #[must_use]
    pub fn remove_rows_set(&self, remove: &[usize]) -> Self {
        debug_assert!(!remove.is_empty());
        debug_assert!(remove.is_sorted());
        debug_assert!(is_subset(self.dim, remove));
        let mut skipped = vec![false; self.dim];
        for &i in remove {
            skipped[i] = true;
        }
        let mut map = vec![0; self.dim - remove.len()];
        let mut j = 0;
        for (i, skip) in skipped.iter().enumerate() {
            if !skip {
                map[j] = i;
                j += 1;
            }
        }
        debug_assert!(self.dim - remove.len() == j);
        let mut new_matrix = MatrixBool::new(j);
        for y in 0..j {
            for x in 0..j {
                new_matrix[(x, y)] = self[(map[x], map[y])];
            }
        }

        new_matrix
    }

    pub fn is_partial_order(&self) -> bool {
        for a in 0..self.dim {
            if !self[(a, a)] {
                return false;
            }
            for c in 0..self.dim {
                if a == c {
                    continue;
                }
                for b in 0..self.dim {
                    if b == a || b == c {
                        continue;
                    }
                    if self[(a, b)] && self[(b, c)] && !self[(a, c)] {
                        return false;
                    }
                }
            }
        }
        true
    }
}

fn is_subset(max: usize, sorted_set: &[usize]) -> bool {
    if max <= sorted_set[0] {
        return false;
    }
    for i in 1..sorted_set.len() {
        if sorted_set[i] == sorted_set[i - 1] || max <= sorted_set[i] {
            return false;
        }
    }
    true
}

impl Index<(usize, usize)> for MatrixBool {
    type Output = bool;

    fn index(&self, i: (usize, usize)) -> &Self::Output {
        self.elements.get(i.0 + self.dim * i.1).unwrap()
    }
}

impl IndexMut<(usize, usize)> for MatrixBool {
    fn index_mut(&mut self, i: (usize, usize)) -> &mut Self::Output {
        self.elements.get_mut(i.0 + self.dim * i.1).unwrap()
    }
}
