pub mod groups;
pub mod rank;
pub mod rank_ref;
pub mod tied_rank;
pub mod tied_rank_ref;

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
