use std::convert::TryInto;

pub fn diff(left: &[u8], right: &[u8]) -> Vec<u8> {
    // TODO: left and right can't be more than std::u32::MAX / 8 each
    let mut xored = left.to_vec();
    xored.resize(right.len(), 0);
    for (l, r) in xored.iter_mut().zip(right.iter()) {
        *l = *l ^ *r;
    }
    // Empirically, left and right will have a different bit about once every 25 bytes.
    let mut set_bit_index_differences = Vec::with_capacity(xored.len() / 25);
    set_bit_index_differences.push(xored.len()); // include the length of the right side first
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

pub fn patch(left: &[u8], patch: &[u8]) -> Result<Vec<u8>, ()> {
    if left.len() >= (std::u32::MAX / 8) as usize {
        return Err(());
    }
    if patch.len() % 4 != 0 {
        return Err(());
    }
    let right_len = u32::from_be_bytes(patch[0..4].try_into().map_err(|_| ())?);
    if right_len >= std::u32::MAX / 8 {
        return Err(());
    }
    let mut right = left.to_vec();
    right.resize(right_len as usize, 0);
    let mut current_bit_index: u32 = 0;
    let bit_index_differences_bytes = patch[4..].chunks_exact(4);
    for bit_index_difference_bytes in bit_index_differences_bytes {
        let bit_index_difference =
            u32::from_be_bytes(bit_index_difference_bytes.try_into().map_err(|_| ())?);
        let new_current_bit_index = match current_bit_index.checked_add(bit_index_difference) {
            Some(result) => result,
            None => return Err(()),
        };
        flip_bit(&mut right, new_current_bit_index as usize)?;
        current_bit_index = new_current_bit_index;
    }
    Ok(right)
}

fn flip_bit(bytes: &mut [u8], bit_index: usize) -> Result<(), ()> {
    if bit_index >= bytes.len() * 8 {
        return Err(());
    }
    // TODO: some of this can be refactored from BitSlice
    let byte_index = bit_index / 8;
    let final_bit_index = bit_index % 8;
    let byte = &mut bytes[byte_index];
    let flip_pattern = match final_bit_index {
        0 => 0b00000001u8,
        1 => 0b00000010u8,
        2 => 0b00000100u8,
        3 => 0b00001000u8,
        4 => 0b00010000u8,
        5 => 0b00100000u8,
        6 => 0b01000000u8,
        7 => 0b10000000u8,
        _ => panic!("impossible final_bit_index value: {}", final_bit_index),
    };
    *byte = *byte ^ flip_pattern;
    Ok(())
}

// TODO: figure out sharing story between this and rust_cascade
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
    use crate::{diff, patch};

    #[test]
    fn test_diff_inputs_same_size() {
        let left = [0b1111_0000, 0b1010_1111, 0b0011_1100, 0b0111_0001];
        let right = [0b1100_0000, 0b1110_1111, 0b0011_1101, 0b0110_0001];
        // The xor of these values will be [0b0011_0000, 0b0100_0000, 0b0000_0001, 0b0001_0000].
        // The `BitSlice` implementation is big endian, so the least significant bit is on the
        // fartheset "right" of each byte, which means that the list of differences between set bits
        // is [4, 1, 9, 2, 12]. The length of the right side is 4, which will appear first in the
        // output.
        let actual = diff(&left, &right);
        let expected = vec![
            0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, 2, 0, 0, 0, 12,
        ];
        assert_eq!(actual, expected);

        let patched = patch(&left, &actual).unwrap();
        assert_eq!(patched, right.to_vec());
    }

    #[test]
    fn test_diff_first_bit_different() {
        let left = [0b1111_0001];
        let right = [0b1111_0000];
        let actual = diff(&left, &right);
        let expected = [0, 0, 0, 1, 0, 0, 0, 0];
        assert_eq!(actual, expected);

        let patched = patch(&left, &actual).unwrap();
        assert_eq!(patched, right.to_vec());
    }

    #[test]
    fn test_diff_readme_example() {
        let left = [0xff, 0xfa];
        let right = [0xff, 0xf8, 0x03];
        let actual = diff(&left, &right);
        let expected = vec![
            0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00,
            0x00, 0x01,
        ];
        assert_eq!(actual, expected);

        let patched = patch(&left, &actual).unwrap();
        assert_eq!(patched, right.to_vec());
    }

    #[test]
    fn test_diff_no_bits_different() {
        let left = [0b0110_0011, 0b1101_1000];
        let right = [0b0110_0011, 0b1101_1000];
        let actual = diff(&left, &right);
        let expected = vec![0, 0, 0, 2];
        assert_eq!(actual, expected);

        let patched = patch(&left, &actual).unwrap();
        assert_eq!(patched, right.to_vec());
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
        let expected = vec![0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 7, 0, 0, 0, 1, 0, 0, 0, 7];
        assert_eq!(actual, expected);

        let patched = patch(&left, &actual).unwrap();
        assert_eq!(patched, right.to_vec());
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
            0, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 7, 0, 0, 0, 1, 0,
            0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 4,
        ];
        assert_eq!(actual, expected);

        let patched = patch(&left, &actual).unwrap();
        assert_eq!(patched, right.to_vec());
    }

    #[test]
    fn test_patch_truncated() {
        let left = [0b0000_0000];
        let patch_bytes = [0, 0, 0, 4, 0, 0];
        assert!(patch(&left, &patch_bytes).is_err());
    }

    #[test]
    fn test_patch_bit_index_out_of_range() {
        let left = [0b0000_0000];
        let patch_bytes = [0, 0, 0, 1, 0, 0, 0, 20];
        assert!(patch(&left, &patch_bytes).is_err());
    }

    #[test]
    fn test_patch_bit_index_overflow() {
        let left = [0b0000_0000];
        let patch_bytes = [0, 0, 0, 1, 0, 0, 0, 1, 255, 255, 255, 255];
        assert!(patch(&left, &patch_bytes).is_err());
    }

    #[test]
    fn test_patch_too_long() {
        let left = [0b0000_0000];
        // TODO: throw a specific error and check for it here?
        let patch_bytes = [255, 255, 255, 253];
        assert!(patch(&left, &patch_bytes).is_err());
    }
}
