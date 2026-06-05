use bitfields::bitfield;

use crate::Material;

pub trait Quad<M: Material> {
    fn new(x: u8, y: u8, z: u8, w: u8, h: u8, m: M) -> Self;
    fn x(&self) -> u8;
    fn y(&self) -> u8;
    fn z(&self) -> u8;
    fn w(&self) -> u8;
    fn h(&self) -> u8;
    fn m(&self) -> M;
}

#[bitfield(u64)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RichQuad {
    #[bits(6)]
    x: u8,
    #[bits(6)]
    y: u8,
    #[bits(6)]
    z: u8,
    #[bits(6)]
    w: u8,
    #[bits(6)]
    h: u8,
    #[bits(2)] // ao
    _reserved: u8,
    m: u32,
}

impl Quad<u32> for RichQuad {
    fn new(x: u8, y: u8, z: u8, w: u8, h: u8, m: u32) -> Self {
        RichQuadBuilder::new()
            .with_x(x)
            .with_y(y)
            .with_z(z)
            .with_w(w)
            .with_h(h)
            .with_m(m)
            .build()
    }
    fn x(&self) -> u8 {
        self.x()
    }
    fn y(&self) -> u8 {
        self.y()
    }
    fn z(&self) -> u8 {
        self.z()
    }
    fn w(&self) -> u8 {
        self.w()
    }
    fn h(&self) -> u8 {
        self.h()
    }
    fn m(&self) -> u32 {
        self.m()
    }
}

#[bitfield(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MiniQuad {
    #[bits(6)]
    x: u8,
    #[bits(6)]
    y: u8,
    #[bits(6)]
    z: u8,
    #[bits(6)]
    w: u8,
    #[bits(6)]
    h: u8,
    #[bits(2)]
    m: u8,
}

impl Quad<u8> for MiniQuad {
    fn new(x: u8, y: u8, z: u8, w: u8, h: u8, m: u8) -> Self {
        MiniQuadBuilder::new()
            .with_x(x)
            .with_y(y)
            .with_z(z)
            .with_w(w)
            .with_h(h)
            .with_m(m)
            .build()
    }
    fn x(&self) -> u8 {
        self.x()
    }
    fn y(&self) -> u8 {
        self.y()
    }
    fn z(&self) -> u8 {
        self.z()
    }
    fn w(&self) -> u8 {
        self.w()
    }
    fn h(&self) -> u8 {
        self.h()
    }
    fn m(&self) -> u8 {
        self.m()
    }
}

#[bitfield(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MicroQuad {
    #[bits(5)]
    x: u8,
    #[bits(5)]
    y: u8,
    #[bits(5)]
    z: u8,
    #[bits(5)]
    w: u8,
    #[bits(5)]
    h: u8,
    #[bits(7)]
    m: u8,
}

impl Quad<u8> for MicroQuad {
    fn new(x: u8, y: u8, z: u8, w: u8, h: u8, m: u8) -> Self {
        MicroQuadBuilder::new()
            .with_x(x)
            .with_y(y)
            .with_z(z)
            .with_w(w)
            .with_h(h)
            .with_m(m)
            .build()
    }
    fn x(&self) -> u8 {
        self.x()
    }
    fn y(&self) -> u8 {
        self.y()
    }
    fn z(&self) -> u8 {
        self.z()
    }
    fn w(&self) -> u8 {
        self.w()
    }
    fn h(&self) -> u8 {
        self.h()
    }
    fn m(&self) -> u8 {
        self.m()
    }
}
