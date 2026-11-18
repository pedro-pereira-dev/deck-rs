pub struct Position {
    pub category: usize,
    pub item: usize,
}

impl Position {
    pub fn new(category: usize) -> Self {
        Self { category, item: 0 }
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
        self.item = (self.item as isize + direction).rem_euclid(count.max(1) as isize) as usize;
    }

    pub fn pick(&mut self, index: usize) {
        self.item = index;
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn rows_wrap_within_the_column() {
        let mut position = Position::new(0);
        position.move_row(3, -1);
        assert_eq!(position.item, 2);
        position.move_row(3, 5);
        assert_eq!(position.item, 1);
    }

    #[test]
    fn categories_stop_at_the_ends() {
        let mut position = Position::new(0);
        position.move_category(3, -1);
        assert_eq!(position.category, 0);
        position.move_category(3, 1);
        assert_eq!(position.category, 1);
        position.move_category(3, 20);
        assert_eq!(position.category, 2);
    }

    #[test]
    fn pick_changes_selected_item() {
        let mut position = Position::new(0);
        position.pick(2);
        assert_eq!(position.item, 2);
    }
}
