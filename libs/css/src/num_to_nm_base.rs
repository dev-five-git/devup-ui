use crate::constant::{M_BASE_ARRAY, N_BASE_ARRAY};

pub fn num_to_nm_base(num: usize) -> String {
    if num == 0 {
        return N_BASE_ARRAY[0].to_string();
    }

    let mut n = num;
    let mut result = String::new();

    let first_base = N_BASE_ARRAY.len();
    let other_base = M_BASE_ARRAY.len();

    while n > 0 {
        if n < first_base {
            result.insert(0, N_BASE_ARRAY[n]);
            break;
        } else {
            result.insert(0, M_BASE_ARRAY[(n - first_base) % other_base]);
            n = (n - first_base) / other_base;
            if n == 0 {
                result.insert(0, N_BASE_ARRAY[0]);
                break;
            }
        }
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
    #[case(26, "aa")]
    #[case(27, "ab")]
    #[case(28, "ac")]
    #[case(29, "ad")]
    #[case(30, "ae")]
    #[case(31, "af")]
    #[case(32, "ag")]
    #[case(33, "ah")]
    #[case(34, "ai")]
    #[case(35, "aj")]
    #[case(36, "ak")]
    #[case(37, "al")]
    #[case(38, "am")]
    #[case(39, "an")]
    #[case(40, "ao")]
    #[case(41, "ap")]
    #[case(42, "aq")]
    #[case(43, "ar")]
    #[case(44, "as")]
    #[case(45, "at")]
    #[case(46, "au")]
    #[case(47, "av")]
    #[case(48, "aw")]
    #[case(49, "ax")]
    #[case(50, "ay")]
    #[case(51, "az")]
    #[case(52, "a0")]
    #[case(53, "a1")]
    #[case(54, "a2")]
    #[case(55, "a3")]
    #[case(56, "a4")]
    #[case(57, "a5")]
    #[case(58, "a6")]
    #[case(59, "a7")]
    #[case(60, "a8")]
    #[case(61, "a9")]
    #[case(62, "ba")]
    #[case(63, "bb")]
    #[case(64, "bc")]
    #[case(65, "bd")]
    #[case(66, "be")]
    #[case(67, "bf")]
    #[case(68, "bg")]
    #[case(69, "bh")]
    #[case(70, "bi")]
    #[case(71, "bj")]
    #[case(72, "bk")]
    #[case(73, "bl")]
    #[case(74, "bm")]
    #[case(75, "bn")]
    #[case(76, "bo")]
    #[case(77, "bp")]
    #[case(78, "bq")]
    #[case(79, "br")]
    #[case(80, "bs")]
    #[case(81, "bt")]
    #[case(82, "bu")]
    #[case(83, "bv")]
    #[case(84, "bw")]
    #[case(85, "bx")]
    #[case(86, "by")]
    #[case(87, "bz")]
    #[case(88, "b0")]
    #[case(89, "b1")]
    #[case(961, "z9")]
    #[case(962, "aaa")]
    fn test_num_to_nm_base_rstest(#[case] input: usize, #[case] expected: &str) {
        assert_eq!(num_to_nm_base(input), expected);
    }
}
