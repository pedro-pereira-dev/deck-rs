pub fn move_selection(selected: usize, count: usize, direction: isize) -> usize {
    if count == 0 { 0 } else { (selected as isize + direction).rem_euclid(count as isize) as usize }
}
