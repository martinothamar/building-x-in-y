use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Vec3 {
    e: [f64; 3],
}

pub type Point3 = Vec3;

#[macro_export]
macro_rules! vec3 {
    () => {
        Vec3::new()
    };
    ($e0:expr, $e1:expr, $e2:expr) => {
        Vec3::new_with($e0, $e1, $e2)
    };
}

impl Vec3 {
    #[inline]
    pub const fn new() -> Self {
        Self { e: [0f64; 3] }
    }

    #[inline]
    pub const fn new_with(e0: f64, e1: f64, e2: f64) -> Self {
        Self { e: [e0, e1, e2] }
    }

    #[inline]
    pub const fn x(&self) -> f64 {
        self.e[0]
    }

    #[inline]
    pub const fn y(&self) -> f64 {
        self.e[1]
    }

    #[inline]
    pub const fn z(&self) -> f64 {
        self.e[2]
    }

    #[inline]
    pub fn len(&self) -> f64 {
        self.len_squared().sqrt()
    }

    #[inline]
    pub const fn len_squared(&self) -> f64 {
        self.e[0] * self.e[0] + self.e[1] * self.e[1] + self.e[2] * self.e[2]
    }

    #[inline]
    pub const fn dot(&self, v: &Vec3) -> f64 {
        let u = self;

        #[rustfmt::skip]
        return u.e[0] * v.e[0] +
               u.e[1] * v.e[1] +
               u.e[2] * v.e[2];
    }

    #[inline]
    pub const fn cross(&self, v: &Vec3) -> Vec3 {
        let u = self;
        #[rustfmt::skip]
        return Vec3::new_with(
            u.e[1] * v.e[2] - u.e[2] * v.e[1],
            u.e[2] * v.e[0] - u.e[0] * v.e[2],
            u.e[0] * v.e[1] - u.e[1] * v.e[0]
        );
    }

    #[inline]
    pub fn unit_vector(&self) -> Vec3 {
        self / self.len()
    }
}

impl Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [x, y, z] = self.e;
        write!(f, "{x} {y} {z}")
    }
}

impl Index<usize> for Vec3 {
    type Output = f64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.e[index]
    }
}

impl IndexMut<usize> for Vec3 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.e[index]
    }
}

impl const AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.e[0] += rhs.e[0];
        self.e[1] += rhs.e[1];
        self.e[2] += rhs.e[2];
    }
}
impl const AddAssign<f64> for Vec3 {
    fn add_assign(&mut self, rhs: f64) {
        self.e[0] += rhs;
        self.e[1] += rhs;
        self.e[2] += rhs;
    }
}
impl const AddAssign for &mut Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.e[0] += rhs.e[0];
        self.e[1] += rhs.e[1];
        self.e[2] += rhs.e[2];
    }
}
impl const AddAssign<f64> for &mut Vec3 {
    fn add_assign(&mut self, rhs: f64) {
        self.e[0] += rhs;
        self.e[1] += rhs;
        self.e[2] += rhs;
    }
}
impl const Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] + rhs.e[0];
        let e1 = self.e[1] + rhs.e[1];
        let e2 = self.e[2] + rhs.e[2];
        Self::new_with(e0, e1, e2)
    }
}
impl const Add<f64> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: f64) -> Self::Output {
        let e0 = self.e[0] + rhs;
        let e1 = self.e[1] + rhs;
        let e2 = self.e[2] + rhs;
        Self::new_with(e0, e1, e2)
    }
}
impl const Add for &Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] + rhs.e[0];
        let e1 = self.e[1] + rhs.e[1];
        let e2 = self.e[2] + rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}
impl const Add<f64> for &Vec3 {
    type Output = Vec3;

    fn add(self, rhs: f64) -> Self::Output {
        let e0 = self.e[0] + rhs;
        let e1 = self.e[1] + rhs;
        let e2 = self.e[2] + rhs;
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.e[0] -= rhs.e[0];
        self.e[1] -= rhs.e[1];
        self.e[2] -= rhs.e[2];
    }
}

impl const SubAssign<f64> for Vec3 {
    fn sub_assign(&mut self, rhs: f64) {
        self.e[0] -= rhs;
        self.e[1] -= rhs;
        self.e[2] -= rhs;
    }
}

impl const SubAssign for &mut Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.e[0] -= rhs.e[0];
        self.e[1] -= rhs.e[1];
        self.e[2] -= rhs.e[2];
    }
}

impl const SubAssign<f64> for &mut Vec3 {
    fn sub_assign(&mut self, rhs: f64) {
        self.e[0] -= rhs;
        self.e[1] -= rhs;
        self.e[2] -= rhs;
    }
}

impl const Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] - rhs.e[0];
        let e1 = self.e[1] - rhs.e[1];
        let e2 = self.e[2] - rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Sub<f64> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: f64) -> Self::Output {
        let e0 = self.e[0] - rhs;
        let e1 = self.e[1] - rhs;
        let e2 = self.e[2] - rhs;
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Sub for &Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] - rhs.e[0];
        let e1 = self.e[1] - rhs.e[1];
        let e2 = self.e[2] - rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Sub<f64> for &Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: f64) -> Self::Output {
        let e0 = self.e[0] - rhs;
        let e1 = self.e[1] - rhs;
        let e2 = self.e[2] - rhs;
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const MulAssign for Vec3 {
    fn mul_assign(&mut self, rhs: Self) {
        self.e[0] *= rhs.e[0];
        self.e[1] *= rhs.e[1];
        self.e[2] *= rhs.e[2];
    }
}

impl const MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        self.e[0] *= rhs;
        self.e[1] *= rhs;
        self.e[2] *= rhs;
    }
}
impl const MulAssign for &mut Vec3 {
    fn mul_assign(&mut self, rhs: Self) {
        self.e[0] *= rhs.e[0];
        self.e[1] *= rhs.e[1];
        self.e[2] *= rhs.e[2];
    }
}

impl const MulAssign<f64> for &mut Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        self.e[0] *= rhs;
        self.e[1] *= rhs;
        self.e[2] *= rhs;
    }
}

impl const Mul for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] * rhs.e[0];
        let e1 = self.e[1] * rhs.e[1];
        let e2 = self.e[2] * rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Mul<f64> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f64) -> Self::Output {
        let e0 = self.e[0] * rhs;
        let e1 = self.e[1] * rhs;
        let e2 = self.e[2] * rhs;
        Self::Output::new_with(e0, e1, e2)
    }
}
impl const Mul for &Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] * rhs.e[0];
        let e1 = self.e[1] * rhs.e[1];
        let e2 = self.e[2] * rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Mul<f64> for &Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f64) -> Self::Output {
        let e0 = self.e[0] * rhs;
        let e1 = self.e[1] * rhs;
        let e2 = self.e[2] * rhs;
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const DivAssign for Vec3 {
    fn div_assign(&mut self, rhs: Self) {
        self.e[0] /= rhs.e[0];
        self.e[1] /= rhs.e[1];
        self.e[2] /= rhs.e[2];
    }
}

impl const DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, rhs: f64) {
        *self *= 1.0 / rhs;
    }
}

impl const DivAssign for &mut Vec3 {
    fn div_assign(&mut self, rhs: Self) {
        self.e[0] /= rhs.e[0];
        self.e[1] /= rhs.e[1];
        self.e[2] /= rhs.e[2];
    }
}

impl const DivAssign<f64> for &mut Vec3 {
    fn div_assign(&mut self, rhs: f64) {
        *self *= 1.0 / rhs;
    }
}

impl const Div for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] / rhs.e[0];
        let e1 = self.e[1] / rhs.e[1];
        let e2 = self.e[2] / rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Div<f64> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f64) -> Self::Output {
        self * (1.0 / rhs)
    }
}

impl const Div for &Vec3 {
    type Output = Vec3;

    fn div(self, rhs: Self) -> Self::Output {
        let e0 = self.e[0] / rhs.e[0];
        let e1 = self.e[1] / rhs.e[1];
        let e2 = self.e[2] / rhs.e[2];
        Self::Output::new_with(e0, e1, e2)
    }
}

impl const Div<f64> for &Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f64) -> Self::Output {
        self * (1.0 / rhs)
    }
}

impl const Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        Self {
            e: [-self.e[0], -self.e[1], -self.e[2]],
        }
    }
}
