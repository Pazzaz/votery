// // TODO: How should we handle safety of these types? Should we check when we
// // create them and them have a "new_unchecked"?

// //! Different data structures which represents orderings of candidates
// //!
// //! There are several different related ways to rank a set of candidates.
// Most //! of these will probably not be useful, but they could be. The
// selection here //! is inspired by [PrefLib](https://www.preflib.org/)

// use std::fmt;

// /// A ranking where some indices can be assigned the same ranking.
// pub struct TiedOrder {
//     v: Vec<usize>,
// }

// /// TOI - Orders with Ties - Incomplete List
// ///
// /// Consider the following example:
// ///
// /// We have the candidates A, B, and C, which are numbered 1, 2 and 3,
// /// respectively. A vote may look like `3,2,1` and would then be a total
// order /// where 3 has the highest rank. But we can also express ties using
// the format /// `2,{1,3}` if we don't have any preference between 1 and 3.
// pub struct TiedOrderSparse {
//     // Each element represents which candidate they put in that position, and
// then the `bool` is     // `true` if it's tied with the next one So "2,{1,3}"
// would be [(2,false), (1,true),     // (3,false)]. The last `bool` is always
// `false`.     v: Vec<(usize, bool)>,
//     candidates: usize,
// }

// pub struct TotalOrder {
//     v: Vec<usize>,
// }

// pub struct TotalOrderSparse {
//     v: Vec<usize>,
//     candidates: usize,
// }

// pub struct BlankTiedOrder {
//     v: Vec<Option<usize>>,
// }

// pub struct BlankTotalOrder {
//     v: Vec<Option<usize>>,
// }

// impl TiedOrderSparse {
//     pub fn new(v: Vec<(usize, bool)>, candidates: usize) -> Option<Self> {
//         let mut seen = vec![false; candidates];
//         for (i, _) in &v {
//             if !(*i < candidates) || seen[*i] {
//                 return None;
//             } else {
//                 seen[*i] = true;
//             }
//         }
//         Some(TiedOrderSparse { v, candidates })
//     }

//     /// Like the normal FromStr, but we want to check if the vote respects
//     /// `candidates`.
//     pub fn from_str(s: &str, candidates: usize) -> Option<Self> {
//         let mut v = Vec::with_capacity(s.len() / 2);
//         let mut grouped = false;
//         for part in s.split(',') {
//             let bracket = if grouped { '}' } else { '{' };
//             let number = match part.strip_prefix(bracket) {
//                 Some(s) => {
//                     grouped = !grouped;
//                     s
//                 }
//                 None => part,
//             };
//             let n: usize = match number.parse() {
//                 Ok(n) => n,
//                 Err(_) => return None,
//             };
//             if !(n < candidates) {
//                 return None;
//             }
//             v.push((n, grouped));
//         }
//         // We didn't end our group
//         if grouped {
//             return None;
//         }
//         Some(TiedOrderSparse { v, candidates })
//     }
// }

// impl fmt::Display for TiedOrderSparse {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let mut grouped = false;
//         for (i, (k, g)) in self.v.iter().enumerate() {
//             match (grouped, g) {
//                 // Start of group
//                 (false, true) => write!(f, "{{{}", k)?,
//                 // End of group
//                 (true, false) => write!(f, "{}}}", k)?,
//                 (_, _) => write!(f, "{}", k)?,
//             };

//             if i != self.v.len() - 1 {
//                 write!(f, ",")?;
//                 grouped = *g;
//             }
//         }
//         Ok(())
//     }
// }

// impl TotalOrderSparse {
//     pub fn new(v: Vec<usize>, candidates: usize) -> Option<Self> {
//         let mut seen = vec![false; candidates];
//         for i in &v {
//             if !(*i < candidates) || seen[*i] {
//                 return None;
//             } else {
//                 seen[*i] = true;
//             }
//         }
//         Some(TotalOrderSparse { v, candidates })
//     }

//     /// Like the normal FromStr, but we want to check if the vote respects
//     /// `candidates`.
//     pub fn from_str(s: &str, candidates: usize) -> Option<Self> {
//         let mut v = Vec::with_capacity(s.len() / 2);
//         for number in s.split(',') {
//             let n: usize = match number.parse() {
//                 Ok(n) => n,
//                 Err(_) => return None,
//             };
//             if !(n < candidates) {
//                 return None;
//             }
//             v.push(n);
//         }
//         Some(TotalOrderSparse { v, candidates })
//     }
// }

// impl fmt::Display for TotalOrderSparse {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         for (i, k) in self.v.iter().enumerate() {
//             if i == self.v.len() - 1 {
//                 write!(f, "{}", k)?;
//             } else {
//                 write!(f, "{},", k)?;
//             }
//         }
//         Ok(())
//     }
// }

// impl TiedOrder {
//     pub fn new(v: Vec<usize>) -> Result<Self, &'static str> {
//         if v.len() == 0 {
//             return Ok(TiedOrder { v });
//         }
//         let mut found = vec![false; v.len()];
//         let mut max = 0;
//         for &i in &v {
//             if i > max {
//                 max = i;
//             }
//             if !found[i] {
//                 found[i] = true;
//             }
//         }
//         for i in 0..=max {
//             if !found[i] {
//                 return Err("Gaps found in TiedOrder");
//             }
//         }
//         Ok(TiedOrder { v })
//     }

//     pub fn max_level(&self) -> Option<usize> {
//         if self.v.is_empty() {
//             None
//         } else {
//             let mut max = 0;
//             for &i in &self.v {
//                 if i > max {
//                     max = i;
//                 }
//             }
//             Some(max)
//         }
//     }

//     pub fn len(&self) -> usize {
//         self.v.len()
//     }

//     /// Treat all candidates above level `n` as tied with level `n`
//     pub fn tie_above_level(&mut self, n: usize) {
//         for i in self.v.iter_mut() {
//             if *i > n {
//                 *i = n;
//             }
//         }
//     }
// }

// impl BlankTiedOrder {
//     pub fn new(v: Vec<Option<usize>>) -> Result<Self, &'static str> {
//         if v.len() == 0 {
//             return Ok(BlankTiedOrder { v });
//         }
//         let mut found = vec![false; v.len()];
//         let mut max = 0;
//         for &o in &v {
//             if let Some(i) = o {
//                 if i > max {
//                     max = i;
//                 }
//                 if !found[i] {
//                     found[i] = true;
//                 }
//             }
//         }
//         for i in 0..=max {
//             if !found[i] {
//                 return Err("Gaps found in TiedOrder");
//             }
//         }
//         Ok(BlankTiedOrder { v })
//     }

//     pub fn max_level(&self) -> Option<usize> {
//         let mut max = None;
//         for &o in &self.v {
//             if let Some(i) = o {
//                 match max {
//                     Some(j) => {
//                         if i > j {
//                             max = Some(i);
//                         }
//                     }
//                     None => max = Some(i),
//                 }
//             }
//         }
//         max
//     }
// }

// impl From<TotalOrder> for TiedOrder {
//     fn from(item: TotalOrder) -> Self {
//         TiedOrder { v: item.v }
//     }
// }

// impl From<TotalOrder> for BlankTotalOrder {
//     fn from(item: TotalOrder) -> Self {
//         let v = item.v.into_iter().map(|i| Some(i)).collect();
//         BlankTotalOrder { v }
//     }
// }

// impl From<TiedOrder> for BlankTiedOrder {
//     fn from(item: TiedOrder) -> Self {
//         let v = item.v.into_iter().map(|i| Some(i)).collect();
//         BlankTiedOrder { v }
//     }
// }

// impl From<BlankTotalOrder> for BlankTiedOrder {
//     fn from(item: BlankTotalOrder) -> Self {
//         BlankTiedOrder { v: item.v }
//     }
// }

// impl TryFrom<TiedOrder> for TotalOrder {
//     type Error = &'static str;

//     fn try_from(value: TiedOrder) -> Result<Self, Self::Error> {
//         match value.max_level() {
//             Some(i) if i == (value.v.len() - 1) => Ok(TotalOrder { v: value.v
// }),             None => {
//                 debug_assert!(value.v.is_empty());
//                 Ok(TotalOrder { v: Vec::new() })
//             }
//             _ => Err("TiedOrder is not a TotalOrder"),
//         }
//     }
// }

// impl TryFrom<BlankTiedOrder> for BlankTotalOrder {
//     type Error = &'static str;

//     fn try_from(value: BlankTiedOrder) -> Result<Self, Self::Error> {
//         match value.max_level() {
//             Some(i) if i == (value.v.len() - 1) => Ok(BlankTotalOrder { v:
// value.v }),             None => {
//                 debug_assert!(value.v.is_empty());
//                 Ok(BlankTotalOrder { v: Vec::new() })
//             }
//             _ => Err("BlankTiedOrder is not a BlankTotalOrder"),
//         }
//     }
// }
