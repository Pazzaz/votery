//! A collection of different types of order formats
//!
//! Each order format consists of a struct which stores all the orders and
//! implements [`OrderFormat`].
//!
//! # Variations
//! When it comes to which specific format to use, there are some parts to
//! consider. One consideration is which type of order the use case would need.
//! Some voting methods require that each order creates a strict order of the
//! candidates while other ones limit each order to a single candidate. There
//! are also multiple voting formats with the same restrictions on orders, but
//! with different internal representations. There are two main considerations:
//! - Sparse vs Dense
//!     - Each order can either have a list containing every order, or a number
//!       specifying how many there are of every order.
//! - Possible inverse
//!     - Many orders can be seen as a function f: Candidate -> Ranking. This
//!       can be represented as an array of length |dom(f)| filed with numbers
//!       representing each candidates ranking. One could also use the "inverse"
//!       representation where we have a list of length |dom(f)| where each
//!       index is a rank and each element is a candidate which achieved that
//!       rank. The problem with this representation is that it's harder to
//!       represent ties, but it can be done by having auxiliary flags
//!       specifying which ranks contain multiple candidates.
//!
//! # Conversions

use rand::Rng;

// Lifetime needed because `Order` may be a reference which then needs a
// lifetime
pub trait DenseOrders<'a> {
    type Order;
    /// Number of elements
    fn elements(&self) -> usize;

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str>;

    /// Removes element from the orders, offsetting the other elements to
    /// take their place.
    fn remove_element(&mut self, target: usize) -> Result<(), &'static str>;

    /// Sample and add `new_orders` uniformly random orders for this format,
    /// using random numbers from `rng`.
    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_orders: usize);

    /// Treat each order as a partial ranking
    fn to_partial_ranking(self) -> TiedOrdersIncomplete;
}

pub mod soc;
pub mod soi;
pub mod toc;
pub mod toi;

mod binary;
pub use binary::Binary;
mod cardinal;
pub use cardinal::Cardinal;
mod specific;
pub use specific::Specific;
mod total_ranking;
pub use total_ranking::TotalRanking;

use self::toi::TiedOrdersIncomplete;

// Utility functions
fn remove_newline(buf: &mut String) {
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
}
