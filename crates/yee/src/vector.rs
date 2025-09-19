/// A two-dimensional vector
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    pub fn from_array(xy: [f64; 2]) -> Self {
        Vector { x: xy[0], y: xy[1] }
    }

    pub fn as_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    pub fn sub(&self, b: &Vector) -> Vector {
        Vector { x: self.x - b.x, y: self.y - b.y }
    }

    pub fn add_assign(&mut self, b: &Vector) {
        self.x += b.x;
        self.y += b.y;
    }

    pub fn add(&self, b: &Vector) -> Vector {
        Vector { x: self.x + b.x, y: self.y + b.y }
    }

    pub fn div_assign_s(&mut self, s: f64) {
        self.x /= s;
        self.y /= s;
    }

    pub fn scaled(&self, s: f64) -> Vector {
        Vector { x: self.x * s, y: self.y * s }
    }

    pub fn clamp(&self, min: f64, max: f64) -> Vector {
        Vector { x: self.x.clamp(min, max), y: self.y.clamp(min, max) }
    }
}
