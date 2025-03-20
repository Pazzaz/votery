//! # Total orders
//!
//! A [total order][wp] is an order of all elements where every element is
//! comparable, ordered from largest to smallest. A chain is like a total order,
//! but only orders a subset of all elements.
//!
//! - [`Total`], an owned total order.
//! - [`TotalRef`], reference to a `Total`.
//! - [`TotalDense`], a collection of `Total`.
//! - [`Chain`], an owned total order of a subset of all elements.
//! - [`ChainRef`], reference to a `Chain`.
//! - [`ChainDense`], a collection of `Chain`.
//!
//! [wp]: https://en.wikipedia.org/wiki/Total_order

mod dense;
mod dense_incomplete;
mod strict;
mod strict_incomplete;
mod strict_incomplete_ref;
mod strict_ref;

pub use dense::*;
pub use dense_incomplete::*;
pub use strict::*;
pub use strict_incomplete::*;
pub use strict_incomplete_ref::*;
pub use strict_ref::*;
