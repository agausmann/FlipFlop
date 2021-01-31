use crate::ivec::Vec2i;
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
    pub fn opposite(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Left => Self::Right,
            Self::Down => Self::Up,
        }
    }

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

    pub fn int_vector(self) -> Vec2i {
        match self {
            Self::Right => Vec2i::unit_x(),
            Self::Up => Vec2i::unit_y(),
            Self::Left => -Vec2i::unit_x(),
            Self::Down => -Vec2i::unit_y(),
        }
    }

    pub fn nearest(vec: Vec2) -> Self {
        if vec.x.abs() >= vec.y.abs() {
            if vec.x >= 0.0 {
                Self::Right
            } else {
                Self::Left
            }
        } else {
            if vec.y >= 0.0 {
                Self::Up
            } else {
                Self::Down
            }
        }
    }

    pub fn int_nearest(vec: Vec2i) -> Self {
        if vec.x.abs() >= vec.y.abs() {
            if vec.x >= 0 {
                Self::Right
            } else {
                Self::Left
            }
        } else {
            if vec.y >= 0 {
                Self::Up
            } else {
                Self::Down
            }
        }
    }
}

impl From<Direction> for Vec2 {
    fn from(dir: Direction) -> Self {
        dir.vector()
    }
}

impl From<Direction> for Vec2i {
    fn from(dir: Direction) -> Self {
        dir.int_vector()
    }
}

impl From<Direction> for Quat {
    fn from(dir: Direction) -> Self {
        Self::from_rotation_z(dir.angle())
    }
}
