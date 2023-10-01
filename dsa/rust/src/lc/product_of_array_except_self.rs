struct Solution;

impl Solution {
    pub fn product_except_self(nums: Vec<i32>) -> Vec<i32> {
        let mut result = vec![0i32; nums.len()];

        let mut prefix = 1;
        for (i, &n) in nums.iter().enumerate() {
            result[i] = prefix;
            prefix *= n;
        }

        let mut suffix = 1;
        for (i, &n) in nums.iter().enumerate().rev() {
            result[i] *= suffix;
            suffix *= n;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = vec![1, 2, 3, 4];
        let expected = vec![24, 12, 8, 6];

        let result = Solution::product_except_self(input);
        assert_eq!(result, expected);
    }
}
