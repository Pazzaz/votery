use std::marker::PhantomData;

/// Stores two slices, a: `&[usize]` and b: `&[bool]`, but only one len.
/// We assume `a.len() == self.a_len` and `b.len() == self.a_len - 1`.
///
/// It does not guarantee anything about the slices' contents.
#[derive(Debug, Clone, Copy)]
pub(super) struct SplitRef<'a> {
    a_len: usize,
    a: *const usize,
    b: *const bool,
    phantom: PhantomData<(&'a usize, &'a bool)>,
}

impl<'a> SplitRef<'a> {
    pub fn new(a: &'a [usize], b: &'a [bool]) -> SplitRef<'a> {
        let a_len = a.len();
        let b_len = a_len.saturating_sub(1);
        assert!(b.len() == b_len);
        SplitRef { a_len, a: a.as_ptr(), b: b.as_ptr(), phantom: PhantomData }
    }

    pub fn a<'b>(self: &'b SplitRef<'a>) -> &'a [usize] {
        unsafe { std::slice::from_raw_parts(self.a, self.a_len) }
    }

    pub fn b<'b>(self: &'b SplitRef<'a>) -> &'a [bool] {
        let b_len = self.a_len.saturating_sub(1);
        unsafe { std::slice::from_raw_parts(self.b, b_len) }
    }
}

impl PartialEq for SplitRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.a_len == other.a_len && self.a() == other.a() && self.b() == other.b()
    }
}

impl Eq for SplitRef<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let a = Vec::new();
        let b = Vec::new();
        let s = SplitRef::new(&a, &b);
        assert!(s.a().is_empty());
        assert!(s.b().is_empty());
    }

    #[test]
    fn single() {
        let a = vec![0];
        let b = Vec::new();
        let s = SplitRef::new(&a, &b);
        assert!(s.a().len() == 1);
        assert!(s.b().is_empty());
    }

    #[test]
    fn two() {
        let a = vec![1, 4241];
        let b = vec![false];
        let s = SplitRef::new(&a, &b);
        assert!(s.a().len() == 2);
        assert!(s.b().len() == 1);
    }

    #[test]
    fn long() {
        let a: [usize; 7] = [1, 4241, 4, 564, 233, 7, 2];
        let b: [bool; 6] = [false, true, false, true, true, false];
        let s = SplitRef::new(&a, &b);
        for aa in a.iter().zip(s.a().iter()) {
            assert!(aa.0 == aa.1);
        }
        for bb in b.iter().zip(s.b().iter()) {
            assert!(bb.0 == bb.1);
        }
        assert!(s.a().len() == a.len());
        assert!(s.b().len() == b.len());
    }
}
