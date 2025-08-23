use std::{io::Write, ops::Range};

pub struct BitWriter<T: Write> {
    output: T,
    buffer: u8,
    bit_count: u8,
}

impl<T: Write> BitWriter<T> {
    pub fn new(output: T) -> Self {
        Self { output, buffer: 0, bit_count: 0 }
    }

    pub fn write_u8(&mut self, value: u8, bits: u8) -> std::io::Result<()> {
        self.buffer |= value << self.bit_count;
        if self.bit_count + bits >= 8 {
            self.output.write_all(&[self.buffer])?;
            self.buffer = value >> (8 - self.bit_count);
        }
        self.bit_count = (self.bit_count + bits) % 8;
        Ok(())
    }

    pub fn write_slice_range(&mut self, slice: &[u8], range: Range<usize>) -> std::io::Result<()> {
        let first_pos = range.start % 8;
        if first_pos > 0 {
            let first = range.start / 8;
            let first_bits = (8 - first_pos).min(range.end - range.start);
            self.write_u8(slice[first] >> first_pos, first_bits as u8)?;
        }
        let whole_bytes_start = first_pos.div_ceil(8);
        let whole_bytes_end = range.end / 8;
        for &byte in &slice[whole_bytes_start..whole_bytes_end] {
            self.write_u8(byte, 8)?;
        }
        let end_pos = range.end % 8;
        if end_pos > 0 {
            let last = range.end / 8;
            let last_bits = 8 - end_pos;
            let mask = (1 << last_bits) - 1;
            self.write_u8(slice[last] & mask, last_bits as u8)?;
        }
        Ok(())
    }
}
