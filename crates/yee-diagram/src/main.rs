use color::Color;
use rand::thread_rng;
use yee::{color, random_candidates, render_animation, ImageConfig};

fn main() {
    let config = ImageConfig::default();
    let candidates = random_candidates(&mut thread_rng(), config.candidates);
    let mut directions = Vec::new();
    for [x, y] in &candidates {
        directions.push([y / 100.0, x / 100.0]);
    }
    let colors: Vec<Color> =
        (0..candidates.len()).into_iter().map(|i| Color::dutch_field(i)).collect();
    render_animation(candidates, directions, &colors, &config);
}
