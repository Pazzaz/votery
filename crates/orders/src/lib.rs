#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod formats;

fn pairwise_lt(v: &[usize]) -> bool {
    if v.len() >= 2 {
        for i in 0..(v.len() - 1) {
            if !(v[i] < v[i + 1]) {
                return false;
            }
        }
    }
    true
}

fn get_order<T: Ord>(v: &[T], reverse: bool) -> Vec<usize> {
    if v.is_empty() {
        return Vec::new();
    } else if v.len() == 1 {
        return vec![0];
    }

    let mut tmp: Vec<(usize, &T)> = Vec::with_capacity(v.len());
    for i in 0..v.len() {
        tmp.push((i, &v[i]));
    }
    tmp.sort_by(|a, b| (*a.1).cmp(b.1));
    if reverse {
        tmp.reverse();
    }
    let mut out = vec![0; v.len()];
    if let Some((b, bs)) = tmp.split_first_mut() {
        let mut current: &T = b.1;
        let mut i: usize = 0;
        for x in bs.iter_mut() {
            if *x.1 != *current {
                current = x.1;
                i += 1;
            }
            out[x.0] = i;
        }
    }
    out
}
