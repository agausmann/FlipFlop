use bevy::math::Vec2;
use std::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Vec2i {
    pub x: i32,
    pub y: i32,
}

impl Vec2i {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn unit_x() -> Self {
        Self::new(1, 0)
    }

    pub fn unit_y() -> Self {
        Self::new(0, 1)
    }

    pub fn floor(vec: Vec2) -> Self {
        Self::new(vec.x.floor() as i32, vec.y.floor() as i32)
    }

    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }
}

impl ops::Neg for Vec2i {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl ops::Add for Vec2i {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl ops::Sub for Vec2i {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl ops::Mul<i32> for Vec2i {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl ops::Mul for Vec2i {
    type Output = Self;

    fn mul(self, rhs: Vec2i) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl From<Vec2i> for Vec2 {
    fn from(vec: Vec2i) -> Self {
        Self::new(vec.x as f32, vec.y as f32)
    }
}
