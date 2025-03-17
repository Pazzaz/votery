use std::marker::PhantomData;

// Stores two slices, a: `&[usize]` and b: `&[bool]`, but only one len.
// We assume `a.len() == self.a_len` and `b.len() == self.a_len - 1`.
//
// Might be overoptimizing, ¯\_(ツ)_/¯
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
