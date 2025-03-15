pub struct Binary {
    values: Vec<bool>,
}

impl Binary {
    pub fn new(v: Vec<bool>) -> Self {
        Binary { values: v }
    }

    pub fn as_ref(&self) -> BinaryRef {
        BinaryRef { values: &self.values }
    }
}

pub struct BinaryRef<'a> {
    values: &'a [bool],
}

impl<'a> BinaryRef<'a> {
    pub fn new(v: &'a [bool]) -> Self {
        BinaryRef { values: v }
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
