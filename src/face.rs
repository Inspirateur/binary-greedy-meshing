use crate::Quad;

const MASK_XYZ: u64 = 0b111111_111111_111111;

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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Vertex(pub u32);

impl Vertex {
    const MASK_6: u32 = 0b111111;

    pub fn new() -> Self {
        Self(0u32)
    }

    pub fn pack(xyz: u32, u: u32, v: u32) -> Self {
        Self((v << 24) | (u << 18) | xyz)
    }

    pub fn x(&self) -> u32 {
        self.0 & Vertex::MASK_6
    }

    pub fn y(&self) -> u32 {
        (self.0 >> 6) & Vertex::MASK_6
    }

    pub fn z(&self) -> u32 {
        (self.0 >> 12) & Vertex::MASK_6
    }

    pub fn u(&self) -> u32 {
        (self.0 >> 18) & Vertex::MASK_6
    }

    pub fn v(&self) -> u32 {
        (self.0 >> 24) & Vertex::MASK_6
    }

    pub fn xyz(&self) -> [u32; 3] {
        [self.x(), self.y(), self.z()]
    }
}

fn packed_xyz(x: u32, y: u32, z: u32) -> u32 {
    (z << 12) | (y << 6) | x
}

impl Face {
    pub fn n(&self) -> [f32; 3] {
        match self {
            Self::Up => [0., 1., 0.],
            Self::Down => [0., -1., 0.],
            Self::Right => [1., 0., 0.],
            Self::Left => [-1., 0., 0.],
            Self::Front => [0., 0., 1.],
            Self::Back => [0., 0., -1.],
        }
    }

    /// Takes a quad as outputted by binary greedy meshing, and outputs 4 vertices encoded as:
    /// (v << 24) | (u << 18) | (z << 12) | (y << 6) | x
    pub fn vertices_packed(&self, quad: Quad) -> [Vertex; 4] {
        let w = quad.width() as u32;
        let h = quad.height() as u32;
        let xyz = (MASK_XYZ & quad.0) as u32;
        match self {
            Face::Left => [
                Vertex::pack(xyz, h, w),
                Vertex::pack(xyz + packed_xyz(0, 0, h), 0, w),
                Vertex::pack(xyz + packed_xyz(0, w, 0), h, 0),
                Vertex::pack(xyz + packed_xyz(0, w, h), 0, 0),
            ],
            Face::Down => [
                Vertex::pack(xyz - packed_xyz(w, 0, 0) + packed_xyz(0, 0, h), w, h),
                Vertex::pack(xyz - packed_xyz(w, 0, 0), w, 0),
                Vertex::pack(xyz + packed_xyz(0, 0, h), 0, h),
                Vertex::pack(xyz, 0, 0),
            ],
            Face::Back => [
                Vertex::pack(xyz, w, h),
                Vertex::pack(xyz + packed_xyz(0, h, 0), w, 0),
                Vertex::pack(xyz + packed_xyz(w, 0, 0), 0, h),
                Vertex::pack(xyz + packed_xyz(w, h, 0), 0, 0),
            ],
            Face::Right => [
                Vertex::pack(xyz, 0, 0),
                Vertex::pack(xyz + packed_xyz(0, 0, h), h, 0),
                Vertex::pack(xyz - packed_xyz(0, w, 0), 0, w),
                Vertex::pack(xyz + packed_xyz(0, 0, h) - packed_xyz(0, w, 0), h, w),
            ],
            Face::Up => [
                Vertex::pack(xyz + packed_xyz(w, 0, h), w, h),
                Vertex::pack(xyz + packed_xyz(w, 0, 0), w, 0),
                Vertex::pack(xyz + packed_xyz(0, 0, h), 0, h),
                Vertex::pack(xyz, 0, 0),
            ],
            Face::Front => [
                Vertex::pack(xyz - packed_xyz(w, 0, 0) + packed_xyz(0, h, 0), 0, 0),
                Vertex::pack(xyz - packed_xyz(w, 0, 0), 0, h),
                Vertex::pack(xyz + packed_xyz(0, h, 0), w, 0),
                Vertex::pack(xyz, w, h),
            ],
        }
    }
}
