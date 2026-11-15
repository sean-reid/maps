//! Which FAOSTAT elements are summable totals, decided from the element name and unit alone.

/// Units that add up across countries, with the factor that takes them to the unit shown.
const UNITS: [(&str, f64, &str); 11] = [
    ("t", 1.0, "t"),
    ("million t", 1e6, "t"),
    ("kt", 1.0, "kt"),
    ("ha", 1.0, "ha"),
    ("1000 ha", 1e3, "ha"),
    ("An", 1.0, "head"),
    ("1000 An", 1e3, "head"),
    ("No", 1.0, "number"),
    ("1000 No", 1e3, "number"),
    ("m3", 1.0, "m3"),
    ("1000 USD", 1e3, "US$"),
];
/// Words that mark an element as a ratio even when its unit looks like a total.
const RATIO_WORDS: [&str; 6] = ["yield", "per", "share", "index", "growth", "rate"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measure {
    pub factor: f64,
    pub unit: &'static str,
}

pub fn measure(element: &str, unit: &str) -> Option<Measure> {
    let lower = element.to_lowercase();
    if lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| RATIO_WORDS.contains(&w))
    {
        return None;
    }
    UNITS
        .iter()
        .find(|(u, _, _)| *u == unit.trim())
        .map(|&(_, factor, unit)| Measure { factor, unit })
}

fn slug_part(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    let dashed = lower.replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    dashed.trim_matches('-').to_string()
}

/// The first unit a pair carries keeps the plain slug; later ones add their unit.
pub fn slug(
    domain: &str,
    item_code: &str,
    element_code: &str,
    unit_at: usize,
    unit: &str,
) -> String {
    let base = format!(
        "fao-{}-{}-{}",
        slug_part(domain),
        slug_part(item_code),
        slug_part(element_code)
    );
    if unit_at == 0 {
        base
    } else {
        format!("{base}-{}", slug_part(unit))
    }
}

/// Drops a trailing note that only gives a year range or says "production" or "export/import".
fn clean_item(item: &str) -> &str {
    let item = item.trim();
    let Some(body) = item.strip_suffix(')') else {
        return item;
    };
    let Some(open) = body.rfind('(') else {
        return item;
    };
    let note = |part: &str| {
        let part = part.trim();
        part == "export/import" || part == "production" || is_range(part)
    };
    if body[open + 1..].split(',').all(note) {
        body[..open].trim_end()
    } else {
        item
    }
}

fn is_range(part: &str) -> bool {
    part.split_once('-').is_some_and(|(a, b)| {
        [a, b]
            .iter()
            .all(|y| y.len() == 4 && y.bytes().all(|c| c.is_ascii_digit()))
    })
}

/// Lowercases plain words and leaves acronyms and formulas such as CO2eq and N2O alone.
fn lower_element(element: &str) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        let mut chars = word.chars();
        let plain =
            word.chars().count() > 1 && chars.next().is_some() && chars.all(|c| c.is_lowercase());
        out.push_str(&if plain {
            word.to_lowercase()
        } else {
            word.clone()
        });
        word.clear();
    };
    for c in element.trim().chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

pub fn title(item: &str, element: &str) -> String {
    format!("{}, {}", clean_item(item), lower_element(element))
}

/// The title with FAO's item wording untouched, for when the cleaned one is shared.
pub fn full_title(item: &str, element: &str) -> String {
    format!("{}, {}", item.trim(), lower_element(element))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_scale_to_their_base_unit() {
        let m = measure("Area", "1000 ha").unwrap();
        assert_eq!((m.factor, m.unit), (1000.0, "ha"));
        let m = measure("Stocks", "1000 An").unwrap();
        assert_eq!((m.factor, m.unit), (1000.0, "head"));
        assert_eq!(measure("Production", "t").unwrap().factor, 1.0);
        assert_eq!(measure("Import value", "1000 USD").unwrap().unit, "US$");
    }

    #[test]
    fn ratios_are_refused() {
        for (element, unit) in [
            ("Yield", "kg/ha"),
            ("Yield", "t"),
            ("Use per area of cropland", "t"),
            ("Share in Land area", "%"),
            ("Area per capita", "ha/cap"),
            ("Production", "kg/ha"),
            ("Index", "No"),
        ] {
            assert!(measure(element, unit).is_none(), "{element} {unit}");
        }
    }

    #[test]
    fn slug_follows_domain_item_element() {
        assert_eq!(slug("QCL", "221", "5312", 0, "ha"), "fao-qcl-221-5312");
        assert_eq!(slug("GT", "5064", "723113", 0, "kt"), "fao-gt-5064-723113");
        assert_eq!(
            slug("QCL", "1062", "5510", 1, "number"),
            "fao-qcl-1062-5510-number"
        );
        assert_eq!(slug("FO", "1", "5622", 2, "US$"), "fao-fo-1-5622-us");
    }

    #[test]
    fn titles_drop_scope_notes_and_keep_meaningful_parentheses() {
        assert_eq!(
            title("Sawdust (export/import)", "Import quantity"),
            "Sawdust, import quantity"
        );
        assert_eq!(
            title("Mechanical wood pulp (1961-2016)", "Production"),
            "Mechanical wood pulp, production"
        );
        assert_eq!(
            title(
                "Wood fuel, all species (export/import, 1961-2016)",
                "Export value"
            ),
            "Wood fuel, all species, export value"
        );
        assert_eq!(
            title("Maize (corn)", "Producing Animals/Slaughtered"),
            "Maize (corn), producing animals/slaughtered"
        );
    }

    #[test]
    fn element_acronyms_keep_their_case() {
        assert_eq!(
            title("Rice", "Emissions (CO2eq) from F-gases (AR5)"),
            "Rice, emissions (CO2eq) from F-gases (AR5)"
        );
        assert_eq!(
            title("Rice", "Direct emissions (N2O)"),
            "Rice, direct emissions (N2O)"
        );
    }
}
