//! Series file v1: little-endian, "MSR1", header, entity codes, f32 rows.

pub const MAGIC: &[u8; 4] = b"MSR1";

/// One indicator: `values` is row-major by entity, `codes.len() * years` long.
pub struct Series {
    pub codes: Vec<String>,
    pub first_year: u16,
    pub years: u16,
    pub values: Vec<f32>,
}

impl Series {
    pub fn last_year(&self) -> u16 {
        self.first_year + self.years - 1
    }
}

pub fn encode(series: &Series) -> Vec<u8> {
    let n = series.codes.len();
    assert_eq!(series.values.len(), n * series.years as usize);
    let mut out = Vec::with_capacity(12 + n * 3 + 3 + series.values.len() * 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(n as u16).to_le_bytes());
    out.extend_from_slice(&series.first_year.to_le_bytes());
    out.extend_from_slice(&series.years.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    for code in &series.codes {
        assert!(code.len() == 3 && code.is_ascii(), "bad entity code {code}");
        out.extend_from_slice(code.as_bytes());
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    for v in &series.values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Series {
        Series {
            codes: vec!["AUS".into(), "BRA".into(), "CHN".into()],
            first_year: 1999,
            years: 3,
            values: vec![
                1.0,
                2.5,
                f32::NAN,
                f32::NAN,
                4.0,
                5.0,
                100.0,
                f32::NAN,
                -0.0,
            ],
        }
    }

    #[test]
    fn matches_committed_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/series-v1.bin"
        );
        let expected = std::fs::read(path).unwrap();
        assert_eq!(encode(&sample()), expected);
    }

    #[test]
    fn header_and_padding_layout() {
        let bytes = encode(&sample());
        assert_eq!(&bytes[0..4], b"MSR1");
        assert_eq!(&bytes[4..12], &[3, 0, 0xCF, 0x07, 3, 0, 0, 0]);
        assert_eq!(&bytes[12..21], b"AUSBRACHN");
        assert_eq!(&bytes[21..24], &[0, 0, 0]);
        assert_eq!(bytes.len(), 24 + 9 * 4);
        assert_eq!(&bytes[24..28], &1.0f32.to_le_bytes());
    }

    #[test]
    fn no_padding_when_aligned() {
        let s = Series {
            codes: vec!["AAA".into(), "BBB".into(), "CCC".into(), "DDD".into()],
            first_year: 2000,
            years: 1,
            values: vec![1.0; 4],
        };
        assert_eq!(encode(&s).len(), 12 + 12 + 16);
    }
}
