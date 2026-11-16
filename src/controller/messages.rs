use std::sync::Arc;
use std::time::Instant;

use crate::model::action::action;
use crate::model::status::system_snapshot;

#[derive(Clone, Copy)]
pub enum nav {
    back,
    down,
    left,
    menu,
    right,
    select,
    up,
}

#[derive(Clone)]
pub enum message {
    done(Option<String>),
    exited(usize),
    frame(Instant),
    idle,
    mouse,
    launch(usize),
    nav(nav),
    pick { category_index: usize, selected_rows: Vec<usize> },
    run(String, action),
    snapshot(Arc<system_snapshot>),
    tick,
}
