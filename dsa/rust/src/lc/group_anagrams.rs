struct Solution;

impl Solution {
    pub fn group_anagrams(strs: Vec<String>) -> Vec<Vec<String>> {
        use std::collections::HashMap;

        let mut curr = AlphabetMap::new();

        let mut anagrams: HashMap<AlphabetMap, Vec<String>> = HashMap::with_capacity(strs.len());

        for str in &strs {
            for &c in str.as_bytes() {
                curr.increment(c);
            }

            let record = anagrams.entry(curr.clone()).or_insert_with(|| Vec::with_capacity(1));
            record.push(str.to_string());

            curr.reset();
        }

        anagrams.into_values().collect()
    }
}

#[derive(Eq, PartialEq, Hash, Clone)]
struct AlphabetMap {
    data: [u8; 26],
}

impl AlphabetMap {
    fn new() -> Self {
        Self { data: [0u8; 26] }
    }

    fn increment(&mut self, key: u8) {
        let idx = key as usize - 97;
        self.data[idx] += 1;
    }

    fn reset(&mut self) {
        self.data = [0u8; 26];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input = vec!["eat", "tea", "tan", "ate", "nat", "bat"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut expected: Vec<Vec<String>> = vec![vec!["eat", "tea", "ate"], vec!["tan", "nat"], vec!["bat"]]
            .into_iter()
            .map(|v| v.into_iter().map(|s| s.to_string()).collect())
            .collect();
        expected.sort();

        let mut result = Solution::group_anagrams(input);
        result.sort();

        assert_eq!(result, expected);
    }
}
