use bitfields::bitfield;

use crate::{Material, Quad};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Face {
    Up,
    Down,
    Right,
    Left,
    Front,
    Back,
}

impl From<u8> for Face {
    fn from(value: u8) -> Self {
        assert!(value < 6);
        match value {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Right,
            3 => Self::Left,
            4 => Self::Front,
            5 => Self::Back,
            _ => unreachable!(),
        }
    }
}

#[bitfield(u32, new = false)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Vertex {
    #[bits(6)]
    x: u8,
    #[bits(6)]
    y: u8,
    #[bits(6)]
    z: u8,
    #[bits(6)]
    u: u8,
    #[bits(6)]
    v: u8,
    #[bits(2)]
    _reserved: u8,
}

impl Vertex {
    pub fn new(x: u8, y: u8, z: u8, u: u8, v: u8) -> Self {
        VertexBuilder::new()
            .with_x(x)
            .with_y(y)
            .with_z(z)
            .with_u(u)
            .with_v(v)
            .build()
    }

    pub const fn xyz(&self) -> [f32; 3] {
        [self.x() as f32, self.y() as f32, self.z() as f32]
    }

    pub const fn uv(&self) -> [f32; 2] {
        [self.u() as f32, self.v() as f32]
    }
}

impl Face {
    pub fn n(&self) -> [i32; 3] {
        match self {
            Self::Up => [0, 1, 0],
            Self::Down => [0, -1, 0],
            Self::Right => [1, 0, 0],
            Self::Left => [-1, 0, 0],
            Self::Front => [0, 0, 1],
            Self::Back => [0, 0, -1],
        }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Front => Self::Back,
            Self::Back => Self::Front,
        }
    }

    /// Takes a quad as outputted by binary greedy meshing, and outputs 4 vertices
    pub fn vertices_packed<M: Material>(&self, quad: impl Quad<M>) -> [Vertex; 4] {
        let x = quad.x();
        let y = quad.y();
        let z = quad.z();
        let w = quad.w();
        let h = quad.h();

        match self {
            Face::Left => [
                Vertex::new(x, y, z, h, w),
                Vertex::new(x, y, z + h, 0, w),
                Vertex::new(x, y + w, z, h, 0),
                Vertex::new(x, y + w, z + h, 0, 0),
            ],
            Face::Down => [
                Vertex::new(x - w, y, z + h, w, h),
                Vertex::new(x - w, y, z, w, 0),
                Vertex::new(x, y, z + h, 0, h),
                Vertex::new(x, y, z, 0, 0),
            ],
            Face::Back => [
                Vertex::new(x, y, z, w, h),
                Vertex::new(x, y + h, z, w, 0),
                Vertex::new(x + w, y, z, 0, h),
                Vertex::new(x + w, y + h, z, 0, 0),
            ],
            Face::Right => [
                Vertex::new(x, y, z, 0, 0),
                Vertex::new(x, y, z + h, h, 0),
                Vertex::new(x, y - w, z, 0, w),
                Vertex::new(x, y - w, z + h, h, w),
            ],
            Face::Up => [
                Vertex::new(x + w, y, z + h, w, h),
                Vertex::new(x + w, y, z, w, 0),
                Vertex::new(x, y, z + h, 0, h),
                Vertex::new(x, y, z, 0, 0),
            ],
            Face::Front => [
                Vertex::new(x - w, y + h, z, 0, 0),
                Vertex::new(x - w, y, z, 0, h),
                Vertex::new(x, y + h, z, w, 0),
                Vertex::new(x, y, z, w, h),
            ],
        }
    }
}
