use std::fmt::Display;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Bool(pub u8);

impl Bool {
    pub fn to_bool(&self) -> bool {
        self.0 != 0
    }
}

impl Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_bool())
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Pad<const N: usize>(pub [u8; N]);

unsafe impl<const N: usize> Pod for Pad<N> {}
unsafe impl<const N: usize> Zeroable for Pad<N> {}

impl<const N: usize> Default for Pad<N> {
    fn default() -> Self {
        Pad([0; N])
    }
}

impl<const N: usize> Display for Pad<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..N {
            write!(f, "{:02x}", self.0[i])?;
        }
        Ok(())
    }
}

#[repr(C, packed)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct Ptr<T> {
    pub ptr: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Display for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = self.ptr;
        write!(f, "{ptr:#010x}")
    }
}
