struct Solution;

impl Solution {
    pub fn max_area(height: Vec<i32>) -> i32 {
        let mut l = 0;
        let mut r = height.len() - 1;

        let mut max_area = 0;
        while l < r {
            let lowest = height[l].min(height[r]);
            let area = (r as i32 - l as i32) * lowest;

            max_area = area.max(max_area);

            if height[l] == lowest {
                l += 1;
            } else {
                r -= 1;
            }
        }

        max_area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let height = vec![1, 8, 6, 2, 5, 4, 8, 3, 7];
        let expected = 49;

        let result = Solution::max_area(height);
        assert_eq!(result, expected);
    }
}
