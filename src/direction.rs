use bevy::prelude::*;
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Right,
    Up,
    Left,
    Down,
}

impl Direction {
    pub fn angle(self) -> f32 {
        match self {
            Self::Right => 0.0,
            Self::Up => PI * 0.5,
            Self::Left => PI,
            Self::Down => PI * 1.5,
        }
    }

    pub fn vector(self) -> Vec2 {
        match self {
            Self::Right => Vec2::unit_x(),
            Self::Up => Vec2::unit_y(),
            Self::Left => -Vec2::unit_x(),
            Self::Down => -Vec2::unit_y(),
        }
    }
}

impl From<Direction> for Vec2 {
    fn from(dir: Direction) -> Self {
        dir.vector()
    }
}

impl From<Direction> for Quat {
    fn from(dir: Direction) -> Self {
        Self::from_rotation_z(dir.angle())
    }
}
