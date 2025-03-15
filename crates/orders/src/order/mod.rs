pub mod binary;
pub mod cardinal;
pub mod partial_order;
pub mod rank;
pub mod tied_rank;


// Returns true iff all elements in `l` are different
pub(super) fn unique<T>(l: &[T]) -> bool
where
    T: std::cmp::PartialEq,
{
    for i in 0..l.len() {
        for j in 0..l.len() {
            if i == j {
                break;
            }
            if l[i] == l[j] {
                return false;
            }
        }
    }
    true
}

// Sort two arrays, sorted according to the values in `b`.
// Uses insertion sort
pub(crate) fn sort_using<A, B>(a: &mut [A], b: &mut [B])
where
    B: PartialOrd,
{
    debug_assert!(a.len() == b.len());
    let mut i: usize = 1;
    while i < b.len() {
        let mut j = i;
        while j > 0 && b[j - 1] > b[j] {
            a.swap(j, j - 1);
            b.swap(j, j - 1);
            j -= 1;
        }
        i += 1;
    }
}
