struct Solution;

const TABLE_SIZE: usize = 126 - 40;
const TABLE_DEC: usize = 40;
const IS_OPENING: [bool; TABLE_SIZE] = {
    let mut lookup = [false; TABLE_SIZE];
    lookup['(' as usize - TABLE_DEC] = true;
    lookup['{' as usize - TABLE_DEC] = true;
    lookup['[' as usize - TABLE_DEC] = true;

    lookup
};
const CLOSING_TOKENS: [char; TABLE_SIZE] = {
    let mut lookup = [' '; TABLE_SIZE];
    lookup['(' as usize - TABLE_DEC] = ')';
    lookup['{' as usize - TABLE_DEC] = '}';
    lookup['[' as usize - TABLE_DEC] = ']';

    lookup
};

const fn is_opening(c: char) -> bool {
    IS_OPENING[c as usize - TABLE_DEC]
}

const fn get_closing(c: char) -> char {
    CLOSING_TOKENS[c as usize - TABLE_DEC]
}

impl Solution {
    pub fn is_valid(s: String) -> bool {
        if s.is_empty() || s.len() == 1 || s.len() % 2 != 0 {
            return false;
        }

        let mut stack = Vec::new();

        for c in s.chars() {
            if is_opening(c) {
                stack.push(get_closing(c));
            } else {
                match stack.pop() {
                    Some(expected_closing) => {
                        if c != expected_closing {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }

        stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = "()".to_string();
        let expected = true;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_2() {
        let input = "()[]{}".to_string();
        let expected = true;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_3() {
        let input = "(]".to_string();
        let expected = false;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_4() {
        let input = "](".to_string();
        let expected = false;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_5() {
        let input = "]".to_string();
        let expected = false;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_6() {
        let input = "(".to_string();
        let expected = false;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_7() {
        let input = "".to_string();
        let expected = false;
        assert_eq!(Solution::is_valid(input), expected);
    }

    #[test]
    fn test_case_8() {
        let input = "((".to_string();
        let expected = false;
        assert_eq!(Solution::is_valid(input), expected);
    }
}
