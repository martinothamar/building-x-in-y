struct Solution;

impl Solution {
    pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
        use std::collections::HashMap;

        let mut num_by_diff: HashMap<i32, i32> = HashMap::new();

        for (i, &n) in nums.iter().enumerate() {
            let diff = target - n;

            match num_by_diff.get(&diff) {
                Some(&idx) => return vec![idx, i as i32],
                None => num_by_diff.insert(n, i as i32),
            };
        }

        unreachable!("All inputs should have 1 answer")
    }
}
