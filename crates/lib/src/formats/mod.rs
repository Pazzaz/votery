//! A collection of different types of vote formats
//!
//! Each vote format consists of a struct which stores all the votes and
//! implements [`VoteFormat`].

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

pub mod soc;
pub mod soi;
pub mod toc;
pub mod toi;
pub mod orders;

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
