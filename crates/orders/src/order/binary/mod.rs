use super::{partial_order::PartialOrder, Order, OrderOwned, OrderRef};

pub struct Binary {
    values: Vec<bool>,
}

impl Binary {
    pub fn new(v: Vec<bool>) -> Self {
        Binary { values: v }
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

impl Order for Binary {
    fn elements(&self) -> usize {
        self.values.len()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    /// Convert to `PartialOrder`. We only know that `true` values are above `false` values, so
    /// those are the only relations that will be included in the result.
    fn as_partial(self) -> PartialOrder {
        let mut po = PartialOrder::new_empty(self.elements());
        for (i1, b1) in self.values.iter().enumerate() {
            for (i2, b2) in self.values[(i1+1)..].iter().enumerate() {
                match (b1, b2) {
                    // SAFETY: There won't be any transitive relations between elements, so we can
                    //         just set them directly.
                    (true, false) => unsafe { po.set_only(i2, i1) },
                    (false, true) => unsafe { po.set_only(i1, i2) },
                    (true, true) | (false, false) => {},
                }
            }
        }
        debug_assert!(po.valid());
        po
    }
}

impl<'a> OrderOwned<'a> for Binary {
    type Ref = BinaryRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        BinaryRef { values: &self.values }
    }
}

impl<'a> OrderRef for BinaryRef<'a> {
    type Owned = Binary;

    fn as_owned(self) -> Self::Owned {
        Binary { values: self.values.to_vec() }
    }
}