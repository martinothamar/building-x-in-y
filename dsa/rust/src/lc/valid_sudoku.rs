struct Solution;

impl Solution {
    pub fn is_valid_sudoku(board: Vec<Vec<char>>) -> bool {
        let rows = board.len();
        assert!(rows > 0);
        let cols = board[0].len();
        assert!(rows == cols);
        assert!(rows < 16 && rows % 2 != 0);
        let sub_grids_split = rows / 3;
        let cells_in_sub_grid = sub_grids_split * 3;

        let mut row_set = vec![0u16; rows];
        let mut col_set = vec![0u16; cols];
        let mut sub_grid = vec![0u16; cells_in_sub_grid];

        for (i, row) in board.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                let n = match cell.to_digit(10) {
                    None => continue,
                    Some(n) => n,
                };

                let mask = 1 << n;

                let sub_grid_index = sub_grids_split * (i / 3) + (j / 3);
                if (row_set[i] & mask) | (col_set[j] & mask) | (sub_grid[sub_grid_index] & mask) != 0 {
                    return false;
                }

                row_set[i] |= mask;
                col_set[j] |= mask;
                sub_grid[sub_grid_index] |= mask;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_1() {
        let input: Vec<Vec<char>> = vec![
            vec!['5', '3', '.', '.', '7', '.', '.', '.', '.'],
            vec!['6', '.', '.', '1', '9', '5', '.', '.', '.'],
            vec!['.', '9', '8', '.', '.', '.', '.', '6', '.'],
            vec!['8', '.', '.', '.', '6', '.', '.', '.', '3'],
            vec!['4', '.', '.', '8', '.', '3', '.', '.', '1'],
            vec!['7', '.', '.', '.', '2', '.', '.', '.', '6'],
            vec!['.', '6', '.', '.', '.', '.', '2', '8', '.'],
            vec!['.', '.', '.', '4', '1', '9', '.', '.', '5'],
            vec!['.', '.', '.', '.', '8', '.', '.', '7', '9'],
        ];

        let result = Solution::is_valid_sudoku(input);
        assert!(result);
    }
}
