use std::fmt::Display;

use crate::vec3::Vec3;

pub type Color = Vec3;

#[macro_export]
macro_rules! color {
    () => {
        Color::new()
    };
    ($e0:expr, $e1:expr, $e2:expr) => {
        Color::new_with($e0, $e1, $e2)
    };
}

impl Color {
    pub fn as_ppm(&self) -> ColorPpm {
        ColorPpm(self)
    }
}

pub struct ColorPpm<'a>(&'a Color);

impl<'a> Display for ColorPpm<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const F: f64 = 255.999;
        let x = (self.0.x() * F) as i32;
        let y = (self.0.y() * F) as i32;
        let z = (self.0.z() * F) as i32;
        writeln!(f, "{x} {y} {z}")
    }
}
