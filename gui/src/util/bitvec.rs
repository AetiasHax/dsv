use bitvec::{order::BitOrder, store::BitStore, vec::BitVec};

pub trait BitVecExt {
    fn truncate_remove(&mut self, len: usize);
}

impl<T: BitStore, O: BitOrder> BitVecExt for BitVec<T, O> {
    fn truncate_remove(&mut self, len: usize) {
        self[len..len.next_multiple_of(8 * size_of::<T>())].fill(false);
        self.truncate(len);
    }
}
