mod groups;
mod rank;
mod rank_ref;
mod tied_rank;
mod tied_rank_ref;

pub use groups::*;
pub use rank::*;
pub use rank_ref::*;
pub use tied_rank::*;
pub use tied_rank_ref::*;

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
