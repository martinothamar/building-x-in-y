struct Solution;

impl Solution {
    pub fn is_palindrome(s: String) -> bool {
        if s == " " {
            return true;
        }

        let mut start = 0;
        let mut end = s.len() - 1;

        let s = s.as_bytes();

        while start < end {
            if !(s[start] as char).is_alphanumeric() {
                start += 1;
            } else if !(s[end] as char).is_alphanumeric() {
                end -= 1;
            } else {
                let sc = s[start] as char;
                let ec = s[end] as char;

                if !sc.eq_ignore_ascii_case(&ec) {
                    return false;
                }

                start += 1;
                end -= 1;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = "A man, a plan, a canal: Panama".to_string();
        let expected = true;
        assert_eq!(Solution::is_palindrome(input), expected);
    }

    #[test]
    fn test_case_2() {
        let input = "race a car".to_string();
        let expected = false;
        assert_eq!(Solution::is_palindrome(input), expected);
    }

    #[test]
    fn test_case_3() {
        let input = "ma d am".to_string();
        let expected = true;
        assert_eq!(Solution::is_palindrome(input), expected);
    }
}
