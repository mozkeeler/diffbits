pub fn diff(left: &[u8], right: &[u8]) -> Vec<u8> {
    // TODO: left and right can't be more than std::u32::MAX each
    let mut xored = left.to_vec();
    xored.resize(right.len(), 0);
    for (l, r) in xored.iter_mut().zip(right.iter()) {
        *l = *l ^ *r;
    }
    // Empirically, left and right will have a different bit about once every 25 bytes.
    let mut set_bit_index_differences = Vec::with_capacity(xored.len() / 25);
    let set_bits = BitSlice::new(&xored, xored.len() * 8);
    let mut previous_set_index = 0;
    for index in 0..set_bits.bit_len {
        if set_bits.get(index) {
            set_bit_index_differences.push(index - previous_set_index);
            previous_set_index = index;
        }
    }
    let set_bit_index_differences_bytes_bytes: Vec<Vec<u8>> = set_bit_index_differences
        .iter()
        .map(integer_to_bytes)
        .collect();
    let set_bit_index_differences_bytes = set_bit_index_differences_bytes_bytes.concat();
    set_bit_index_differences_bytes
}

fn integer_to_bytes(i: &usize) -> Vec<u8> {
    assert!(*i <= std::u32::MAX as usize);
    i.to_be_bytes()[4..8].to_vec()
}

/// Helper struct to provide bit access to a slice of bytes.
struct BitSlice<'a> {
    /// The slice of bytes we're interested in.
    bytes: &'a [u8],
    /// The number of bits that are valid to access in the slice.
    /// Not necessarily equal to `bytes.len() * 8`, but it will not be greater than that.
    bit_len: usize,
}

impl<'a> BitSlice<'a> {
    /// Creates a new `BitSlice` of the given bit length over the given slice of data.
    /// Panics if the indicated bit length is larger than fits in the slice.
    ///
    /// # Arguments
    /// * `bytes` - The slice of bytes we need bit-access to
    /// * `bit_len` - The number of bits that are valid to access in the slice
    fn new(bytes: &'a [u8], bit_len: usize) -> BitSlice<'a> {
        if bit_len > bytes.len() * 8 {
            panic!(
                "bit_len too large for given data: {} > {} * 8",
                bit_len,
                bytes.len()
            );
        }
        BitSlice { bytes, bit_len }
    }

    /// Get the value of the specified bit.
    /// Panics if the specified bit is out of range for the number of bits in this instance.
    ///
    /// # Arguments
    /// * `bit_index` - The bit index to access
    fn get(&self, bit_index: usize) -> bool {
        if bit_index >= self.bit_len {
            panic!(
                "bit index out of range for bit slice: {} >= {}",
                bit_index, self.bit_len
            );
        }
        let byte_index = bit_index / 8;
        let final_bit_index = bit_index % 8;
        let byte = self.bytes[byte_index];
        let test_value = match final_bit_index {
            0 => byte & 0b00000001u8,
            1 => byte & 0b00000010u8,
            2 => byte & 0b00000100u8,
            3 => byte & 0b00001000u8,
            4 => byte & 0b00010000u8,
            5 => byte & 0b00100000u8,
            6 => byte & 0b01000000u8,
            7 => byte & 0b10000000u8,
            _ => panic!("impossible final_bit_index value: {}", final_bit_index),
        };
        test_value > 0
    }
}

#[cfg(test)]
mod tests {
    use crate::diff;

    #[test]
    fn test_diff_inputs_same_size() {
        let left = [0b1111_0000, 0b1010_1111, 0b0011_1100, 0b0111_0001];
        let right = [0b1100_0000, 0b1110_1111, 0b0011_1101, 0b0110_0001];
        // The xor of these values will be [0b0011_0000, 0b0100_0000, 0b0000_0001, 0b0001_0000].
        // The `BitSlice` implementation is big endian, so the least significant bit is on the
        // fartheset "right" of each byte, which means that the list of differences between set bits
        // is [4, 1, 9, 2, 12].
        let actual = diff(&left, &right);
        let expected = vec![0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, 2, 0, 0, 0, 12];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_diff_first_bit_different() {
        let left = [0b1111_0001];
        let right = [0b1111_0000];
        let actual = diff(&left, &right);
        let expected = [0, 0, 0, 0];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_diff_no_bits_different() {
        let left = [0b0110_0011, 0b1101_1000];
        let right = [0b0110_0011, 0b1101_1000];
        let actual = diff(&left, &right);
        let expected = vec![];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_diff_left_longer() {
        let left = [
            0b1101_1111,
            0b0000_0000,
            0b0110_0000,
            0b1010_0111,
            0b0001_0001,
        ];
        let right = [0b1101_1101, 0b0000_0011, 0b0110_0001];
        // The xor of these values, truncated to the length of the right side, will be
        // [0b0000_0010, 0b0000_0011, 0b0000_0001]. The list of differences between set bits will be
        // [1, 7, 1, 7].
        let actual = diff(&left, &right);
        let expected = vec![0, 0, 0, 1, 0, 0, 0, 7, 0, 0, 0, 1, 0, 0, 0, 7];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_diff_right_longer() {
        let left = [0b1001_1011, 0b1110_0011, 0b0111_0001];
        let right = [
            0b1001_1111,
            0b0111_0011,
            0b0111_0011,
            0b1010_0111,
            0b0001_0001,
        ];
        // The xor of these values, extended to the length of the right side, will be
        // [0b0000_0100, 0b1001_0000, 0b0000_0010, 0b1010_0111, 0b0001_0001]. The list of
        // differences between set bits will be [2, 10, 3, 2, 7, 1, 1, 3, 2, 1, 4].
        let actual = diff(&left, &right);
        let expected = vec![
            0, 0, 0, 2, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 7, 0, 0, 0, 1, 0, 0, 0, 1, 0,
            0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 4,
        ];
        assert_eq!(actual, expected);
    }
}
