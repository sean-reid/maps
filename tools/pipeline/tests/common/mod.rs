use std::fs;
use std::path::{Path, PathBuf};

pub fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pipeline-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// Decodes a series file by hand, independent of the encoder.
pub fn decode(bytes: &[u8]) -> (Vec<String>, u16, u16, Vec<f32>) {
    assert_eq!(&bytes[0..4], b"MSR1");
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let (n, first, years) = (u16_at(4) as usize, u16_at(6), u16_at(8));
    assert_eq!(u16_at(10), 0);
    let codes = (0..n)
        .map(|i| String::from_utf8(bytes[12 + 3 * i..15 + 3 * i].to_vec()).unwrap())
        .collect();
    let data = (12 + 3 * n).div_ceil(4) * 4;
    assert_eq!(bytes.len(), data + 4 * n * years as usize);
    let values = bytes[data..]
        .chunks(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    (codes, first, years, values)
}

pub fn read_series(out: &Path, slug: &str) -> (Vec<String>, u16, u16, Vec<f32>) {
    decode(&fs::read(out.join(format!("series/{slug}.bin"))).unwrap())
}
