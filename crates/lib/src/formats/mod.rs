//! A collection of different types of vote formats
//!
//! Each vote format consists of a struct which stores all the votes and
//! implements [`VoteFormat`].
//!
//! # Variations
//! When it comes to which specific format to use, there are some parts to
//! consider. One consideration is whcih type of vote the use case would need.
//! Some voting methods require that each vote creates a strict order of the
//! candidates while other ones limit each vote to a single candidate. There are
//! also multiple voting formats with the same restrictions on votes, but with
//! different internal representations. There are two main considerations:
//! - Sparse vs Dense
//!     - Each vote can either have a list containing every vote, or a number
//!       specifying how many there are of every vote.
//! - Possible inverse
//!     - Many votes can be seen as a function f: Candidate -> Ranking. This can
//!       be represented as an array of length |dom(f)| filed with numbers
//!       representing each candidates ranking. One could also use the "inverse"
//!       representation where we have a list of length |dom(f)| where each
//!       index is a rank and each element is a candidate which achieved that
//!       rank. The problem with this representation is that it's harder to
//!       represent ties, but it can be done by having auxiliary flags
//!       specifying which ranks contain multiple candidates.
//!
//! # Conversions

use rand::Rng;

// Lifetime needed because `Vote` may be a reference which then needs a lifetime
pub trait VoteFormat<'a> {
    type Vote;
    /// List the number of candidates
    fn candidates(&self) -> usize;

    /// Add more votes from `f`
    // fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str>;

    fn add(&mut self, v: Self::Vote) -> Result<(), &'static str>;

    /// Removes candidate from the votes, offsetting the other candidates to
    /// take their place.
    fn remove_candidate(&mut self, targets: usize) -> Result<(), &'static str>;

    /// Sample and add `new_voters` uniformly random votes for this format,
    /// using random numbers from `rng`.
    fn generate_uniform<R: Rng>(&mut self, rng: &mut R, new_voters: usize);

    /// Treat each vote as a partial ranking
    fn to_partial_ranking(self) -> TiedOrdersIncomplete;
}

pub mod orders;
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

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};
    use rand::{rngs::StdRng, SeedableRng};

    // `Gen` contains a rng, but it's a private member so this method is used to get
    // a standard rng generated from `Gen`
    pub fn std_rng(g: &mut Gen) -> StdRng {
        let mut seed = [0u8; 32];
        for i in 0..32 {
            seed[i] = Arbitrary::arbitrary(g);
        }
        StdRng::from_seed(seed)
    }
}
