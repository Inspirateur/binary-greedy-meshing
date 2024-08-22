const MASK_6: u64 = 0b111111;
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

fn packed_xyz(x: u32, y: u32, z: u32) -> u32 {
    (z << 12) | (y << 6) | x
}

fn vertex_info(xyz: u32, u: u32, v: u32) -> u32 {
    (v << 24) | (u << 18) | xyz
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
    pub fn vertices_packed(&self, quad: u64) -> [u32; 4] {
        let w = (MASK_6 & (quad >> 18)) as u32;
        let h = (MASK_6 & (quad >> 24)) as u32;
        let xyz = (MASK_XYZ & quad) as u32;
        match self {
            Face::Left => [
                vertex_info(xyz, h, w),
                vertex_info(xyz+packed_xyz(0, 0, h), 0, w),
                vertex_info(xyz+packed_xyz(0, w, 0), h, 0),
                vertex_info(xyz+packed_xyz(0, w, h), 0, 0),
            ],
            Face::Down => [
                vertex_info(xyz-packed_xyz(w, 0, 0)+packed_xyz(0, 0, h), w, h),
                vertex_info(xyz-packed_xyz(w, 0, 0), w, 0),
                vertex_info(xyz+packed_xyz(0, 0, h), 0, h),
                vertex_info(xyz, 0, 0),
            ],
            Face::Back => [
                vertex_info(xyz, w, h),
                vertex_info(xyz+packed_xyz(0, h, 0), w, 0),
                vertex_info(xyz+packed_xyz(w, 0, 0), 0, h),
                vertex_info(xyz+packed_xyz(w, h, 0), 0, 0),
            ],
            Face::Right => [
                vertex_info(xyz, 0, 0),
                vertex_info(xyz+packed_xyz(0, 0, h), h, 0),
                vertex_info(xyz-packed_xyz(0, w, 0), 0, w),
                vertex_info(xyz+packed_xyz(0, 0, h)-packed_xyz(0, w, 0), h, w),
            ],
            Face::Up => [
                vertex_info(xyz+packed_xyz(w, 0, h), w, h),
                vertex_info(xyz+packed_xyz(w, 0, 0), w, 0),
                vertex_info(xyz+packed_xyz(0, 0, h), 0, h),
                vertex_info(xyz, 0, 0),
            ],
            Face::Front => [
                vertex_info(xyz-packed_xyz(w, 0, 0)+packed_xyz(0, h, 0), 0, 0),
                vertex_info(xyz-packed_xyz(w, 0, 0), 0, h),
                vertex_info(xyz+packed_xyz(0, h, 0), w, 0),
                vertex_info(xyz, w, h),
            ],
        }
    }
}