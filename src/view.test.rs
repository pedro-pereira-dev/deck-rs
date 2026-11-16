#[test]
fn columns_are_centered_and_spaced_by_depth() {
    assert_eq!(super::column_x(100.0, 1000.0, 0, 0.0), 400.0);
    assert_eq!(super::column_x(100.0, 1000.0, 1, 0.0), 750.0);
    assert_eq!(super::column_x(100.0, 1000.0, 1, 1.0), 400.0);
}

#[test]
fn selected_rows_are_larger_than_distant_rows() {
    let (selected_y, selected_size) = super::spot(100.0, 0, 2, 2.0);
    let (near_y, near_size) = super::spot(100.0, 0, 3, 2.0);
    assert_eq!(selected_y, 460.0);
    assert!(near_y > selected_y);
    assert!(selected_size > near_size);
}

#[test]
fn nested_columns_use_smaller_row_icons() {
    let (_, root_size) = super::spot(100.0, 0, 0, 0.0);
    let (_, nested_size) = super::spot(100.0, 1, 0, 0.0);
    assert!(nested_size < root_size);
}
