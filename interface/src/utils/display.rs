#![allow(warnings)]

pub fn format_file_size(size: u64) -> String {
    if size < 1_000 {
        format!("{}o", size)
    } else if size < 1_000_000 {
        format!("{}ko", size / 1_000)
    } else if size < 1_000_000_000 {
        format!("{}Mo", size / 1_000_000)
    } else {
        format!("{}Go", size / 1_000_000_000)
    }
}

