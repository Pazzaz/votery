//! This is a crate used to study and use different types of voting methods.
//!
//! **This crate is currently work in progress, and is not suitable for any
//! purpose, at any time, anywhere**
//!
//! Example usage:
//! ```
//! use votery::prelude::*;
//! use votery::methods::Approval;
//! use orders::formats::Binary;
//! use votery::methods::VotingMethod;
//!
//! let mut votes = Binary::new(3);
//! votes.add(&[false, true, true]);
//! votes.add(&[true, false, false]);
//! votes.add(&[true, true, false]);
//!
//! let count = Approval::count(&votes).unwrap().get_order();
//! assert_eq!(count, &[0, 0, 1]);
//! ```
#![feature(option_zip)]
#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod generators;
pub mod methods;


pub enum Winner {
    Solo(usize),
    Ties(Vec<usize>),
}

/// Commonly used traits
pub mod prelude {
    pub use orders::dense::DenseOrders;
}

pub use orders;

pub fn single_winner(ranking: &Vec<usize>) -> Winner {
    let mut winners = Vec::with_capacity(1);
    for i in 0..ranking.len() {
        if ranking[i] == 0 {
            winners.push(i);
        }
    }
    match winners.len() {
        0 => panic!("Single winner had no winner"),
        1 => Winner::Solo(winners[0]),
        _ => Winner::Ties(winners),
    }
}

// Test if list is strictly ordered from largest to smallest
// fn pairwise_gt(v: &[usize]) -> bool {
//     if v.len() >= 2 {
//         for i in 0..(v.len() - 1) {
//             if !(v[i] > v[i + 1]) {
//                 return false;
//             }
//         }
//     }
//     true
// }

pub mod tarjan;
