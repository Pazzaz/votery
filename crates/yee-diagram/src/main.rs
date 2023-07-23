use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use color::{blend_colors, blend_colors_weighted, Color};
use png::Writer;
use rand::{seq::SliceRandom, thread_rng, Rng};
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, ParallelDrainRange},
};
use votery::{
    formats::{
        toi::{TiedOrdersIncomplete, TiedVote},
        Specific,
    },
    generators::gaussian::Gaussian,
    methods::{
        random_ballot::{RandomBallot, RandomBallotSingle},
        Borda, Fptp, RandomVotingMethod,
    },
    prelude::VotingMethod,
};

mod color;

#[derive(PartialEq, Eq)]
enum Adaptive {
    Disable,
    Enable,
    Display,
}

// We only support 2 dimensional images right now
const DIMENSIONS: usize = 2;

// Each image is contained in a box [0.0, 1.0] x [0.0, 1.0]
const MIN: f64 = 0.0;
const MAX: f64 = 1.0;

struct ImageConfig {
    points: usize,
    resolution: usize,
    frames: usize,
    candidates: usize,
    sample_size: usize,
    max_noise: f64,
    variance: f64,
    adapt_mode: Adaptive,
    around_size: usize,
    blending: Blending,
    vote_color: VoteColor,
}

enum Blending {
    Max,
    Average,
}

#[derive(Clone, Copy)]
enum VoteColor {
    Winners,
    Harmonic,
}

impl Default for ImageConfig {
    fn default() -> Self {
        ImageConfig {
            points: 1000,
            resolution: 50,
            frames: 100,
            candidates: 4,
            sample_size: 5,
            max_noise: 0.2,
            variance: 0.2,
            adapt_mode: Adaptive::Enable,
            around_size: 2,
            blending: Blending::Max,
            vote_color: VoteColor::Winners,
        }
    }
}

fn create_png_writer(filename: &str, resolution: usize) -> Writer<BufWriter<File>> {
    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, resolution as u32, resolution as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap()
}

fn sample_pixel<R: Rng>(
    g: &Gaussian,
    xi: usize,
    yi: usize,
    rng: &mut R,
    colors: &[Color],
    config: &ImageConfig,
) -> Color {
    let x: f64 = (xi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let y: f64 = (yi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let votes = g.sample(rng, &[x, y]);
    let vote_fixed: TiedOrdersIncomplete = votes.into_iter().map(|x| x.owned()).collect();
    let mut mixes: Vec<Color> = Vec::new();
    let mut weights: Vec<f64> = Vec::new();
    let vote: TiedVote = Borda::count(&vote_fixed).unwrap().as_vote();
    match config.vote_color {
        VoteColor::Harmonic => {
            for (gi, group) in vote.slice().iter_groups().enumerate() {
                let mut hmm = Vec::new();
                for &i in group {
                    debug_assert!(i < colors.len());
                    hmm.push(colors[i]);
                }
                let new_c = blend_colors(&hmm);
                mixes.push(new_c);
                weights.push(1.0 / (gi + 1) as f64)
            }
            blend_colors_weighted(&mixes, Some(&weights))
        }
        VoteColor::Winners => {
            let i_colors: Vec<Color> = vote.slice().winners().iter().map(|&i| colors[i]).collect();
            blend_colors(&i_colors)
        }
    }
}

fn main() {
    let candidates = vec![[0.3, 0.7], [0.5, 0.84], [0.8, 0.2], [0.8, 0.7]];
    let mut directions = Vec::new();
    for [x, y] in &candidates {
        directions.push([y / 100.0, x / 100.0]);
    }
    let colors: Vec<Color> = vec![
        Color::new(255.0, 0.0, 0.0),
        Color::new(0.0, 255.0, 0.0),
        Color::new(0.0, 0.0, 255.0),
        Color::new(0.0, 0.0, 0.0),
    ];
    let config = ImageConfig::default();
    render_animation(candidates, directions, &colors, &config);
}

fn render_animation(
    mut candidates: Vec<[f64; 2]>,
    mut directions: Vec<[f64; 2]>,
    colors: &[Color],
    config: &ImageConfig,
) {
    for i in 0..config.frames {
        for j in 0..config.candidates {
            let [x, y] = candidates[j];
            let [dx, dy] = directions[j];
            let new_x = x + dx;
            let new_y = y + dy;
            if new_x < 0.0 {
                candidates[j][0] = 0.0;
                directions[j][0] = -directions[j][0];
            } else if new_x > 1.0 {
                candidates[j][0] = 1.0;
                directions[j][0] = -directions[j][0];
            } else {
                candidates[j][0] = new_x;
            }

            if new_y < 0.0 {
                candidates[j][1] = 0.0;
                directions[j][1] = -directions[j][1];
            } else if new_y > 1.0 {
                candidates[j][1] = 1.0;
                directions[j][1] = -directions[j][1];
            } else {
                candidates[j][1] = new_y;
            }
        }
        println!("{:?}", candidates[0]);
        render_image(&format!("animation/slow_borda_{}", i), &candidates, colors, config);
    }
}

fn render_image(name: &str, candidates: &[[f64; 2]], colors: &[Color], config: &ImageConfig) {
    debug_assert!(candidates.len() == config.candidates);
    // Output file
    let mut writer = create_png_writer(&format!("{}.png", name), config.resolution);
    let writer_adaptive: Option<_> = if config.adapt_mode == Adaptive::Display {
        Some(create_png_writer(&format!("{}_bw.png", name), config.resolution))
    } else {
        None
    };

    debug_assert!(colors.len() == config.candidates);
    let mut g = Gaussian::new(DIMENSIONS, config.variance, config.points);
    for c in candidates {
        assert!(vector(c));
        g.add_candidate(c);
    }
    let mut iterations = 0;
    let mut all_samples: Vec<Vec<Vec<Color>>> =
        vec![vec![Vec::new(); config.resolution]; config.resolution];
    let mut needs_samples = vec![vec![true; config.resolution]; config.resolution];
    let mut queue = Vec::with_capacity(config.resolution * config.resolution);
    let mut sample_count: Vec<Vec<usize>> = vec![vec![0; config.resolution]; config.resolution];
    let mut final_image: Vec<Vec<[u8; 3]>> = loop {
        iterations += 1;
        // First we'll add every pixel that needs samples to the queue
        queue.clear();
        for yi in 0..config.resolution {
            for xi in 0..config.resolution {
                if needs_samples[yi][xi] {
                    queue.push((xi, yi));
                    needs_samples[yi][xi] = false;
                }
            }
        }
        println!("{}: pixels to sample: {}", iterations, queue.len());
        // Then we actually get some samples
        let new_samples: Vec<(usize, usize, Vec<Color>)> = queue
            .par_drain(..)
            .map(|(xi, yi)| {
                let mut rng = thread_rng();
                let mut new_samples = Vec::with_capacity(config.sample_size);
                for _ in 0..config.sample_size {
                    let pixel = sample_pixel(&g, xi, yi, &mut rng, &colors, &config);
                    new_samples.push(pixel);
                }
                (xi, yi, new_samples)
            })
            .collect();
        // Then we need to decide which pixels need more samples. We say that a pixel
        // needs more samples if it hasn't converged, or if any of its neighbours
        // haven't converged yet
        let mut done = true;
        for (xi, yi, new) in new_samples {
            sample_count[yi][xi] += 1;
            let old = &mut all_samples[yi][xi];
            if old.len() == 0 || needs_samples[yi][xi] {
                needs_samples[yi][xi] = true;
                old.extend(new);
                done = false;
                continue;
            }
            let more_samples = match config.blending {
                Blending::Max => {
                    let old_color = most_common(old);
                    old.extend(new);
                    let new_color = most_common(old);
                    old_color != new_color
                }
                Blending::Average => {
                    let old_color = blend_colors(old);
                    old.extend(new);
                    let new_color = blend_colors(old);
                    let d = old_color.dist(&new_color);
                    d > config.max_noise
                }
            };
            if more_samples {
                done = false;
                let max_xi = xi.saturating_add(config.around_size).min(config.resolution - 1);
                let min_xi = xi.saturating_sub(config.around_size);
                let max_yi = yi.saturating_add(config.around_size).min(config.resolution - 1);
                let min_yi = yi.saturating_sub(config.around_size);
                for y in min_yi..=max_yi {
                    for x in min_xi..=max_xi {
                        needs_samples[y][x] = true;
                    }
                }
            }
        }

        if done {
            let mut final_image = vec![vec![[0, 0, 0]; config.resolution]; config.resolution];
            for yi in 0..config.resolution {
                for xi in 0..config.resolution {
                    final_image[yi][xi] = blend_colors(&all_samples[yi][xi]).quantize();
                }
            }
            break final_image;
        }
    };
    if config.adapt_mode == Adaptive::Display {
        let max_samples = sample_count.iter().map(|c| c.iter().max().unwrap()).max().unwrap();
        let adaptive_image: Vec<Vec<[u8; 3]>> = sample_count
            .iter()
            .map(|c| c.iter().map(|x| Color::bw(*x, *max_samples).quantize()).collect())
            .collect();
        let image_bytes: Vec<u8> = adaptive_image.iter().flatten().flatten().copied().collect();
        writer_adaptive.unwrap().write_image_data(&image_bytes).unwrap();
    }
    for c in 0..config.candidates {
        add_circle(&mut final_image, colors[c], &candidates[c], config.resolution);
    }
    let image_bytes: Vec<u8> = final_image.iter().flatten().flatten().copied().collect();
    writer.write_image_data(&image_bytes).unwrap();
}

fn most_common<T>(v: &mut Vec<T>) -> T
where
    T: Default + PartialOrd + Clone,
{
    if v.len() == 0 {
        return T::default();
    }
    v.sort_by(|a, b| a.partial_cmp(&b).unwrap());
    let mut most_common = None;
    let mut current_count = 0;
    let mut max_count = 0;
    let mut prev = None;
    for o in v.iter() {
        match most_common {
            Some(_) => {
                if prev.unwrap() == o {
                    current_count += 1;
                    if current_count > max_count {
                        max_count = current_count;
                        most_common = Some(o)
                    }
                } else {
                    current_count = 1;
                }
            }
            None => {
                most_common = Some(o);
                current_count = 1;
            }
        }
        prev = Some(o);
    }

    most_common.unwrap().clone()
}

fn add_circle(
    image: &mut Vec<Vec<[u8; 3]>>,
    color: Color,
    pos: &[f64; DIMENSIONS],
    resolution: usize,
) {
    let r = 0.02;
    let pi = std::f64::consts::PI;
    let mut angle: f64 = 0.0;
    while angle < 360.0 {
        let mut r_in = 0.0;
        while r_in < r {
            let x1 = r_in * f64::cos(angle * pi / 180.0);
            let y1 = r_in * f64::sin(angle * pi / 180.0);
            let x = pos[0] + x1;
            let y = pos[1] + y1;
            put_pixel(image, x, y, color, resolution);
            r_in += 0.001
        }
        angle += 0.1;
    }

    let mut angle: f64 = 0.0;
    while angle < 360.0 {
        let x1 = r * f64::cos(angle * pi / 180.0);
        let y1 = r * f64::sin(angle * pi / 180.0);
        let x = pos[0] + x1;
        let y = pos[1] + y1;
        put_pixel(image, x, y, color::BLACK, resolution);
        angle += 0.1;
    }
}

// maps [MIN, MAX) -> [0, RESOLUTION)
fn f64_to_coord(u: f64, resolution: usize) -> usize {
    let s = ((u - MIN) / (MAX - MIN) * resolution as f64) as usize;
    if s >= resolution {
        resolution - 1
    } else {
        s
    }
}

fn put_pixel(image: &mut Vec<Vec<[u8; 3]>>, x: f64, y: f64, color: Color, resolution: usize) {
    let xx = f64_to_coord(x, resolution);
    let yy = f64_to_coord(y, resolution);
    image[yy][xx] = color.quantize();
}

// void DrawCircle(int x, int y, int r, int color)
// {
//       static const double PI = 3.1415926535;
//       double i, angle, x1, y1;

//       for(i = 0; i < 360; i += 0.1)
//       {
//             angle = i;
//             x1 = r * cos(angle * PI / 180);
//             y1 = r * sin(angle * PI / 180);
//             putpixel(x + x1, y + y1, color);
//       }
// }

fn vector(n: &[f64]) -> bool {
    if n.len() != DIMENSIONS {
        return false;
    }
    for &i in n {
        if i < MIN || MAX < i {
            return false;
        }
    }
    true
}
