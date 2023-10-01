struct Solution;

impl Solution {
    pub fn two_sum(numbers: Vec<i32>, target: i32) -> Vec<i32> {
        let mut low = 0;
        let mut high = numbers.len() - 1;

        while low < high {
            match (numbers[low] + numbers[high]).cmp(&target) {
                std::cmp::Ordering::Less => low += 1,
                std::cmp::Ordering::Greater => high -= 1,
                std::cmp::Ordering::Equal => return vec![low as i32 + 1, high as i32 + 1],
            };
        }

        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let numbers = vec![2, 7, 11, 15];
        let target = 9;
        let expected = vec![1, 2];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_case_2() {
        let numbers = vec![2, 3, 4];
        let target = 6;
        let expected = vec![1, 3];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_case_3() {
        let numbers = vec![-1, 0];
        let target = -1;
        let expected = vec![1, 2];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_case_4() {
        let numbers = vec![5, 25, 75];
        let target = 100;
        let expected = vec![2, 3];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_case_5() {
        let numbers = vec![3, 24, 50, 79, 88, 150, 345];
        let target = 200;
        let expected = vec![3, 6];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_case_6() {
        let numbers = vec![-1000, -1, 0, 1];
        let target = 1;
        let expected = vec![3, 4];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_case_7() {
        let numbers = vec![1, 2, 3, 4, 4, 9, 56, 90];
        let target = 8;
        let expected = vec![4, 5];

        let result = Solution::two_sum(numbers, target);
        assert_eq!(result, expected);
    }
}
