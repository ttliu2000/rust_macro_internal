    /// Get the value of a bit at a specific index
    fn get_bit(&self, bit_idx: usize) -> bool {
        let byte_idx = bit_idx / 8;
        let bit_pos = bit_idx % 8;
        
        if byte_idx < self.data.len() {
            (self.data[byte_idx] & (1 << bit_pos)) != 0
        } else {
            false
        }
    }

    /// Set a range of bits to a specific value
    /// start_bit: the starting bit index (inclusive)
    /// end_bit: the ending bit index (inclusive)
    /// value: the value to set in the bit range
    pub fn set_bit_range_value(&mut self, start_bit: usize, end_bit: usize, value: u64) {
        let num_bits = end_bit - start_bit + 1;
        
        for i in 0..num_bits {
            let bit_idx = start_bit + i;
            let byte_idx = bit_idx / 8;
            let bit_pos = bit_idx % 8;
            
            if byte_idx < self.data.len() {
                // Extract the i-th bit from value
                let bit_value = (value >> i) & 1;
                
                if bit_value != 0 {
                    self.data[byte_idx] |= 1 << bit_pos;
                } else {
                    self.data[byte_idx] &= !(1 << bit_pos);
                }
            }
        }
    }

    /// Convert the internal bit vector to a little-endian byte array
    pub fn to_le_bytes(&self) -> Vec<u8> {
        self.data
            .iter()
            .rev()              // reverse byte order
            .cloned()
            .collect()
    }