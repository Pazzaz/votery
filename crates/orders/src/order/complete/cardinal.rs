use super::Binary;

pub struct Cardinal {
    values: Vec<usize>,
}

impl Cardinal {
    pub fn new(v: Vec<usize>) -> Self {
        Cardinal { values: v }
    }

    pub fn remove(&mut self, n: usize) {
        self.values.remove(n);
    }
}

pub struct CardinalRef<'a> {
    values: &'a [usize],
}


impl CardinalRef<'_> {

    /// Returns the number of elements in the order
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to binary order, where any value less than `cutoff` becomes `false` and larger becomes `true`.
    pub fn to_binary(&self, cutoff: usize) -> Binary {
        let values = self.values.iter().map(|x| *x >= cutoff).collect();
        Binary::new(values)
    }
}
