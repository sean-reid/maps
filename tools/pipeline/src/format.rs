//! Series file v1: little-endian, "MSR1", header, entity codes, f32 rows.

use crate::Result;

pub const MAGIC: &[u8; 4] = b"MSR1";

pub fn is_code(code: &str) -> bool {
    code.len() == 3 && code.is_ascii()
}

#[derive(Debug)]
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

pub fn encode(series: &Series) -> Result<Vec<u8>> {
    let n = u16::try_from(series.codes.len()).map_err(|_| "more than 65535 entities")?;
    let n = n as usize;
    if series.first_year.checked_add(series.years).is_none() {
        return Err("year range overflows u16".into());
    }
    if series.values.len() != n * series.years as usize {
        return Err("values do not match entities and years".into());
    }
    let mut out = Vec::with_capacity(12 + n * 3 + 3 + series.values.len() * 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(n as u16).to_le_bytes());
    out.extend_from_slice(&series.first_year.to_le_bytes());
    out.extend_from_slice(&series.years.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    for code in &series.codes {
        if !is_code(code) {
            return Err(format!("bad entity code {code:?}").into());
        }
        out.extend_from_slice(code.as_bytes());
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    for v in &series.values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    Ok(out)
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
        assert_eq!(encode(&sample()).unwrap(), expected);
    }

    #[test]
    fn header_and_padding_layout() {
        let bytes = encode(&sample()).unwrap();
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
        assert_eq!(encode(&s).unwrap().len(), 12 + 12 + 16);
    }

    #[test]
    fn rejects_bad_codes() {
        for code in ["AB", "ABCD", "ÄBC"] {
            let s = Series {
                codes: vec![code.into()],
                first_year: 2000,
                years: 1,
                values: vec![1.0],
            };
            assert!(encode(&s).is_err(), "{code}");
        }
    }

    #[test]
    fn rejects_year_overflow() {
        let s = Series {
            codes: vec!["AAA".into()],
            first_year: 65535,
            years: 2,
            values: vec![1.0; 2],
        };
        assert!(encode(&s).is_err());
    }

    #[test]
    fn rejects_too_many_entities() {
        let s = Series {
            codes: vec!["AAA".into(); 65536],
            first_year: 2000,
            years: 1,
            values: vec![1.0; 65536],
        };
        assert!(encode(&s).is_err());
    }

    #[test]
    fn rejects_too_many_years() {
        let s = Series {
            codes: vec!["AAA".into()],
            first_year: 0,
            years: 65535,
            values: vec![1.0; 65535],
        };
        assert!(encode(&s).is_ok());
        let s = Series {
            years: u16::MAX,
            first_year: 1,
            ..s
        };
        assert!(encode(&s).is_err());
    }
}
