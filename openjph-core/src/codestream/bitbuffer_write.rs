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
        while self.bits_used >= 8 {
            let byte = (self.buf >> 56) as u8;
            if self.unstuff {
                // After 0xFF, write only 7 bits (MSB is forced to 0)
                let val = byte & 0x7F;
                self.data.push(val);
                self.buf <<= 7;
                self.bits_used -= 7;
                self.unstuff = false;
            } else {
                self.data.push(byte);
                self.buf <<= 8;
                self.bits_used -= 8;
                self.unstuff = byte == 0xFF;
            }
        }
    }

    /// Flush all remaining bits, padding with zeros. Returns whether
    /// the final byte was 0xFF (requires an extra zero byte).
    pub fn finalize(&mut self) -> bool {
        // Pad remaining bits
        if self.bits_used > 0 || self.unstuff {
            let remaining = if self.unstuff { 7 } else { 8 };
            if self.bits_used > 0 {
                // Pad to a full byte boundary
                let pad = remaining - (self.bits_used % remaining);
                if pad < remaining {
                    self.bits_used += pad;
                }
            } else if self.unstuff {
                // Need to write a zero byte after 0xFF
                self.data.push(0);
                self.unstuff = false;
                return false;
            }
            self.flush_bytes();
        }
        // If the last byte written was 0xFF, append a 0x00
        if self.unstuff {
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
