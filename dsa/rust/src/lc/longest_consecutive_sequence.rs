struct Solution;

impl Solution {
    pub fn longest_consecutive(nums: Vec<i32>) -> i32 {
        use std::collections::HashSet;

        if nums.is_empty() {
            return 0;
        }
        if nums.len() == 1 {
            return 1;
        }

        let set: HashSet<_> = nums.into_iter().collect();
        let mut max_count = 0;

        for &num in &set {
            if set.contains(&(num - 1)) {
                continue;
            }

            let count = (num..).take_while(|num| set.contains(num)).count();
            max_count = max_count.max(count);
        }
        max_count as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = vec![100, 4, 200, 1, 3, 2];
        let expected = 4;

        let result = Solution::longest_consecutive(input);
        assert_eq!(result, expected);
    }
}
