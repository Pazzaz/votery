pub struct PartialOrder {
    // 2D matrix of length n*n, order[a*len + b] is `true` if a <= b
    order: Vec<bool>,
    len: usize,
}

fn is_partial_order(v: &[bool], len: usize) -> bool {
    if len*len != v.len() {
        return false;
    }
    for a in 0..len {
        if !v[a*len + a] {
            return false;
        }
        for c in a..len {
            if a == c { continue; }
            for b in a..c {
                if b == a { continue; }
                if v[a*len + b] && v[b*len + c] && !v[a*len + c] {
                    return false;
                }
            }
        }
    }
    true
}

impl PartialOrder {
    pub fn new(order: Vec<bool>, len: usize) -> Self {
        assert!(is_partial_order(&order, len));
        Self { order, len }
    }

    pub fn new_empty(n: usize) -> Self {
        let mut order = vec![false; n];
        for i in 0..n {
            order[i*n + i] = true;
        }
        Self { order, len: n }
    }

    pub unsafe fn new_unchecked(order: Vec<bool>, len: usize) -> Self {
        Self { order, len }
    }

    // Returns true if a <= b
    pub fn le(&self, a: usize, b: usize) -> bool {
        assert!(a < self.len && b < self.len);
        self.order[a*self.len + b]
    }

    pub fn eq(&self, a: usize, b: usize) -> bool {
        assert!(a < self.len && b < self.len);
        a == b || self.le(a, b) && self.le(b, a)
    }
}
