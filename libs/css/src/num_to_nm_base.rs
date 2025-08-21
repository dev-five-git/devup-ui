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
    #[case(26, "_")]
    #[case(27, "aa")]
    #[case(28, "ab")]
    #[case(29, "ac")]
    #[case(30, "ad")]
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
    #[case(63, "a-")]
    #[case(64, "a_")]
    #[case(65, "ba")]
    #[case(66, "bb")]
    #[case(67, "bc")]
    #[case(68, "bd")]
    #[case(69, "be")]
    #[case(70, "bf")]
    #[case(71, "bg")]
    #[case(72, "bh")]
    #[case(73, "bi")]
    #[case(74, "bj")]
    #[case(75, "bk")]
    #[case(76, "bl")]
    #[case(77, "bm")]
    #[case(78, "bn")]
    #[case(79, "bo")]
    #[case(80, "bp")]
    #[case(81, "bq")]
    #[case(82, "br")]
    #[case(83, "bs")]
    #[case(84, "bt")]
    #[case(85, "bu")]
    #[case(86, "bv")]
    #[case(87, "bw")]
    #[case(88, "bx")]
    #[case(89, "by")]
    #[case(90, "bz")]
    #[case(91, "b0")]
    #[case(92, "b1")]
    #[case(1052, "__")]
    #[case(1053, "aaa")]
    fn test_num_to_nm_base_rstest(#[case] input: usize, #[case] expected: &str) {
        assert_eq!(num_to_nm_base(input), expected);
    }
}
