//! canonical country and currency references
//! used for config validation and map/currency lookups

#[derive(Debug, Clone, Copy)]
pub struct CountryReference {
    pub code: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct CurrencyReference {
    pub code: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub focal_country_code: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct RepresentativeCityReference {
    pub country_code: &'static str,
    pub city_code: &'static str,
    pub city_name: &'static str,
    pub country_name: &'static str,
    pub timezone: &'static str,
    pub currency_code: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/reference_data.rs"));

pub fn normalise_country_code(value: &str) -> String {
    value.trim().to_uppercase()
}

pub fn normalise_currency_code(value: &str) -> String {
    value.trim().to_uppercase()
}

pub fn is_valid_country_code(value: &str) -> bool {
    let value = normalise_country_code(value);
    value.len() == 3 && value.chars().all(|ch| ch.is_ascii_alphabetic())
}

pub fn is_valid_currency_code(value: &str) -> bool {
    let value = normalise_currency_code(value);
    value.len() == 3 && value.chars().all(|ch| ch.is_ascii_alphabetic())
}

pub fn country_by_code(code: &str) -> Option<&'static CountryReference> {
    let code = normalise_country_code(code);
    COUNTRY_REFERENCES
        .iter()
        .find(|country| country.code == code.as_str())
}

pub fn lookup_country(query: &str) -> Option<&'static CountryReference> {
    let query = query.trim().to_lowercase();
    COUNTRY_REFERENCES.iter().find(|country| {
        country.name.eq_ignore_ascii_case(query.as_str())
            || country.code.eq_ignore_ascii_case(query.as_str())
            || country
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(query.as_str()))
    })
}

pub fn currency_by_code(code: &str) -> Option<&'static CurrencyReference> {
    let code = normalise_currency_code(code);
    CURRENCY_REFERENCES
        .iter()
        .find(|currency| currency.code == code.as_str())
}

pub fn lookup_currency(query: &str) -> Option<&'static CurrencyReference> {
    let query = query.trim().to_lowercase();
    CURRENCY_REFERENCES.iter().find(|currency| {
        currency.name.eq_ignore_ascii_case(query.as_str())
            || currency.code.eq_ignore_ascii_case(query.as_str())
            || currency
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(query.as_str()))
    })
}

pub fn focal_country_code_for_currency(code: &str) -> Option<&'static str> {
    currency_by_code(code).map(|currency| currency.focal_country_code)
}

pub fn canonical_currency_code_for_country(code: &str) -> Option<&'static str> {
    let code = normalise_country_code(code);
    CURRENCY_REFERENCES
        .iter()
        .find(|currency| currency.focal_country_code == code.as_str())
        .map(|currency| currency.code)
}

pub fn representative_city_by_country_code(
    code: &str,
) -> Option<&'static RepresentativeCityReference> {
    let code = normalise_country_code(code);
    REPRESENTATIVE_CITY_REFERENCES
        .iter()
        .find(|city| city.country_code == code.as_str())
}

pub fn representative_city_by_city_code(
    code: &str,
) -> Option<&'static RepresentativeCityReference> {
    let code = code.trim().to_uppercase();
    REPRESENTATIVE_CITY_REFERENCES
        .iter()
        .find(|city| city.city_code == code.as_str())
}

pub fn representative_city_by_currency_code(
    code: &str,
) -> Option<&'static RepresentativeCityReference> {
    let country_code = focal_country_code_for_currency(code)?;
    representative_city_by_country_code(country_code)
}

pub fn search_representative_cities(query: &str) -> Vec<&'static RepresentativeCityReference> {
    let query = query.trim().to_lowercase();
    let mut matches: Vec<_> = REPRESENTATIVE_CITY_REFERENCES
        .iter()
        .filter(|city| matches_representative_city(city, &query))
        .collect();
    matches.sort_by_key(|city| representative_city_match_rank(city, &query));
    matches
}

pub fn search_countries(query: &str) -> Vec<&'static CountryReference> {
    let query = query.trim().to_lowercase();
    let mut matches: Vec<_> = COUNTRY_REFERENCES
        .iter()
        .filter(|country| matches_country(country, &query))
        .collect();
    matches.sort_by_key(|country| country_match_rank(country, &query));
    matches
}

pub fn search_currencies(query: &str) -> Vec<&'static CurrencyReference> {
    let query = query.trim().to_lowercase();
    let mut matches: Vec<_> = CURRENCY_REFERENCES
        .iter()
        .filter(|currency| matches_currency(currency, &query))
        .collect();
    matches.sort_by_key(|currency| currency_match_rank(currency, &query));
    matches
}

fn matches_country(country: &CountryReference, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    country.code.to_lowercase().contains(query)
        || country.name.to_lowercase().contains(query)
        || country
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().contains(query))
}

fn matches_currency(currency: &CurrencyReference, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    currency.code.to_lowercase().contains(query)
        || currency.name.to_lowercase().contains(query)
        || currency
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().contains(query))
}

fn country_match_rank(country: &CountryReference, query: &str) -> u8 {
    if query.is_empty() {
        return 3;
    }
    if country.code.eq_ignore_ascii_case(query) {
        0
    } else if country.name.eq_ignore_ascii_case(query)
        || country
            .aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(query))
    {
        1
    } else {
        2
    }
}

fn currency_match_rank(currency: &CurrencyReference, query: &str) -> u8 {
    if query.is_empty() {
        return 3;
    }
    if currency.code.eq_ignore_ascii_case(query) {
        0
    } else if currency.name.eq_ignore_ascii_case(query)
        || currency
            .aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(query))
    {
        1
    } else {
        2
    }
}

fn matches_representative_city(city: &RepresentativeCityReference, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    city.city_code.to_lowercase().contains(query)
        || city.city_name.to_lowercase().contains(query)
        || city.country_name.to_lowercase().contains(query)
        || country_by_code(city.country_code)
            .map(|country| {
                country
                    .aliases
                    .iter()
                    .any(|alias| alias.to_lowercase().contains(query))
            })
            .unwrap_or(false)
        || city.currency_code.to_lowercase().contains(query)
        || currency_by_code(city.currency_code)
            .map(|currency| {
                currency.name.to_lowercase().contains(query)
                    || currency
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase().contains(query))
            })
            .unwrap_or(false)
}

fn representative_city_match_rank(city: &RepresentativeCityReference, query: &str) -> u8 {
    if query.is_empty() {
        return 5;
    }
    if city.city_code.eq_ignore_ascii_case(query) {
        0
    } else if city.city_name.eq_ignore_ascii_case(query) {
        1
    } else if city.country_name.eq_ignore_ascii_case(query)
        || country_by_code(city.country_code)
            .map(|country| {
                country
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(query))
            })
            .unwrap_or(false)
    {
        2
    } else if city.currency_code.eq_ignore_ascii_case(query)
        || currency_by_code(city.currency_code)
            .map(|currency| {
                currency.name.eq_ignore_ascii_case(query)
                    || currency
                        .aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(query))
            })
            .unwrap_or(false)
    {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_country_aliases() {
        let country = lookup_country("uk").expect("country alias should resolve");
        assert_eq!(country.code, "GBR");
    }

    #[test]
    fn looks_up_currency_aliases() {
        let currency = lookup_currency("yen").expect("currency alias should resolve");
        assert_eq!(currency.code, "JPY");
        assert_eq!(currency.name, "Japanese yen");
    }

    #[test]
    fn searches_country_aliases() {
        let countries = search_countries("brit");
        assert_eq!(countries.first().map(|country| country.code), Some("GBR"));
    }

    #[test]
    fn searches_currency_aliases() {
        let currencies = search_currencies("yen");
        assert_eq!(
            currencies.first().map(|currency| currency.code),
            Some("JPY")
        );
    }

    #[test]
    fn looks_up_representative_city_by_country_code() {
        let city = representative_city_by_country_code("fra")
            .expect("representative city should resolve for france");
        assert_eq!(city.city_code, "PAR");
        assert_eq!(city.city_name, "Paris");
    }

    #[test]
    fn searches_representative_cities_by_country_name() {
        let cities = search_representative_cities("japan");
        assert_eq!(cities.first().map(|city| city.city_code), Some("TYO"));
    }

    #[test]
    fn searches_representative_cities_by_currency_alias() {
        let cities = search_representative_cities("yen");
        assert_eq!(cities.first().map(|city| city.city_code), Some("TYO"));
    }

    #[test]
    fn searches_representative_cities_by_city_name() {
        let cities = search_representative_cities("copenhagen");
        assert_eq!(cities.first().map(|city| city.city_code), Some("CPH"));
    }

    #[test]
    fn looks_up_new_country_and_currency_entries() {
        let country = lookup_country("iran").expect("iran should resolve");
        let currency = lookup_currency("shekel").expect("shekel should resolve");

        assert_eq!(country.code, "IRN");
        assert_eq!(currency.code, "ILS");
    }

    #[test]
    fn every_country_has_a_representative_city() {
        for country in COUNTRY_REFERENCES {
            let city = representative_city_by_country_code(country.code)
                .unwrap_or_else(|| panic!("missing representative city for {}", country.code));
            assert_eq!(city.country_code, country.code);
        }
    }

    #[test]
    fn every_currency_has_a_focal_country_and_representative_city() {
        for currency in CURRENCY_REFERENCES {
            let country = country_by_code(currency.focal_country_code).unwrap_or_else(|| {
                panic!(
                    "missing focal country {} for currency {}",
                    currency.focal_country_code, currency.code
                )
            });
            let city = representative_city_by_country_code(country.code).unwrap_or_else(|| {
                panic!(
                    "missing representative city for focal country {} of currency {}",
                    country.code, currency.code
                )
            });
            assert_eq!(city.country_code, country.code);
        }
    }
}
