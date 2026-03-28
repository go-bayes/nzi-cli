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

pub const COUNTRY_REFERENCES: &[CountryReference] = &[
    CountryReference {
        code: "NZL",
        name: "New Zealand",
        aliases: &["nz", "aotearoa"],
        lat: -41.0,
        lon: 174.0,
    },
    CountryReference {
        code: "AUS",
        name: "Australia",
        aliases: &[],
        lat: -25.0,
        lon: 133.0,
    },
    CountryReference {
        code: "USA",
        name: "United States",
        aliases: &["usa", "us", "united states of america", "america"],
        lat: 39.5,
        lon: -98.35,
    },
    CountryReference {
        code: "CAN",
        name: "Canada",
        aliases: &[],
        lat: 56.1,
        lon: -106.3,
    },
    CountryReference {
        code: "MEX",
        name: "Mexico",
        aliases: &[],
        lat: 23.6,
        lon: -102.6,
    },
    CountryReference {
        code: "BRA",
        name: "Brazil",
        aliases: &[],
        lat: -10.8,
        lon: -52.9,
    },
    CountryReference {
        code: "ARG",
        name: "Argentina",
        aliases: &[],
        lat: -38.4,
        lon: -63.6,
    },
    CountryReference {
        code: "CHL",
        name: "Chile",
        aliases: &[],
        lat: -35.7,
        lon: -71.5,
    },
    CountryReference {
        code: "PER",
        name: "Peru",
        aliases: &[],
        lat: -9.2,
        lon: -75.0,
    },
    CountryReference {
        code: "COL",
        name: "Colombia",
        aliases: &[],
        lat: 4.6,
        lon: -74.1,
    },
    CountryReference {
        code: "GBR",
        name: "United Kingdom",
        aliases: &["uk", "great britain", "britain"],
        lat: 54.0,
        lon: -2.0,
    },
    CountryReference {
        code: "IRL",
        name: "Ireland",
        aliases: &[],
        lat: 53.1,
        lon: -8.0,
    },
    CountryReference {
        code: "FRA",
        name: "France",
        aliases: &[],
        lat: 46.2,
        lon: 2.2,
    },
    CountryReference {
        code: "DEU",
        name: "Germany",
        aliases: &[],
        lat: 51.1657,
        lon: 10.4515,
    },
    CountryReference {
        code: "NLD",
        name: "Netherlands",
        aliases: &[],
        lat: 52.1,
        lon: 5.3,
    },
    CountryReference {
        code: "BEL",
        name: "Belgium",
        aliases: &[],
        lat: 50.5,
        lon: 4.5,
    },
    CountryReference {
        code: "CHE",
        name: "Switzerland",
        aliases: &[],
        lat: 46.8,
        lon: 8.2,
    },
    CountryReference {
        code: "AUT",
        name: "Austria",
        aliases: &[],
        lat: 47.5,
        lon: 14.5,
    },
    CountryReference {
        code: "ITA",
        name: "Italy",
        aliases: &[],
        lat: 41.9,
        lon: 12.6,
    },
    CountryReference {
        code: "ESP",
        name: "Spain",
        aliases: &[],
        lat: 40.4,
        lon: -3.7,
    },
    CountryReference {
        code: "PRT",
        name: "Portugal",
        aliases: &[],
        lat: 39.7,
        lon: -8.0,
    },
    CountryReference {
        code: "GRC",
        name: "Greece",
        aliases: &[],
        lat: 39.1,
        lon: 22.9,
    },
    CountryReference {
        code: "POL",
        name: "Poland",
        aliases: &[],
        lat: 52.0,
        lon: 19.1,
    },
    CountryReference {
        code: "CZE",
        name: "Czechia",
        aliases: &["czech", "czech republic"],
        lat: 49.8,
        lon: 15.5,
    },
    CountryReference {
        code: "SWE",
        name: "Sweden",
        aliases: &[],
        lat: 62.0,
        lon: 15.0,
    },
    CountryReference {
        code: "NOR",
        name: "Norway",
        aliases: &[],
        lat: 64.5,
        lon: 11.5,
    },
    CountryReference {
        code: "FIN",
        name: "Finland",
        aliases: &[],
        lat: 64.0,
        lon: 26.0,
    },
    CountryReference {
        code: "DNK",
        name: "Denmark",
        aliases: &[],
        lat: 56.0,
        lon: 10.0,
    },
    CountryReference {
        code: "RUS",
        name: "Russia",
        aliases: &[],
        lat: 61.5,
        lon: 105.0,
    },
    CountryReference {
        code: "UKR",
        name: "Ukraine",
        aliases: &[],
        lat: 49.0,
        lon: 32.0,
    },
    CountryReference {
        code: "TUR",
        name: "Turkey",
        aliases: &[],
        lat: 39.0,
        lon: 35.0,
    },
    CountryReference {
        code: "EGY",
        name: "Egypt",
        aliases: &[],
        lat: 26.8,
        lon: 30.8,
    },
    CountryReference {
        code: "NGA",
        name: "Nigeria",
        aliases: &[],
        lat: 9.1,
        lon: 8.7,
    },
    CountryReference {
        code: "KEN",
        name: "Kenya",
        aliases: &[],
        lat: 0.2,
        lon: 37.9,
    },
    CountryReference {
        code: "ETH",
        name: "Ethiopia",
        aliases: &[],
        lat: 9.1,
        lon: 40.5,
    },
    CountryReference {
        code: "ZAF",
        name: "South Africa",
        aliases: &[],
        lat: -29.0,
        lon: 24.0,
    },
    CountryReference {
        code: "SAU",
        name: "Saudi Arabia",
        aliases: &["saudi"],
        lat: 23.9,
        lon: 45.1,
    },
    CountryReference {
        code: "ARE",
        name: "United Arab Emirates",
        aliases: &["uae"],
        lat: 23.4,
        lon: 53.8,
    },
    CountryReference {
        code: "QAT",
        name: "Qatar",
        aliases: &[],
        lat: 25.3,
        lon: 51.2,
    },
    CountryReference {
        code: "IND",
        name: "India",
        aliases: &[],
        lat: 21.0,
        lon: 78.0,
    },
    CountryReference {
        code: "BGD",
        name: "Bangladesh",
        aliases: &[],
        lat: 23.685,
        lon: 90.3563,
    },
    CountryReference {
        code: "CHN",
        name: "China",
        aliases: &[],
        lat: 35.9,
        lon: 104.2,
    },
    CountryReference {
        code: "JPN",
        name: "Japan",
        aliases: &[],
        lat: 36.2,
        lon: 138.2,
    },
    CountryReference {
        code: "KOR",
        name: "South Korea",
        aliases: &["korea"],
        lat: 36.5,
        lon: 127.9,
    },
    CountryReference {
        code: "SGP",
        name: "Singapore",
        aliases: &[],
        lat: 1.3521,
        lon: 103.8198,
    },
    CountryReference {
        code: "MYS",
        name: "Malaysia",
        aliases: &[],
        lat: 4.2105,
        lon: 101.9758,
    },
    CountryReference {
        code: "IDN",
        name: "Indonesia",
        aliases: &[],
        lat: -2.5,
        lon: 117.2,
    },
];

pub const CURRENCY_REFERENCES: &[CurrencyReference] = &[
    CurrencyReference {
        code: "NZD",
        name: "New Zealand dollar",
        aliases: &["nzd", "new zealand dollar"],
        focal_country_code: "NZL",
    },
    CurrencyReference {
        code: "AUD",
        name: "Australian dollar",
        aliases: &["aud", "australian dollar"],
        focal_country_code: "AUS",
    },
    CurrencyReference {
        code: "USD",
        name: "US dollar",
        aliases: &["usd", "us dollar", "dollar"],
        focal_country_code: "USA",
    },
    CurrencyReference {
        code: "CAD",
        name: "Canadian dollar",
        aliases: &["cad", "canadian dollar"],
        focal_country_code: "CAN",
    },
    CurrencyReference {
        code: "MXN",
        name: "Mexican peso",
        aliases: &["mxn", "peso"],
        focal_country_code: "MEX",
    },
    CurrencyReference {
        code: "BRL",
        name: "Brazilian real",
        aliases: &["brl", "real"],
        focal_country_code: "BRA",
    },
    CurrencyReference {
        code: "ARS",
        name: "Argentine peso",
        aliases: &["ars"],
        focal_country_code: "ARG",
    },
    CurrencyReference {
        code: "CLP",
        name: "Chilean peso",
        aliases: &["clp"],
        focal_country_code: "CHL",
    },
    CurrencyReference {
        code: "PEN",
        name: "Peruvian sol",
        aliases: &["pen"],
        focal_country_code: "PER",
    },
    CurrencyReference {
        code: "COP",
        name: "Colombian peso",
        aliases: &["cop"],
        focal_country_code: "COL",
    },
    CurrencyReference {
        code: "GBP",
        name: "Pound sterling",
        aliases: &["gbp", "pound", "sterling"],
        focal_country_code: "GBR",
    },
    CurrencyReference {
        code: "EUR",
        name: "Euro",
        aliases: &["eur", "euro"],
        focal_country_code: "DEU",
    },
    CurrencyReference {
        code: "CHF",
        name: "Swiss franc",
        aliases: &["chf", "franc"],
        focal_country_code: "CHE",
    },
    CurrencyReference {
        code: "SEK",
        name: "Swedish krona",
        aliases: &["sek"],
        focal_country_code: "SWE",
    },
    CurrencyReference {
        code: "NOK",
        name: "Norwegian krone",
        aliases: &["nok"],
        focal_country_code: "NOR",
    },
    CurrencyReference {
        code: "DKK",
        name: "Danish krone",
        aliases: &["dkk"],
        focal_country_code: "DNK",
    },
    CurrencyReference {
        code: "RUB",
        name: "Russian ruble",
        aliases: &["rub", "ruble"],
        focal_country_code: "RUS",
    },
    CurrencyReference {
        code: "UAH",
        name: "Ukrainian hryvnia",
        aliases: &["uah"],
        focal_country_code: "UKR",
    },
    CurrencyReference {
        code: "TRY",
        name: "Turkish lira",
        aliases: &["try", "lira"],
        focal_country_code: "TUR",
    },
    CurrencyReference {
        code: "EGP",
        name: "Egyptian pound",
        aliases: &["egp"],
        focal_country_code: "EGY",
    },
    CurrencyReference {
        code: "NGN",
        name: "Nigerian naira",
        aliases: &["ngn"],
        focal_country_code: "NGA",
    },
    CurrencyReference {
        code: "KES",
        name: "Kenyan shilling",
        aliases: &["kes"],
        focal_country_code: "KEN",
    },
    CurrencyReference {
        code: "ETB",
        name: "Ethiopian birr",
        aliases: &["etb", "birr"],
        focal_country_code: "ETH",
    },
    CurrencyReference {
        code: "ZAR",
        name: "South African rand",
        aliases: &["zar", "rand"],
        focal_country_code: "ZAF",
    },
    CurrencyReference {
        code: "SAR",
        name: "Saudi riyal",
        aliases: &["sar", "riyal"],
        focal_country_code: "SAU",
    },
    CurrencyReference {
        code: "AED",
        name: "UAE dirham",
        aliases: &["aed", "dirham"],
        focal_country_code: "ARE",
    },
    CurrencyReference {
        code: "QAR",
        name: "Qatari riyal",
        aliases: &["qar"],
        focal_country_code: "QAT",
    },
    CurrencyReference {
        code: "INR",
        name: "Indian rupee",
        aliases: &["inr", "rupee"],
        focal_country_code: "IND",
    },
    CurrencyReference {
        code: "BDT",
        name: "Bangladeshi taka",
        aliases: &["bdt", "taka"],
        focal_country_code: "BGD",
    },
    CurrencyReference {
        code: "CNY",
        name: "Chinese yuan",
        aliases: &["cny", "yuan", "renminbi"],
        focal_country_code: "CHN",
    },
    CurrencyReference {
        code: "JPY",
        name: "Japanese yen",
        aliases: &["jpy", "yen"],
        focal_country_code: "JPN",
    },
    CurrencyReference {
        code: "KRW",
        name: "South Korean won",
        aliases: &["krw", "won"],
        focal_country_code: "KOR",
    },
    CurrencyReference {
        code: "SGD",
        name: "Singapore dollar",
        aliases: &["sgd"],
        focal_country_code: "SGP",
    },
    CurrencyReference {
        code: "MYR",
        name: "Malaysian ringgit",
        aliases: &["myr", "ringgit"],
        focal_country_code: "MYS",
    },
    CurrencyReference {
        code: "IDR",
        name: "Indonesian rupiah",
        aliases: &["idr", "rupiah"],
        focal_country_code: "IDN",
    },
];

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
}
