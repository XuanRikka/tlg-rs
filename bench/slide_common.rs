use std::path::Path;

pub const BENCH_SIZES: [usize; 3] = [256 * 1024, 1024 * 1024, 4 * 1024 * 1024];

pub fn prepare_input(size: usize) -> Vec<u8> {
    let path = Path::new("bench/data/slide_raw.bin");
    let data = std::fs::read(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });

    if data.len() < size {
        panic!(
            "{} is too short: expected at least {} bytes, got {} bytes",
            path.display(),
            size,
            data.len()
        );
    }

    data[..size].to_vec()
}
