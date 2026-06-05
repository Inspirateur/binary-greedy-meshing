use num_traits::Zero;

pub trait Material: PartialEq + Copy {
    fn is_air(&self) -> bool;
    fn is_solid(&self) -> bool {
        !self.is_air()
    }
}

impl<T: Zero + PartialEq + Copy> Material for T {
    fn is_air(&self) -> bool {
        self.is_zero()
    }
}
