use crate::util::bitwriter::BitWriter;

pub trait VecExt {
    fn shift_bits_left(&mut self, count: usize);
    fn assign_bits(
        &mut self,
        dst: usize,
        data: &[u8],
        src: usize,
        count: usize,
    ) -> std::io::Result<()>;
}

impl VecExt for Vec<u8> {
    fn shift_bits_left(&mut self, count: usize) {
        let len = self.len();
        self.resize(len + count.div_ceil(8), 0);
        self.copy_within(..len, count / 8);
        for i in 0..count / 8 {
            self[i] = 0;
        }
        let remaining = count % 8;
        if remaining == 0 {
            return;
        }
        for i in (0..self.len() - 1).rev() {
            let excess = self[i] >> (8 - remaining);
            self[i] <<= remaining;
            self[i + 1] |= excess;
        }
    }

    fn assign_bits(
        &mut self,
        dst: usize,
        data: &[u8],
        src: usize,
        count: usize,
    ) -> std::io::Result<()> {
        let start = dst / 8;
        let start_pos = dst % 8;
        let leading_bits = self[start] & ((1 << start_pos) - 1);

        let end_bits = (8 - (dst + count)) % 8;
        let trailing_byte = (end_bits > 0).then(|| {
            let end_byte = (dst + count) / 8;
            self[end_byte] >> (8 - end_bits)
        });

        let mut writer = BitWriter::new(&mut self[start..]);
        writer.write_u8(leading_bits, start_pos as u8)?;
        writer.write_slice_range(data, src..src + count)?;
        if let Some(trailing_byte) = trailing_byte {
            writer.write_u8(trailing_byte, end_bits as u8)?;
        }

        Ok(())
    }
}
