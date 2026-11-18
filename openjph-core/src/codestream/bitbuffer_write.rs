//! Bit-buffer writer for codestream generation.
//!
//! Port of `ojph_bitbuffer_write.h`. Handles JPEG 2000 byte-stuffing:
//! after writing a 0xFF byte, the next byte must have its MSB clear.

/// Bit-level writer with JPEG 2000 0xFF byte-stuffing.
#[derive(Debug)]
pub struct BitBufferWrite {
    data: Vec<u8>,
    /// Accumulated bit buffer (left-justified).
    buf: u64,
    /// Number of valid bits in `buf`.
    bits_used: u32,
    /// True if the last flushed byte was 0xFF.
    unstuff: bool,
}

impl BitBufferWrite {
    /// Create a new bit writer.
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(256),
            buf: 0,
            bits_used: 0,
            unstuff: false,
        }
    }

    /// Write `n` bits from the MSB side of `val` (n <= 32).
    #[inline]
    pub fn write(&mut self, val: u32, n: u32) {
        debug_assert!(n <= 32);
        self.buf |= (val as u64) << (64 - self.bits_used - n);
        self.bits_used += n;
        if self.bits_used >= 32 {
            self.flush_bytes();
        }
    }

    /// Flush complete bytes from the buffer to the output, applying stuffing.
    fn flush_bytes(&mut self) {
        loop {
            if self.unstuff {
                if self.bits_used < 7 {
                    break;
                }
                // After 0xFF, output top 7 bits with MSB forced to 0
                let val = ((self.buf >> 57) as u8) & 0x7F;
                self.data.push(val);
                self.buf <<= 7;
                self.bits_used -= 7;
                self.unstuff = false;
            } else {
                if self.bits_used < 8 {
                    break;
                }
                let byte = (self.buf >> 56) as u8;
                self.data.push(byte);
                self.buf <<= 8;
                self.bits_used -= 8;
                self.unstuff = byte == 0xFF;
            }
        }
    }

    /// Flush all remaining bits, padding the last byte with zeros. Returns
    /// whether a zero byte was appended after a trailing 0xFF.
    ///
    /// Emits the fewest bytes that hold the written bits, as OpenJPH's
    /// `bb_terminate` does: a decoder finds the end of a packet header by
    /// where its bits end, so any extra padding byte would be read as the
    /// first byte of the packet body.
    pub fn finalize(&mut self) -> bool {
        // Emit every complete byte (8 bits, or 7 after a 0xFF).
        self.flush_bytes();
        if self.bits_used > 0 {
            // Fewer bits than a byte remain: zero-pad them to one byte (the
            // bits of `buf` past `bits_used` are already zero).
            self.bits_used = if self.unstuff { 7 } else { 8 };
            self.flush_bytes();
        }
        if self.unstuff {
            // A trailing 0xFF is followed by a zero byte.
            self.data.push(0);
            self.unstuff = false;
            return true;
        }
        false
    }

    /// Get the written data.
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }

    /// Take ownership of the written data.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Returns the current byte length of written data.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if no data has been written.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() && self.bits_used == 0
    }

    /// Reset the writer.
    pub fn reset(&mut self) {
        self.data.clear();
        self.buf = 0;
        self.bits_used = 0;
        self.unstuff = false;
    }
}

impl Default for BitBufferWrite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_writer() {
        let writer = BitBufferWrite::new();
        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);
        assert_eq!(writer.get_data(), &[]);
    }

    #[test]
    fn write_single_byte() {
        let mut writer = BitBufferWrite::new();
        writer.write(0xAB, 8);
        writer.finalize();
        assert_eq!(writer.get_data()[0], 0xAB);
    }

    #[test]
    fn write_individual_bits() {
        let mut writer = BitBufferWrite::new();
        // Write 1,0,1,0,1,0,1,1 = 0xAB
        writer.write(1, 1);
        writer.write(0, 1);
        writer.write(1, 1);
        writer.write(0, 1);
        writer.write(1, 1);
        writer.write(0, 1);
        writer.write(1, 1);
        writer.write(1, 1);
        writer.finalize();
        assert_eq!(writer.get_data()[0], 0xAB);
    }

    #[test]
    fn write_multi_bit_values() {
        let mut writer = BitBufferWrite::new();
        writer.write(0xA, 4); // 1010
        writer.write(0xB, 4); // 1011
        writer.finalize();
        assert_eq!(writer.get_data()[0], 0xAB);
    }

    #[test]
    fn byte_stuffing_after_0xff() {
        let mut writer = BitBufferWrite::new();
        // Write 0xFF (8 bits of 1s)
        writer.write(0xFF, 8);
        // After flushing 0xFF, the next byte should have MSB=0
        // Write 0xFF again → only 7 bits fit in stuffed byte
        writer.write(0xFF, 8);
        writer.finalize();
        let data = writer.get_data();
        assert_eq!(data[0], 0xFF);
        // After 0xFF, next byte has MSB forced to 0
        assert_eq!(data[1] & 0x80, 0x00);
    }

    #[test]
    fn finalize_pads_with_zeros() {
        let mut writer = BitBufferWrite::new();
        writer.write(0x7, 3); // 111
        writer.finalize();
        // 111 padded to 8 bits → 1110_0000 = 0xE0
        assert_eq!(writer.get_data()[0], 0xE0);
    }

    #[test]
    fn finalize_adds_zero_after_final_0xff() {
        let mut writer = BitBufferWrite::new();
        writer.write(0xFF, 8);
        let was_ff = writer.finalize();
        let data = writer.get_data();
        // After 0xFF there should be a 0x00 byte appended
        assert!(data.len() >= 2);
        assert_eq!(data[0], 0xFF);
        assert_eq!(data[1], 0x00);
        // finalize returns true or the trailing zero is present
        let _ = was_ff;
    }

    #[test]
    fn finalize_emits_no_extra_byte_after_stuffing() {
        // 0xFF, then 9 bits: the byte after 0xFF holds 7 of them, the last
        // byte the other 2. Three bytes; a fourth would be read as the
        // packet body by a decoder.
        let mut writer = BitBufferWrite::new();
        writer.write(0xFF, 8);
        writer.write(0x1FF, 9);
        assert!(!writer.finalize());
        assert_eq!(writer.get_data(), &[0xFF, 0x7F, 0xC0]);
    }

    #[test]
    fn finalize_after_stuffing_fills_exactly_one_byte() {
        // From issue #2: 0xFF followed by 7 bits is exactly two bytes.
        let mut writer = BitBufferWrite::new();
        writer.write(0xFF, 8);
        writer.write(0x7F, 7);
        assert!(!writer.finalize());
        assert_eq!(writer.get_data(), &[0xFF, 0x7F]);
    }

    #[test]
    fn len_and_is_empty() {
        let mut writer = BitBufferWrite::new();
        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);
        writer.write(0xAB, 8);
        writer.write(0xCD, 8);
        // After writing 16 bits, flush_bytes triggers for both
        // is_empty now depends on whether bits remain in buffer
        writer.finalize();
        assert!(!writer.is_empty());
        assert!(writer.len() >= 2);
    }

    #[test]
    fn reset_clears_all() {
        let mut writer = BitBufferWrite::new();
        writer.write(0xAB, 8);
        writer.write(0xCD, 8);
        writer.finalize();
        assert!(!writer.is_empty());

        writer.reset();
        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);
        assert_eq!(writer.get_data(), &[]);
    }

    #[test]
    fn write_then_finalize_small() {
        let mut writer = BitBufferWrite::new();
        writer.write(0x5, 3); // 101
        writer.finalize();
        // 101_00000 = 0xA0
        assert_eq!(writer.get_data()[0], 0xA0);
    }

    #[test]
    fn write_32_bits() {
        let mut writer = BitBufferWrite::new();
        writer.write(0x12345678, 32);
        writer.finalize();
        let data = writer.get_data();
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0x56);
        assert_eq!(data[3], 0x78);
    }

    #[test]
    fn into_data_takes_ownership() {
        let mut writer = BitBufferWrite::new();
        writer.write(0xAB, 8);
        writer.finalize();
        let data = writer.into_data();
        assert_eq!(data[0], 0xAB);
    }

    #[test]
    fn roundtrip_write_then_read() {
        use super::super::bitbuffer_read::BitBufferRead;

        let mut writer = BitBufferWrite::new();
        // Write known values of various widths
        writer.write(0x7, 3); // 111
        writer.write(0x0, 1); // 0
        writer.write(0x15, 5); // 10101
        writer.write(0xAB, 8); // 10101011
        writer.write(0x3, 2); // 11
        writer.finalize();

        let data = writer.get_data();
        let mut reader = BitBufferRead::new(data);

        // Read back in same order
        assert_eq!(reader.read(3), 0x7);
        assert_eq!(reader.read(1), 0x0);
        assert_eq!(reader.read(5), 0x15);
        assert_eq!(reader.read(8), 0xAB);
        assert_eq!(reader.read(2), 0x3);
    }

    #[test]
    fn roundtrip_with_byte_stuffing() {
        use super::super::bitbuffer_read::BitBufferRead;

        // Test 1: 0xFF followed by normal data
        let mut writer = BitBufferWrite::new();
        writer.write(0xFF, 8);
        writer.write(0x55, 8);
        writer.finalize();

        let data = writer.get_data();
        assert_eq!(data[0], 0xFF);
        assert_eq!(data[1] & 0x80, 0); // MSB forced to 0 after 0xFF

        let mut reader = BitBufferRead::new(data);
        assert_eq!(reader.read(8), 0xFF);
        assert_eq!(reader.read(8), 0x55);

        // Test 2: multiple 0xFF bytes
        let mut writer = BitBufferWrite::new();
        writer.write(0xFF, 8);
        writer.write(0xFF, 8);
        writer.write(0xAA, 8);
        writer.finalize();

        let data = writer.get_data();
        let mut reader = BitBufferRead::new(data);
        assert_eq!(reader.read(8), 0xFF);
        assert_eq!(reader.read(8), 0xFF);
        assert_eq!(reader.read(8), 0xAA);

        // Test 3: 0xFF with mixed bit-width writes
        let mut writer = BitBufferWrite::new();
        writer.write(0xFF, 8);
        writer.write(0x5, 3); // 101
        writer.write(0xA, 4); // 1010
        writer.finalize();

        let data = writer.get_data();
        let mut reader = BitBufferRead::new(data);
        assert_eq!(reader.read(8), 0xFF);
        assert_eq!(reader.read(3), 0x5);
        assert_eq!(reader.read(4), 0xA);
    }

    #[test]
    fn default_trait() {
        let writer = BitBufferWrite::default();
        assert!(writer.is_empty());
    }
}
