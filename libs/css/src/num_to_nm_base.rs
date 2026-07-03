use crate::constant::{M_BASE_ARRAY, N_BASE_ARRAY};

#[inline]
pub fn num_to_nm_base(num: usize) -> String {
    if num == 0 {
        return String::from(N_BASE_ARRAY[0] as char);
    }

    let first_base = N_BASE_ARRAY.len();
    let other_base = M_BASE_ARRAY.len();

    // Emit ASCII digits least-significant first into a stack buffer,
    // then build the string in reverse; usize::MAX needs at most 13 digits here.
    let mut buf = [0u8; 16];
    let mut len = 0;
    let mut n = num;

    while n > 0 {
        if n < first_base {
            buf[len] = N_BASE_ARRAY[n];
            len += 1;
            break;
        }
        buf[len] = M_BASE_ARRAY[(n - first_base) % other_base];
        len += 1;
        n = (n - first_base) / other_base;
        if n == 0 {
            buf[len] = N_BASE_ARRAY[0];
            len += 1;
            break;
        }
    }
    // Reversed bytes are all ASCII; build directly into a pre-sized String.
    // The class must never end in "ad" (avoids g-ad / google-ad blocking), so
    // when the final two chars would be "a","d" we splice a '-' between them.
    // Exact capacity: `len` bytes, plus 1 for the possible "a-d" fixup.
    let needs_fixup = len >= 2 && buf[0] == b'd' && buf[1] == b'a';
    let mut result = String::with_capacity(len + usize::from(needs_fixup));
    for (i, &b) in buf[..len].iter().rev().enumerate() {
        // buf is stored least-significant-first, so the reversed suffix "ad"
        // corresponds to buf[0]=='d' (last emitted char) preceded by buf[1]=='a'.
        if needs_fixup && i == len - 1 {
            result.push('-');
        }
        result.push(char::from(b));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0, "a")]
    #[case(1, "b")]
    #[case(2, "c")]
    #[case(3, "d")]
    #[case(4, "e")]
    #[case(5, "f")]
    #[case(6, "g")]
    #[case(7, "h")]
    #[case(8, "i")]
    #[case(9, "j")]
    #[case(10, "k")]
    #[case(11, "l")]
    #[case(12, "m")]
    #[case(13, "n")]
    #[case(14, "o")]
    #[case(15, "p")]
    #[case(16, "q")]
    #[case(17, "r")]
    #[case(18, "s")]
    #[case(19, "t")]
    #[case(20, "u")]
    #[case(21, "v")]
    #[case(22, "w")]
    #[case(23, "x")]
    #[case(24, "y")]
    #[case(25, "z")]
    #[case(26, "_")]
    #[case(27, "aa")]
    #[case(28, "ab")]
    #[case(29, "ac")]
    #[case(30, "a-d")]
    #[case(31, "ae")]
    #[case(32, "af")]
    #[case(33, "ag")]
    #[case(34, "ah")]
    #[case(35, "ai")]
    #[case(36, "aj")]
    #[case(37, "ak")]
    #[case(38, "al")]
    #[case(39, "am")]
    #[case(40, "an")]
    #[case(41, "ao")]
    #[case(42, "ap")]
    #[case(43, "aq")]
    #[case(44, "ar")]
    #[case(45, "as")]
    #[case(46, "at")]
    #[case(47, "au")]
    #[case(48, "av")]
    #[case(49, "aw")]
    #[case(50, "ax")]
    #[case(51, "ay")]
    #[case(52, "az")]
    #[case(53, "a0")]
    #[case(54, "a1")]
    #[case(55, "a2")]
    #[case(56, "a3")]
    #[case(57, "a4")]
    #[case(58, "a5")]
    #[case(59, "a6")]
    #[case(60, "a7")]
    #[case(61, "a8")]
    #[case(62, "a9")]
    #[case(63, "a_")]
    #[case(64, "ba")]
    #[case(65, "bb")]
    #[case(66, "bc")]
    #[case(67, "bd")]
    #[case(68, "be")]
    #[case(69, "bf")]
    #[case(70, "bg")]
    #[case(71, "bh")]
    #[case(72, "bi")]
    #[case(73, "bj")]
    #[case(74, "bk")]
    #[case(75, "bl")]
    #[case(76, "bm")]
    #[case(77, "bn")]
    #[case(78, "bo")]
    #[case(79, "bp")]
    #[case(80, "bq")]
    #[case(81, "br")]
    #[case(82, "bs")]
    #[case(83, "bt")]
    #[case(84, "bu")]
    #[case(85, "bv")]
    #[case(86, "bw")]
    #[case(87, "bx")]
    #[case(88, "by")]
    #[case(89, "bz")]
    #[case(90, "b0")]
    #[case(91, "b1")]
    #[case(987, "z9")]
    #[case(1026, "aaa")]
    fn test_num_to_nm_base_rstest(#[case] input: usize, #[case] expected: &str) {
        assert_eq!(num_to_nm_base(input), expected);
    }
}
