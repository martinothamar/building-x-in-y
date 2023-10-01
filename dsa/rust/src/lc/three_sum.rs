struct Solution;

impl Solution {
    pub fn three_sum(mut nums: Vec<i32>) -> Vec<Vec<i32>> {
        let mut results = Vec::new();

        nums.sort_unstable();

        for (i, &ni) in nums.iter().enumerate() {
            if i > 0 && ni == nums[i - 1] {
                continue;
            }

            let mut j = i + 1;
            let mut k = nums.len() - 1;

            while j < k {
                let nj = nums[j];
                let nk = nums[k];

                let curr = ni + nj + nk;
                match curr.cmp(&0) {
                    std::cmp::Ordering::Less => j += 1,
                    std::cmp::Ordering::Equal => {
                        results.push(vec![ni, nj, nk]);
                        j += 1;
                        while nums[j] == nums[j - 1] && j < k {
                            j += 1;
                        }
                    }
                    std::cmp::Ordering::Greater => k -= 1,
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = vec![-1, 0, 1, 2, -1, -4];
        let expected = vec![vec![-1, -1, 2], vec![-1, 0, 1]];

        let result = Solution::three_sum(input);
        assert_eq!(result, expected);
    }
}
