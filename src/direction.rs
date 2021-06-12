use glam::{mat2, Mat2, Vec2};

#[derive(Clone, Copy)]
pub enum Direction {
    East,
    North,
    West,
    South,
}

impl Direction {
    pub fn rotate(self, relative: Relative) -> Self {
        match (self, relative) {
            (Self::East, Relative::Same) => Self::East,
            (Self::North, Relative::Same) => Self::North,
            (Self::West, Relative::Same) => Self::West,
            (Self::South, Relative::Same) => Self::South,

            (Self::East, Relative::Opposite) => Self::West,
            (Self::North, Relative::Opposite) => Self::South,
            (Self::West, Relative::Opposite) => Self::East,
            (Self::South, Relative::Opposite) => Self::North,

            (Self::East, Relative::Right) => Self::South,
            (Self::North, Relative::Right) => Self::East,
            (Self::West, Relative::Right) => Self::North,
            (Self::South, Relative::Right) => Self::West,

            (Self::East, Relative::Left) => Self::North,
            (Self::North, Relative::Left) => Self::West,
            (Self::West, Relative::Left) => Self::South,
            (Self::South, Relative::Left) => Self::East,
        }
    }

    pub fn to(self, other: Self) -> Relative {
        match (self, other) {
            (Self::East, Self::East)
            | (Self::North, Self::North)
            | (Self::West, Self::West)
            | (Self::South, Self::South) => Relative::Same,

            (Self::East, Self::West)
            | (Self::North, Self::South)
            | (Self::West, Self::East)
            | (Self::South, Self::North) => Relative::Opposite,

            (Self::East, Self::South)
            | (Self::North, Self::East)
            | (Self::West, Self::North)
            | (Self::South, Self::West) => Relative::Right,

            (Self::East, Self::North)
            | (Self::North, Self::West)
            | (Self::West, Self::South)
            | (Self::South, Self::East) => Relative::Left,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Relative {
    Same,
    Opposite,
    Right,
    Left,
}

impl Relative {
    pub fn transform(self) -> Mat2 {
        match self {
            Self::Same => mat2(Vec2::X, Vec2::Y),
            Self::Opposite => mat2(-Vec2::X, -Vec2::Y),
            Self::Right => mat2(-Vec2::Y, Vec2::X),
            Self::Left => mat2(Vec2::Y, -Vec2::X),
        }
    }
}
