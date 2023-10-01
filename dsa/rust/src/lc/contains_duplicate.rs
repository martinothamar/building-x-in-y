struct Solution;

impl Solution {
    // pub fn contains_duplicate(nums: Vec<i32>) -> bool {
    //     use std::collections::HashSet;

    //     let mut set = HashSet::with_capacity(nums.len());

    //     for n in &nums {
    //         if !set.insert(*n) {
    //             return true;
    //         }
    //     }

    //     false
    // }

    pub fn contains_duplicate(mut nums: Vec<i32>) -> bool {
        if nums.is_empty() || nums.len() == 1 {
            return false;
        }

        nums.sort_unstable();

        let mut prev: i32 = nums[0];

        for n in nums {
            if n == prev {
                return true;
            }

            prev = n;
        }

        false
    }
}
