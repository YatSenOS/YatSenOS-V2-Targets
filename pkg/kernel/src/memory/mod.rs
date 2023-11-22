pub mod gdt;

pub fn humanized_size(size: u64) -> (f64, &'static str) {
    let bytes = size as f64;

    // use 1000 to keep the max length of the number is 3 digits
    if bytes < 1000f64 {
        (bytes, "  B")
    } else if (bytes / (1 << 10) as f64) < 1000f64 {
        (bytes / (1 << 10) as f64, "KiB")
    } else if (bytes / (1 << 20) as f64) < 1000f64 {
        (bytes / (1 << 20) as f64, "MiB")
    } else {
        (bytes / (1 << 30) as f64, "GiB")
    }
}
