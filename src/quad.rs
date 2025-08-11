use alloc::string::String;

pub(crate) const MASK_6: u64 = 0b111111;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Quad(pub u64);

impl Quad {
    /// x: 6 bits
    /// y: 6 bits
    /// z: 6 bits 18
    /// width (w): 6 bits
    /// height (h): 6 bits
    /// voxel id (v): 32 bits
    ///
    /// ao (a): 2 bits
    ///
    /// layout:
    /// 0bvvvv_vvvv_vvvv_vvvv_vvvv_vvvv_vvvv_vvvv_00hh_hhhh_wwww_wwzz_zzzz_yyyy_yyxx_xxxx
    #[inline]
    pub fn pack(x: usize, y: usize, z: usize, w: usize, h: usize, v_type: usize) -> Self {
        Quad(((v_type << 32) | (h << 24) | (w << 18) | (z << 12) | (y << 6) | x) as u64)
    }

    #[inline]
    pub fn xyz(&self) -> [u64; 3] {
        let x = (self.0) & MASK_6;
        let y = (self.0 >> 6) & MASK_6;
        let z = (self.0 >> 12) & MASK_6;
        [x, y, z]
    }

    #[inline]
    pub fn width(&self) -> u64 {
        (self.0 >> 18) & MASK_6
    }

    #[inline]
    pub fn height(&self) -> u64 {
        (self.0 >> 24) & MASK_6
    }

    #[inline]
    pub fn voxel_id(&self) -> u64 {
        self.0 >> 32
    }

    /// Unpacks quad data and formats it as "{x};{y};{z} {w}x{h} v={v_type}" for debugging
    #[inline]
    pub fn debug_quad(&self) -> String {
        let mut quad = self.0;
        let x = quad & MASK_6;
        quad >>= 6;
        let y = quad & MASK_6;
        quad >>= 6;
        let z = quad & MASK_6;
        quad >>= 6;
        let w = quad & MASK_6;
        quad >>= 6;
        let h = quad & MASK_6;
        quad >>= 8;
        let v_type = quad;
        format!("{x};{y};{z} {w}x{h} v={v_type}")
    }
}

impl alloc::fmt::Debug for Quad {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Quad")
            .field("position", &self.xyz())
            .field("width", &self.width())
            .field("height", &self.height())
            .field("voxel_id", &self.voxel_id())
            .finish()
    }
}
