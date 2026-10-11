//! Fixed SHA-256 (FIPS 180-4), independent of host hashers and dependencies.

const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];
const ROUND: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

fn compress(state: &mut [u32; 8], block: &[u8]) {
    let mut words = [0_u32; 64];
    for (word, bytes) in words.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_be_bytes(*bytes);
    }
    for i in 16..64 {
        let x = words[i - 15];
        let y = words[i - 2];
        let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
        let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
        words[i] = words[i - 16]
            .wrapping_add(s0)
            .wrapping_add(words[i - 7])
            .wrapping_add(s1);
    }
    let [
        mut reg_a,
        mut reg_b,
        mut reg_c,
        mut reg_d,
        mut reg_e,
        mut reg_f,
        mut reg_g,
        mut reg_h,
    ] = *state;
    for (word, constant) in words.into_iter().zip(ROUND) {
        let s1 = reg_e.rotate_right(6) ^ reg_e.rotate_right(11) ^ reg_e.rotate_right(25);
        let choose = (reg_e & reg_f) ^ (!reg_e & reg_g);
        let t1 = reg_h
            .wrapping_add(s1)
            .wrapping_add(choose)
            .wrapping_add(constant)
            .wrapping_add(word);
        let s0 = reg_a.rotate_right(2) ^ reg_a.rotate_right(13) ^ reg_a.rotate_right(22);
        let majority = (reg_a & reg_b) ^ (reg_a & reg_c) ^ (reg_b & reg_c);
        let t2 = s0.wrapping_add(majority);
        reg_h = reg_g;
        reg_g = reg_f;
        reg_f = reg_e;
        reg_e = reg_d.wrapping_add(t1);
        reg_d = reg_c;
        reg_c = reg_b;
        reg_b = reg_a;
        reg_a = t1.wrapping_add(t2);
    }
    for (part, added) in state
        .iter_mut()
        .zip([reg_a, reg_b, reg_c, reg_d, reg_e, reg_f, reg_g, reg_h])
    {
        *part = part.wrapping_add(added);
    }
}

/// SHA-256 bytes, with the standard big-endian bit length and padding.
#[must_use]
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut state = INITIAL;
    let (chunks, tail) = bytes.as_chunks::<64>();
    for block in chunks {
        compress(&mut state, block);
    }
    let mut padding = [0_u8; 128];
    padding[..tail.len()].copy_from_slice(tail);
    padding[tail.len()] = 0x80;
    let length = if tail.len() < 56 { 64 } else { 128 };
    let bit_length = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    padding[length - 8..length].copy_from_slice(&bit_length.to_be_bytes());
    for block in padding[..length].as_chunks::<64>().0 {
        compress(&mut state, block);
    }
    let mut result = [0_u8; 32];
    for (out, word) in result.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        out.copy_from_slice(&word.to_be_bytes());
    }
    result
}

/// First 80 SHA-256 bits, big-endian, in sixteen zero-padded base-37 digits.
#[must_use]
pub fn fingerprint(bytes: &[u8]) -> String {
    fingerprint_with_bits(bytes, FingerprintBits::PRODUCTION)
}

/// A nonzero prefix width bounded by the production fingerprint's 80 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FingerprintBits(u8);

impl FingerprintBits {
    pub const PRODUCTION: Self = Self(80);

    #[must_use]
    pub const fn new(bits: u8) -> Option<Self> {
        if bits > 0 && bits <= 80 {
            Some(Self(bits))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn digits(self) -> usize {
        let maximum = (1_u128 << self.0) - 1;
        let mut capacity = 37_u128;
        let mut digits = 1;
        while capacity <= maximum {
            capacity *= 37;
            digits += 1;
        }
        digits
    }
}

/// The same fixed digest/encoding at an explicitly supplied prefix width.
#[must_use]
pub fn fingerprint_with_bits(bytes: &[u8], bits: FingerprintBits) -> String {
    let digest = sha256(bytes);
    let value = digest[..10]
        .iter()
        .fold(0_u128, |value, byte| (value << 8) | u128::from(*byte));
    encode(value >> (80 - bits.0), bits.digits())
}

fn encode(mut value: u128, length: usize) -> String {
    const ALPHABET: &[u8; 37] = b"abcdefghijklmnopqrstuvwxyz0123456789_";
    let mut digits = vec![b'a'; length];
    for digit in digits.iter_mut().rev() {
        let index = usize::try_from(value % 37).unwrap_or_default();
        *digit = ALPHABET[index];
        value /= 37;
    }
    digits.into_iter().map(char::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_sha_goldens_cover_empty_single_and_multiple_blocks() {
        for (input, expected) in [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (
                "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
                "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
            ),
        ] {
            let actual: String = sha256(input.as_bytes())
                .iter()
                .flat_map(|byte| {
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    [
                        char::from(HEX[usize::from(byte >> 4)]),
                        char::from(HEX[usize::from(byte & 15)]),
                    ]
                })
                .collect();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn eighty_bits_keep_leading_zeroes_and_big_endian_order() {
        assert_eq!(encode(0, 16), "aaaaaaaaaaaaaaaa");
        assert_eq!(encode(37, 16), "aaaaaaaaaaaaaaba");
        assert_eq!(fingerprint(b"abc"), encode(0xba78_16bf_8f01_cfea_4141, 16));
    }

    #[test]
    fn nist_complete_block_golden_keeps_the_first_eighty_bits() {
        let input = vec![b'a'; 1_000_000];
        let digest = sha256(&input);
        assert_eq!(
            &digest[..10],
            &[0xcd, 0xc7, 0x6e, 0x5c, 0x99, 0x14, 0xfb, 0x92, 0x81, 0xa1]
        );
    }
}
