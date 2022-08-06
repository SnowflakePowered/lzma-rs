use std::ops::{Index, IndexMut};

/// A 2 dimensional matrix in row-major order backed by a contiguous `Vec`
#[derive(Debug)]
pub struct Vec2D<T> {
    data: Box<[T]>,
    rows: usize,
    cols: usize,
}

impl<T> Vec2D<T> {
    /// Initialize a grid of size (`rows`, `cols`) with the given data element.
    pub fn init(data: T, size: (usize, usize)) -> Vec2D<T>
    where
        T: Clone,
    {
        let (rows, cols) = size;
        let len = rows
            .checked_mul(cols)
            .unwrap_or_else(|| panic!("{} rows by {} cols exceeds usize::MAX", rows, cols));
        Vec2D {
            data: vec![data; len].into_boxed_slice(),
            rows,
            cols,
        }
    }

    /// Fills the grid with elements by cloning `value`.
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.data.fill(value)
    }
}

impl<T> Index<usize> for Vec2D<T> {
    type Output = [T];

    fn index(&self, row: usize) -> &Self::Output {
        if row < self.rows {
            let start_row = row * self.cols;
            &self.data[start_row..start_row + self.cols]
        } else {
            panic!("row index {:?} out of bounds.", row);
        }
    }
}

impl<T> IndexMut<usize> for Vec2D<T> {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        if row < self.rows {
            let start_row = row * self.cols;
            &mut self.data[start_row..start_row + self.cols]
        } else {
            panic!("row index {:?} out of bounds.", row);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vec2d_init() {
        let vec2d = Vec2D::init(1, (2, 3));
        assert_eq!(vec2d[0], [1, 1, 1]);
        assert_eq!(vec2d[1], [1, 1, 1]);
    }

    #[test]
    fn vec2d_fill() {
        let mut vec2d = Vec2D::init(0, (2, 3));
        vec2d.fill(7);
        assert_eq!(vec2d[0], [7, 7, 7]);
        assert_eq!(vec2d[1], [7, 7, 7]);
    }

    #[test]
    fn vec2d_index() {
        let vec2d = Vec2D {
            data: vec![0, 1, 2, 3, 4, 5, 6, 7, 8].into_boxed_slice(),
            rows: 3,
            cols: 3,
        };
        assert_eq!(vec2d[0], [0, 1, 2]);
        assert_eq!(vec2d[1], [3, 4, 5]);
        assert_eq!(vec2d[2], [6, 7, 8]);
    }

    #[test]
    fn vec2d_index_mut() {
        let mut vec2d = Vec2D {
            data: vec![0, 1, 2, 3, 4, 5, 6, 7, 8].into_boxed_slice(),
            rows: 3,
            cols: 3,
        };

        vec2d[1][1] = 9;
        assert_eq!(vec2d[0], [0, 1, 2]);
        assert_eq!(vec2d[1], [3, 9, 5]);
        assert_eq!(vec2d[2], [6, 7, 8]);
    }

    #[test]
    #[should_panic]
    fn vec2d_index_out_of_bounds() {
        let vec2d = Vec2D::init(1, (2, 3));
        let _x = vec2d[2][4];
    }

    #[test]
    #[should_panic]
    fn vec2d_index_out_of_bounds_vec_edge() {
        let vec2d = Vec2D::init(1, (2, 3));
        let _x = vec2d[1][3];
    }

    #[test]
    #[should_panic]
    fn vec2d_index_out_of_bounds_overflow() {
        let vec2d = Vec2D::init(1, (2, 3));
        let _x = vec2d[0][3];
    }

    #[test]
    #[should_panic]
    fn vec2d_indexmut_out_of_bounds_vec_edge() {
        let mut vec2d = Vec2D::init(1, (2, 3));
        vec2d[1][3] = 0;
    }

    #[test]
    #[should_panic]
    fn vec2d_indexmut_out_of_bounds_overflow() {
        let mut vec2d = Vec2D::init(1, (2, 3));
        vec2d[0][3] = 0;
    }

    #[test]
    #[should_panic]
    fn vec2d_indexmut_out_of_bounds() {
        let mut vec2d = Vec2D::init(1, (2, 3));
        vec2d[2][4] = 0;
    }
}
