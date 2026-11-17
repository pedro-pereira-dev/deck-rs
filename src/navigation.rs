pub struct Position {
    pub category: usize,
    pub rows: Vec<usize>,
}

impl Position {
    pub fn new(category: usize) -> Self {
        Self { category, rows: vec![0] }
    }

    pub fn depth(&self) -> usize {
        self.rows.len() - 1
    }

    pub fn go_to_category(&mut self, category: usize) {
        *self = Self::new(category);
    }

    pub fn move_category(&mut self, count: usize, direction: isize) {
        if count > 0 {
            let category = (self.category as isize + direction).clamp(0, count as isize - 1) as usize;
            self.go_to_category(category);
        }
    }

    pub fn move_row(&mut self, count: usize, direction: isize) {
        let depth = self.depth();
        self.rows[depth] = (self.rows[depth] as isize + direction).clamp(0, count.max(1) as isize - 1) as usize;
    }

    pub fn enter(&mut self) {
        self.rows.push(0);
    }

    pub fn go_back(&mut self) -> bool {
        let can_go_back = self.depth() > 0;
        if can_go_back {
            self.rows.pop();
        }
        can_go_back
    }

    pub fn pick(&mut self, column: usize, index: usize) {
        self.rows.truncate(column + 1);
        self.rows[column] = index;
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn rows_clamp_to_the_column() {
        let mut position = Position::new(0);
        position.move_row(3, -1);
        assert_eq!(position.rows, [0]);
        position.move_row(3, 5);
        assert_eq!(position.rows, [2]);
    }

    #[test]
    fn enter_and_back_change_depth() {
        let mut position = Position::new(1);
        position.enter();
        assert_eq!(position.depth(), 1);
        assert!(position.go_back());
        assert!(!position.go_back());
    }

    #[test]
    fn pick_drops_deeper_columns() {
        let mut position = Position::new(0);
        position.enter();
        position.pick(0, 2);
        assert_eq!(position.rows, [2]);
    }
}
