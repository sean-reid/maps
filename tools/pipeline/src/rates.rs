//! Rules that turn a WDI rate into a total by multiplying with a denominator WDI also carries.

use crate::wdi::split_name;

/// Parenthesised suffixes that qualify the source of an estimate and do not name a unit.
const QUALIFIERS: [&str; 3] = [
    "modeled ILO estimate",
    "national estimate",
    "modeled estimate",
];

/// Title fragments that name a part of the population.
const SUBGROUPS: [&str; 12] = [
    ", female",
    ", male",
    "quintile",
    "poorest",
    "richest",
    "primary education",
    "secondary education",
    "tertiary education",
    "older adults",
    "young adults",
    "in labor force",
    "out of labor force",
];

/// Definitions that describe a rate no multiplication can turn back into a count.
const STANDARDIZED: &str = "age-standardi";

/// Title wording that names a rate; the derived series is a count.
const TITLE_WORDS: [(&str, &str); 6] = [
    (" headcount ratio", " headcount"),
    ("Labor force participation rate", "Labor force"),
    ("Birth rate, crude", "Births"),
    ("Death rate, crude", "Deaths"),
    ("Mortality rate attributed to", "Deaths attributed to"),
    (" mortality rate", " deaths"),
];

pub struct Rule {
    pub name: &'static str,
    /// Exact unit strings the rule applies to.
    pub units: &'static [&'static str],
    /// Also applies to every money unit that is not a local currency.
    pub money: bool,
    /// The title must contain this text when it is not empty.
    pub title_has: &'static str,
    /// The title must contain this text when it is not empty, and loses it.
    pub title_strip: &'static str,
    /// The unit does not say whose population it is, so a title naming a subgroup is refused.
    pub loose: bool,
    /// Series codes whose values add up to the denominator.
    pub denom: &'static [&'static str],
    /// Multiplies rate times denominator.
    pub scale: f64,
    /// Unit of the product; empty keeps the unit of the rate.
    pub out_unit: &'static str,
    pub note: &'static str,
}

const NONE: Rule = Rule {
    name: "",
    units: &[],
    money: false,
    title_has: "",
    title_strip: "",
    loose: false,
    denom: &[],
    scale: 1.0,
    out_unit: "",
    note: "",
};

const POP: &[&str] = &["SP.POP.TOTL"];
const POP_F: &[&str] = &["SP.POP.TOTL.FE.IN"];
const POP_M: &[&str] = &["SP.POP.TOTL.MA.IN"];
const P1564: &[&str] = &["SP.POP.1564.TO"];
const P1564_F: &[&str] = &["SP.POP.1564.FE.IN"];
const P1564_M: &[&str] = &["SP.POP.1564.MA.IN"];
const P15UP: &[&str] = &["SP.POP.1564.TO", "SP.POP.65UP.TO"];
const P15UP_F: &[&str] = &["SP.POP.1564.FE.IN", "SP.POP.65UP.FE.IN"];
const P15UP_M: &[&str] = &["SP.POP.1564.MA.IN", "SP.POP.65UP.MA.IN"];

#[rustfmt::skip]
pub const RULES: &[Rule] = &[
    Rule { name: "pct-population", loose: true, units: &["% of population"], denom: POP, scale: 0.01, out_unit: "people", note: "share of population times population", ..NONE },
    Rule { name: "pct-total-population", units: &["% of total population"], denom: POP, scale: 0.01, out_unit: "people", note: "share of population times population", ..NONE },
    Rule { name: "pct-population-female", units: &["% of female population"], denom: POP_F, scale: 0.01, out_unit: "people", note: "share of female population times female population", ..NONE },
    Rule { name: "pct-population-male", units: &["% of male population"], denom: POP_M, scale: 0.01, out_unit: "people", note: "share of male population times male population", ..NONE },
    Rule { name: "pct-urban", units: &["% of urban population"], denom: &["SP.URB.TOTL"], scale: 0.01, out_unit: "people", note: "share of urban population times urban population", ..NONE },
    Rule { name: "pct-rural", units: &["% of rural population"], denom: &["SP.RUR.TOTL"], scale: 0.01, out_unit: "people", note: "share of rural population times rural population", ..NONE },
    Rule { name: "pct-ages-15-64", units: &["% of total population ages 15-64"], denom: P1564, scale: 0.01, out_unit: "people", note: "share of population ages 15-64 times that population", ..NONE },
    Rule { name: "pct-ages-15-64-female", units: &["% of female population ages 15-64"], denom: P1564_F, scale: 0.01, out_unit: "people", note: "share of female population ages 15-64 times that population", ..NONE },
    Rule { name: "pct-ages-15-64-male", units: &["% of male population ages 15-64"], denom: P1564_M, scale: 0.01, out_unit: "people", note: "share of male population ages 15-64 times that population", ..NONE },
    Rule { name: "pct-total-ages-15-up", units: &["% of total population ages 15+"], denom: P15UP, scale: 0.01, out_unit: "people", note: "share of population ages 15 and above times that population", ..NONE },
    Rule { name: "pct-ages-15-up", loose: true, units: &["% of population ages 15+"], denom: P15UP, scale: 0.01, out_unit: "people", note: "share of population ages 15 and above times that population", ..NONE },
    Rule { name: "pct-ages-15-up-female", units: &["% of female population ages 15+"], denom: P15UP_F, scale: 0.01, out_unit: "people", note: "share of female population ages 15 and above times that population", ..NONE },
    Rule { name: "pct-ages-15-up-male", units: &["% of male population ages 15+"], denom: P15UP_M, scale: 0.01, out_unit: "people", note: "share of male population ages 15 and above times that population", ..NONE },
    Rule { name: "pct-exposed-population", loose: true, units: &["% of total"], title_has: "population exposed", denom: POP, scale: 0.01, out_unit: "people", note: "share of population times population", ..NONE },
    Rule { name: "pct-labor-force", units: &["% of total labor force"], denom: &["SL.TLF.TOTL.IN"], scale: 0.01, out_unit: "people", note: "share of labor force times labor force", ..NONE },
    Rule { name: "per-100-people", loose: true, units: &["per 100 people"], denom: POP, scale: 0.01, out_unit: "number", note: "rate per 100 people times population divided by 100", ..NONE },
    Rule { name: "per-1000-people", loose: true, units: &["per 1,000 people", "per 1,000 population"], denom: POP, scale: 0.001, out_unit: "number", note: "rate per 1,000 people times population divided by 1,000", ..NONE },
    Rule { name: "per-100000-people", loose: true, units: &["per 100,000 people", "per 100,000 population"], denom: POP, scale: 0.00001, out_unit: "number", note: "rate per 100,000 people times population divided by 100,000", ..NONE },
    Rule { name: "per-million-people", loose: true, units: &["per million people", "per 1 million people"], denom: POP, scale: 0.000001, out_unit: "number", note: "rate per million people times population divided by a million", ..NONE },
    Rule { name: "per-100000-female", units: &["per 100,000 female population", "per 100,000 female"], denom: POP_F, scale: 0.00001, out_unit: "number", note: "rate per 100,000 women times female population divided by 100,000", ..NONE },
    Rule { name: "per-100000-male", units: &["per 100,000 male population", "per 100,000 male"], denom: POP_M, scale: 0.00001, out_unit: "number", note: "rate per 100,000 men times male population divided by 100,000", ..NONE },
    Rule { name: "per-1000-ages-15-64", units: &["new registrations per 1,000 people ages 15-64"], denom: P1564, scale: 0.001, out_unit: "new registrations", note: "rate per 1,000 people ages 15-64 times that population divided by 1,000", ..NONE },
    Rule { name: "pct-gdp", units: &["% of GDP"], denom: &["NY.GDP.MKTP.CD"], scale: 0.01, out_unit: "current US$", note: "share of GDP times GDP", ..NONE },
    Rule { name: "pct-gni", units: &["% of GNI"], denom: &["NY.GNP.MKTP.CD"], scale: 0.01, out_unit: "current US$", note: "share of GNI times GNI", ..NONE },
    Rule { name: "pct-land-area", units: &["% of land area", "% of total land area"], denom: &["AG.LND.TOTL.K2"], scale: 0.01, out_unit: "sq. km", note: "share of land area times land area", ..NONE },
    Rule { name: "pct-agricultural-land", units: &["% of total agricultural land"], denom: &["AG.LND.AGRI.K2"], scale: 0.01, out_unit: "sq. km", note: "share of agricultural land times agricultural land", ..NONE },
    Rule { name: "pct-merchandise-exports", units: &["% of total merchandise exports", "% of merchandise exports"], denom: &["TX.VAL.MRCH.CD.WT"], scale: 0.01, out_unit: "current US$", note: "share of merchandise exports times merchandise exports", ..NONE },
    Rule { name: "pct-merchandise-imports", units: &["% of total merchandise imports", "% of merchandise imports"], denom: &["TM.VAL.MRCH.CD.WT"], scale: 0.01, out_unit: "current US$", note: "share of merchandise imports times merchandise imports", ..NONE },
    Rule { name: "pct-service-exports", units: &["% of service exports, BoP"], denom: &["BX.GSR.NFSV.CD"], scale: 0.01, out_unit: "current US$, BoP", note: "share of service exports times service exports", ..NONE },
    Rule { name: "pct-service-imports", units: &["% of service imports, BoP"], denom: &["BM.GSR.NFSV.CD"], scale: 0.01, out_unit: "current US$, BoP", note: "share of service imports times service imports", ..NONE },
    Rule { name: "pct-commercial-service-exports", units: &["% of commercial service exports"], denom: &["TX.VAL.SERV.CD.WT"], scale: 0.01, out_unit: "current US$", note: "share of commercial service exports times commercial service exports", ..NONE },
    Rule { name: "pct-commercial-service-imports", units: &["% of commercial service imports"], denom: &["TM.VAL.SERV.CD.WT"], scale: 0.01, out_unit: "current US$", note: "share of commercial service imports times commercial service imports", ..NONE },
    Rule { name: "pct-exports-income", units: &["% of exports of goods, services and primary income"], denom: &["BX.GSR.TOTL.CD"], scale: 0.01, out_unit: "current US$, BoP", note: "share of exports of goods, services and primary income times those exports", ..NONE },
    Rule { name: "pct-imports-income", units: &["% of imports of goods, services and primary income"], denom: &["BM.GSR.TOTL.CD"], scale: 0.01, out_unit: "current US$, BoP", note: "share of imports of goods, services and primary income times those imports", ..NONE },
    Rule { name: "pct-external-debt", units: &["% of total external debt"], denom: &["DT.DOD.DECT.CD"], scale: 0.01, out_unit: "current US$", note: "share of total external debt times total external debt", ..NONE },
    Rule { name: "pct-freshwater-withdrawal", units: &["% of total freshwater withdrawal"], denom: &["ER.H2O.FWTL.K3"], scale: 0.01, out_unit: "billion cubic meters", note: "share of freshwater withdrawal times total withdrawal", ..NONE },
    Rule { name: "pct-internal-resources", units: &["% of internal resources"], denom: &["ER.H2O.INTR.K3"], scale: 0.01, out_unit: "billion cubic meters", note: "share of internal freshwater resources times those resources", ..NONE },
    Rule { name: "per-capita-money", money: true, title_strip: " per capita", denom: POP, note: "per capita value times population", ..NONE },
    Rule { name: "per-capita-water", units: &["cubic meters"], title_strip: " per capita", denom: POP, note: "per capita value times population", ..NONE },
    Rule { name: "per-capita-co2e", units: &["t CO2e/capita"], title_strip: " per capita", denom: POP, out_unit: "t CO2e", note: "per capita value times population", ..NONE },
    Rule { name: "per-capita-electricity", units: &["kWh per capita"], denom: POP, out_unit: "kWh", note: "per capita value times population", ..NONE },
    Rule { name: "per-capita-energy", units: &["kg of oil equivalent per capita"], denom: POP, out_unit: "kg of oil equivalent", note: "per capita value times population", ..NONE },
    Rule { name: "per-person-land", units: &["hectares per person"], denom: POP, out_unit: "hectares", note: "hectares per person times population", ..NONE },
    Rule { name: "per-hectare-arable", units: &["kilograms per hectare of arable land"], denom: &["AG.LND.ARBL.HA"], out_unit: "kg", note: "kilograms per hectare times arable land", ..NONE },
];

/// A rule applied to one indicator, with the title and unit it produces.
pub struct Plan {
    pub rule: &'static Rule,
    pub title: String,
    pub unit: String,
}

fn is_money(unit: &str) -> bool {
    let rest = ["current ", "constant "]
        .iter()
        .find_map(|k| unit.strip_prefix(k));
    rest.is_some_and(|r| {
        let r = r.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ');
        r == "US$" || r == "international $"
    })
}

fn matches(rule: &Rule, title: &str, unit: &str) -> bool {
    let subgroup = rule.loose && SUBGROUPS.iter().any(|s| title.contains(s));
    (rule.units.contains(&unit) || (rule.money && is_money(unit)))
        && title.contains(rule.title_has)
        && title.contains(rule.title_strip)
        && !subgroup
}

fn retitle(rule: &Rule, base: &str, qualifier: Option<&str>) -> String {
    let mut title = base.replacen(rule.title_strip, "", 1);
    for (from, to) in TITLE_WORDS {
        title = title.replacen(from, to, 1);
    }
    match qualifier {
        Some(q) => format!("{title} ({q})"),
        None => title,
    }
}

/// Finds the rule for an indicator name and definition and the title and unit its product gets.
pub fn plan(name: &str, definition: &str) -> Option<Plan> {
    if definition.to_lowercase().contains(STANDARDIZED) {
        return None;
    }
    let (mut base, mut unit) = split_name(name);
    let mut qualifier = None;
    if QUALIFIERS.contains(&unit.as_str()) {
        let (inner, inner_unit) = split_name(&base);
        qualifier = Some(unit);
        (base, unit) = (inner, inner_unit);
    }
    let rule = RULES.iter().find(|r| matches(r, &base, &unit))?;
    let out = if rule.out_unit.is_empty() {
        &unit
    } else {
        rule.out_unit
    };
    Some(Plan {
        rule,
        title: retitle(rule, &base, qualifier.as_deref()),
        unit: out.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::plan;

    fn title_unit(name: &str) -> Option<(String, String)> {
        plan(name, "").map(|p| (p.title, p.unit))
    }

    fn some(title: &str, unit: &str) -> Option<(String, String)> {
        Some((title.to_string(), unit.to_string()))
    }

    #[test]
    fn shares_and_per_n_units_become_counts() {
        assert_eq!(
            title_unit("Internet users (% of population)"),
            some("Internet users", "people")
        );
        assert_eq!(
            title_unit("Beds (per 1,000 people)"),
            some("Beds", "number")
        );
        assert_eq!(
            title_unit("Protected (% of total land area)"),
            some("Protected", "sq. km")
        );
        assert_eq!(
            title_unit("Suicide mortality rate, male (per 100,000 male population)"),
            some("Suicide deaths, male", "number")
        );
    }

    #[test]
    fn qualifier_moves_after_the_title() {
        assert_eq!(
            title_unit("Unemployment, total (% of total labor force) (modeled ILO estimate)"),
            some("Unemployment, total (modeled ILO estimate)", "people")
        );
        assert_eq!(
            title_unit("Labor force participation rate, total (modeled ILO estimate)"),
            None
        );
    }

    #[test]
    fn per_capita_money_keeps_its_unit_and_loses_the_words() {
        assert_eq!(
            title_unit("GDP per capita, PPP (constant 2021 international $)"),
            some("GDP, PPP", "constant 2021 international $")
        );
        assert_eq!(title_unit("GDP per capita (constant LCU)"), None);
        assert_eq!(title_unit("GDP per capita growth (annual %)"), None);
        assert_eq!(
            title_unit("Spending per student (% of GDP per capita)"),
            None
        );
    }

    #[test]
    fn indexes_prices_scores_and_growth_have_no_rule() {
        for name in [
            "Import volume index (2015 = 100)",
            "Official exchange rate (LCU per US$, period average)",
            "Lending interest rate (%)",
            "Voice and accountability: Estimate (approx. -2.5 to +2.5)",
            "Life expectancy at birth, total (years)",
            "Broad money (annual % growth)",
            "Mortality rate, under-5 (per 1,000 live births)",
            "Age dependency ratio (% of working-age population)",
            "Fertility (births per woman)",
        ] {
            assert_eq!(title_unit(name), None, "{name}");
        }
    }

    #[test]
    fn subgroup_titles_need_an_explicit_group_in_the_unit() {
        assert_eq!(
            title_unit("Coverage, poorest quintile (% of population)"),
            None
        );
        assert_eq!(
            title_unit("Account, female (% of population ages 15+)"),
            None
        );
        assert!(title_unit("Internet users, female (% of female population)").is_some());
        assert!(title_unit("Labor force, female (% of total labor force)").is_some());
    }

    #[test]
    fn age_standardized_definitions_are_refused() {
        assert!(plan(
            "Deaths (per 100,000 population)",
            "Rates are age-standardized."
        )
        .is_none());
        assert!(plan("Deaths (per 100,000 population)", "Crude rate.").is_some());
    }
}
