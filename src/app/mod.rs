pub mod state;

pub fn increment(x: usize, len: usize, wrap: bool) -> usize {
    if wrap {
        (x + 1) % len
    } else {
        x.saturating_add(1).min(len - 1)
    }
}

pub fn decrement(x: usize, len: usize, wrap: bool) -> usize {
    if wrap {
        (x + len - 1) % len
    } else {
        x.saturating_sub(1)
    }
}
