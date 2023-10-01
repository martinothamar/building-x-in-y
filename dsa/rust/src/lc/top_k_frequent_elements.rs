struct Solution;

impl Solution {
    pub fn top_k_frequent(nums: Vec<i32>, k: i32) -> Vec<i32> {
        use std::collections::BinaryHeap;
        use std::collections::HashMap;

        let mut count_by_num = HashMap::with_capacity(nums.len());
        let mut heap: BinaryHeap<(i32, i32)> = BinaryHeap::with_capacity(k as usize);

        for n in nums {
            let entry = count_by_num.entry(n).or_insert(0);
            *entry += 1;
        }

        for (k, v) in count_by_num {
            heap.push((v, k));
        }

        (0..k).map(|_| heap.pop().unwrap().1).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = vec![1, 1, 1, 2, 2, 3];
        let k = 2;

        let mut expected = vec![1, 2];
        let mut result = Solution::top_k_frequent(input, k);
        expected.sort();
        result.sort();

        assert_eq!(result, expected);
    }
}
