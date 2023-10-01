struct Solution;

impl Solution {
    pub fn is_anagram(s: String, t: String) -> bool {
        if s.len() != t.len() {
            return false;
        }

        let mut s_count = AlphabetMap::new();
        let mut t_count = AlphabetMap::new();

        let s = s.as_bytes();
        let t = t.as_bytes();

        for i in 0..s.len() {
            let sc = s[i];
            let tc = t[i];

            s_count.increment(sc);
            t_count.increment(tc);
        }

        s_count == t_count
    }
}

#[derive(Eq, PartialEq)]
struct AlphabetMap {
    data: [usize; 26],
}

impl AlphabetMap {
    fn new() -> Self {
        Self { data: [0; 26] }
    }

    fn increment(&mut self, key: u8) {
        let idx = key as usize - 97;
        self.data[idx] += 1;
    }
}
