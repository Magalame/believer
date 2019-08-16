use crate::GF2;

pub struct ParityCheckMatrix {
    row_ranges: Vec<usize>,
    column_indices: Vec<usize>,
}

impl ParityCheckMatrix {
    /// Creates a new `ParityCheckMatrix` from a list of `(row, col)` where
    /// each tuple is the position of a non zero element.
    ///
    /// # Example
    ///
    /// ```
    /// # use::believer::ParityCheckMatrix;
    /// // The parity check matrix of a 3 bits repetition code.
    /// let parity_check = ParityCheckMatrix::new(vec![
    ///     (0, 0),
    ///     (0, 1),    
    ///     (1, 1),
    ///     (1, 2),
    /// ]);
    /// ```
    pub fn new(positions: Vec<(usize, usize)>) -> Self {
        let mut column_indices = Vec::with_capacity(positions.len());
        let mut row_ranges = Vec::new();
        row_ranges.push(0);

        let mut active_row = 0;
        let mut row_lenght = 0;

        for (row, col) in positions.into_iter() {
            if row == active_row {
                row_lenght += 1;
            } else {
                while active_row < row {
                    row_ranges.push(*row_ranges.last().unwrap_or(&0) + row_lenght);
                    active_row += 1;
                }
                row_lenght = 1;
            }
            column_indices.push(col);
        }

        row_ranges.push(*row_ranges.last().unwrap_or(&0) + row_lenght);

        Self {
            row_ranges,
            column_indices,
        }
    }

    pub fn row_slice(&self, row: usize) -> Option<Slice> {
        self.row_ranges.get(row).and_then(|&row_start| {
            self.row_ranges.get(row + 1).map(|&row_end| {
                Slice {
                    positions: &self.column_indices[row_start..row_end]
                }
            })
        })
    }
}

pub struct Slice<'a> {
    positions: &'a [usize],
}

impl<'a> Slice<'a> {
    pub fn dot(&self, other: &[GF2]) -> GF2 {
        let mut total = GF2::B0;
        self.positions.iter().for_each(|&pos| {
            other.get(pos).map(|&value| total = total + value);
        });
        total
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn dot_product() {
        let parity_check = ParityCheckMatrix::new(vec![
            (0, 0),
            (0, 1),    
            (1, 1),
            (1, 2),
        ]);   
        let bits = vec![GF2::B0, GF2::B1, GF2::B1];
        
        assert_eq!(parity_check.row_slice(0).unwrap().dot(&bits), GF2::B1);
        assert_eq!(parity_check.row_slice(1).unwrap().dot(&bits), GF2::B0);
    }
}