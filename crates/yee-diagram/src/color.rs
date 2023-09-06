use votery::formats::orders::TiedRankRef;

// Normal RGB color
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Color {
    values: [f64; 3],
}

///
#[derive(Clone, Copy)]
pub enum VoteColorBlending {
    /// The average of the winners of a vote
    Winners,
    /// The average of all ranked candidates, weighted according to it's group.
    /// The winners get the weight 1/1, second place gets 1/2, etc.
    Harmonic,
}

pub const BLACK: Color = Color { values: [0.0, 0.0, 0.0] };

impl Color {
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        let c = Color { values: [r, g, b] };
        debug_assert!(c.is_valid());
        c
    }

    fn is_valid(&self) -> bool {
        0.0 <= self.r()
            && self.r() <= 255.0
            && 0.0 <= self.g()
            && self.g() <= 255.0
            && 0.0 <= self.b()
            && self.b() <= 255.0
    }

    pub fn bw(x: usize, max: usize) -> Self {
        let v = 255.0 * x as f64 / max as f64;
        Color::new(v, v, v)
    }

    pub const fn r(&self) -> f64 {
        self.values[0]
    }

    pub const fn g(&self) -> f64 {
        self.values[1]
    }

    pub const fn b(&self) -> f64 {
        self.values[2]
    }

    // TODO: Is there some other way to do
    // perceptual color distance? Should I really be using euclidean distance?
    pub fn dist(&self, b: &Color) -> f64 {
        let [ai, bi, ci] = self.values;
        let [aj, bj, cj] = b.values;
        ((ai - aj).powi(2) + (bi - bj).powi(2) + (ci - cj).powi(2)).sqrt()
    }

    pub fn quantize(&self) -> [u8; 3] {
        debug_assert!(self.is_valid());
        [self.r() as u8, self.g() as u8, self.b() as u8]
    }

    fn to_srgb(&self) -> [f64; 3] {
        fn f(u: f64) -> f64 {
            ((u + 0.055) / 1.055).powf(2.4)
        }
        [f(self.r()), f(self.g()), f(self.b())]
    }

    fn from_srgb([r, g, b]: [f64; 3]) -> Self {
        fn f_inv(u: f64) -> f64 {
            let res = (1.055 * (u.powf(1.0 / 2.4))) - 0.055;
            res.clamp(0.0, 255.0)
        }
        Color::new(f_inv(r), f_inv(g), f_inv(b))
    }

    pub fn from_str_checked(s: &str) -> Result<Color, &'static str> {
        if s.len() != 7 {
            return Err("Wrong length RGB code encountered while parsing");
        }
        let rest = s.strip_prefix('#').ok_or(r##"Did not start with "#""##)?;
        let rstr = rest.get(0..2).ok_or("Could not parse RGB")?;
        let r = usize::from_str_radix(rstr, 16).or(Err("Not hexadecimal"))?;
        let gstr = rest.get(2..4).ok_or("Could not parse RGB")?;
        let g = usize::from_str_radix(gstr, 16).or(Err("Not hexadecimal"))?;
        let bstr = rest.get(4..6).ok_or("Could not parse RGB")?;
        let b = usize::from_str_radix(bstr, 16).or(Err("Not hexadecimal"))?;
        Ok(Color::new(r as f64, g as f64, b as f64))
    }

    // Panic if `s` is not a valid hexadecimal color code.
    const fn from_str(s: &str) -> Color {
        assert!(s.len() == 7);
        let s_bytes = s.as_bytes();
        assert!(s_bytes[0] == b'#');
        let ra = unwrap((s_bytes[1] as char).to_digit(16));
        let rb = unwrap((s_bytes[2] as char).to_digit(16));
        let ga = unwrap((s_bytes[3] as char).to_digit(16));
        let gb = unwrap((s_bytes[4] as char).to_digit(16));
        let ba = unwrap((s_bytes[5] as char).to_digit(16));
        let bb = unwrap((s_bytes[6] as char).to_digit(16));
        let r = ra * 16 + rb;
        let g = ga * 16 + gb;
        let b = ba * 16 + bb;
        Color { values: [r as f64, g as f64, b as f64] }
    }

    pub const fn dutch_field(n: usize) -> Color {
        const DUTCH_FIELD_LEN: usize = 9;
        assert!(n < DUTCH_FIELD_LEN);
        const DUTCH_FIELD: [&'static str; DUTCH_FIELD_LEN] = [
            "#e60049", "#0bb4ff", "#50e991", "#e6d800", "#9b19f5", "#ffa300", "#dc0ab4", "#b3d4ff",
            "#00bfa0",
        ];

        // We convert the list of strings to colors at compile time, so this function
        // should just be an array lookup
        const DUTCH_FIELD_COLORS: [Color; DUTCH_FIELD_LEN] = {
            let mut tmp = [BLACK; DUTCH_FIELD_LEN];
            let mut i = 0;
            while i < DUTCH_FIELD_LEN {
                tmp[i] = Color::from_str(DUTCH_FIELD[i]);
                i += 1;
            }
            tmp
        };
        DUTCH_FIELD_COLORS[n]
    }

    /// Turn a vote into a color.
    pub fn from_vote(vote_color: VoteColorBlending, vote: TiedRankRef, colors: &[Color]) -> Color {
        match vote_color {
            VoteColorBlending::Harmonic => {
                let mut mixes: Vec<Color> = Vec::new();
                let mut weights: Vec<f64> = Vec::new();
                for (gi, group) in vote.iter_groups().enumerate() {
                    let mut hmm = Vec::new();
                    for &i in group {
                        debug_assert!(i < colors.len());
                        hmm.push(colors[i]);
                    }
                    let new_c = blend_colors(hmm.iter());
                    mixes.push(new_c);
                    weights.push(1.0 / (gi + 1) as f64)
                }
                blend_colors_weighted(mixes.iter(), Some(&weights))
            }
            VoteColorBlending::Winners => {
                let i_colors = vote.winners().iter().map(|&i| &colors[i]);
                blend_colors(i_colors)
            }
        }
    }
}

// Used instead of Option::unwrap in const contexts
const fn unwrap<X>(o: Option<X>) -> X
where
    X: Copy,
{
    match o {
        Some(x) => x,
        None => unreachable!(),
    }
}

pub fn blend_colors<'a, I>(cs: I) -> Color
where
    I: Iterator<Item = &'a Color>,
{
    blend_colors_weighted(cs, None)
}

pub fn blend_colors_weighted<'a, I>(cs: I, ws: Option<&[f64]>) -> Color
where
    I: Iterator<Item = &'a Color>,
{
    let mut rr = 0.0;
    let mut gg = 0.0;
    let mut bb = 0.0;
    let mut total = 0.0;
    for (i, rgb) in cs.enumerate() {
        let weight = match ws {
            Some(v) => v[i],
            None => 1.0,
        };
        let [sr, sg, sb] = rgb.to_srgb();
        rr += sr * weight;
        gg += sg * weight;
        bb += sb * weight;
        total += weight;
    }
    debug_assert!(total != 0.0);
    let res = [rr / total, gg / total, bb / total];
    Color::from_srgb(res)
}

impl Default for Color {
    fn default() -> Self {
        Color::new(0.0, 0.0, 0.0)
    }
}
