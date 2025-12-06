pub mod state;

pub fn incrememnt(x: usize, len: usize, wrap: bool) -> usize {
    if x == len - 1 {
        if wrap { 0 } else { len - 1 }
    } else {
        x + 1
    }
}

pub fn decrement(x: usize, len: usize, wrap: bool) -> usize {
    if x == 0 {
        if wrap { len - 1 } else { 0 }
    } else {
        x - 1
    }
}
