//! configuration management for nzi-cli
//! handles loading and saving user preferences from ~/.config/nzi-cli/config.toml
//! follows margo-style config: simple toml with manual parsing

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::reference::{
    is_valid_country_code, is_valid_currency_code, lookup_country, normalise_country_code,
    normalise_currency_code,
};

/// city configuration with timezone and currency info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub name: String,
    pub code: String,
    pub country: String,
    pub timezone: String,
    pub currency: String,
}

impl City {
    pub fn wellington() -> Self {
        Self {
            name: "Wellington".to_string(),
            code: "WLG".to_string(),
            country: "New Zealand".to_string(),
            timezone: "Pacific/Auckland".to_string(),
            currency: "NZD".to_string(),
        }
    }

    pub fn boston() -> Self {
        Self {
            name: "Boston".to_string(),
            code: "BOS".to_string(),
            country: "USA".to_string(),
            timezone: "America/New_York".to_string(),
            currency: "USD".to_string(),
        }
    }

    pub fn london() -> Self {
        Self {
            name: "London".to_string(),
            code: "LDN".to_string(),
            country: "United Kingdom".to_string(),
            timezone: "Europe/London".to_string(),
            currency: "GBP".to_string(),
        }
    }

    pub fn sydney() -> Self {
        Self {
            name: "Sydney".to_string(),
            code: "SYD".to_string(),
            country: "Australia".to_string(),
            timezone: "Australia/Sydney".to_string(),
            currency: "AUD".to_string(),
        }
    }

    pub fn tokyo() -> Self {
        Self {
            name: "Tokyo".to_string(),
            code: "TYO".to_string(),
            country: "Japan".to_string(),
            timezone: "Asia/Tokyo".to_string(),
            currency: "JPY".to_string(),
        }
    }

    pub fn los_angeles() -> Self {
        Self {
            name: "Los Angeles".to_string(),
            code: "LAX".to_string(),
            country: "USA".to_string(),
            timezone: "America/Los_Angeles".to_string(),
            currency: "USD".to_string(),
        }
    }

    pub fn singapore() -> Self {
        Self {
            name: "Singapore".to_string(),
            code: "SIN".to_string(),
            country: "Singapore".to_string(),
            timezone: "Asia/Singapore".to_string(),
            currency: "SGD".to_string(),
        }
    }

    pub fn kuala_lumpur() -> Self {
        Self {
            name: "Kuala Lumpur".to_string(),
            code: "KL".to_string(),
            country: "Malaysia".to_string(),
            timezone: "Asia/Kuala_Lumpur".to_string(),
            currency: "MYR".to_string(),
        }
    }

    pub fn paris() -> Self {
        Self {
            name: "Paris".to_string(),
            code: "PAR".to_string(),
            country: "France".to_string(),
            timezone: "Europe/Paris".to_string(),
            currency: "EUR".to_string(),
        }
    }

    pub fn berlin() -> Self {
        Self {
            name: "Berlin".to_string(),
            code: "BER".to_string(),
            country: "Germany".to_string(),
            timezone: "Europe/Berlin".to_string(),
            currency: "EUR".to_string(),
        }
    }

    pub fn austin() -> Self {
        Self {
            name: "Austin".to_string(),
            code: "AUS".to_string(),
            country: "USA".to_string(),
            timezone: "America/Chicago".to_string(),
            currency: "USD".to_string(),
        }
    }

    pub fn rio() -> Self {
        Self {
            name: "RIO".to_string(),
            code: "RIO".to_string(),
            country: "Brazil".to_string(),
            timezone: "America/Sao_Paulo".to_string(),
            currency: "BRL".to_string(),
        }
    }

    pub fn addis_ababa() -> Self {
        Self {
            name: "Addis Ababa".to_string(),
            code: "ADD".to_string(),
            country: "Ethiopia".to_string(),
            timezone: "Africa/Addis_Ababa".to_string(),
            currency: "ETB".to_string(),
        }
    }

    pub fn dhaka() -> Self {
        Self {
            name: "Dhaka".to_string(),
            code: "DAC".to_string(),
            country: "Bangladesh".to_string(),
            timezone: "Asia/Dhaka".to_string(),
            currency: "BDT".to_string(),
        }
    }

    pub fn beijing() -> Self {
        Self {
            name: "Beijing".to_string(),
            code: "BJS".to_string(),
            country: "China".to_string(),
            timezone: "Asia/Shanghai".to_string(),
            currency: "CNY".to_string(),
        }
    }
}

/// display preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub show_seconds: bool,
    pub use_24_hour: bool,
    pub show_animations: bool,
    pub animation_speed_ms: u64,
    /// editor command for /edit (defaults to $EDITOR or nvim)
    #[serde(default)]
    pub editor: Option<String>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_seconds: true,
            use_24_hour: true,
            show_animations: true,
            animation_speed_ms: 100,
            editor: None,
        }
    }
}

impl DisplayConfig {
    /// get the editor command, checking config, $EDITOR, then falling back to nvim
    pub fn get_editor(&self) -> String {
        self.editor
            .clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "nvim".to_string())
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConfig {
    #[serde(default = "default_true")]
    pub sync_with_cities: bool,
    #[serde(default)]
    pub pinned_codes: Vec<String>,
    #[serde(default)]
    pub default_from: Option<String>,
    #[serde(default)]
    pub default_to: Option<String>,
}

impl Default for CurrencyConfig {
    fn default() -> Self {
        Self {
            sync_with_cities: true,
            pinned_codes: Vec::new(),
            default_from: None,
            default_to: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MapMode {
    Route,
    Cities,
    Countries,
    Both,
}

impl Default for MapMode {
    fn default() -> Self {
        Self::Route
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapConfig {
    #[serde(default)]
    pub mode: MapMode,
    #[serde(default)]
    pub focus_city_code: Option<String>,
    #[serde(default)]
    pub focus_country_codes: Vec<String>,
    #[serde(default)]
    pub focal_country_code: Option<String>,
}

/// main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// where the user currently lives in NZ
    pub current_city: City,
    /// the user's home city overseas
    pub home_city: City,
    /// additional cities to track
    pub tracked_cities: Vec<City>,
    /// display preferences
    pub display: DisplayConfig,
    /// optional currency behaviour overrides
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<CurrencyConfig>,
    /// optional map focus overrides
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<MapConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // wellington is home - NZ anchor city
            current_city: City::wellington(),
            // boston as primary world city (for "around the world")
            home_city: City::boston(),
            // track other world cities for world clock
            tracked_cities: vec![
                City::london(),
                City::los_angeles(),
                City::austin(),
                City::paris(),
                City::berlin(),
                City::sydney(),
                City::tokyo(),
                City::singapore(),
                City::kuala_lumpur(),
                City::rio(),
                City::addis_ababa(),
                City::dhaka(),
                City::beijing(),
            ],
            display: DisplayConfig::default(),
            currency: None,
            map: None,
        }
    }
}

impl Config {
    /// path to config directory (~/.config/nzi-cli) - margo style
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("nzi-cli")
    }

    /// get the config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// load configuration from file, or create default if it doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("failed to read config file")?;
            let mut config: Config =
                toml::from_str(&content).context("failed to parse config file")?;
            let mut updated = false;
            updated |= config.normalize_legacy_cities();
            updated |= config.normalize();
            updated |= config.ensure_tracked_city(City::rio());
            updated |= config.ensure_tracked_city(City::addis_ababa());
            updated |= config.ensure_tracked_city(City::kuala_lumpur());
            updated |= config.ensure_tracked_city(City::berlin());
            updated |= config.ensure_tracked_city(City::dhaka());
            updated |= config.ensure_tracked_city(City::beijing());
            config.validate()?;
            if updated {
                config.save()?;
            }
            Ok(config)
        } else {
            // create default config
            let config = Config::default();
            config.validate()?;
            config.save()?;
            Ok(config)
        }
    }

    /// save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        // ensure the config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("failed to create config directory")?;
        }

        let mut config = self.clone();
        config.normalize();
        config.validate()?;

        let content = toml::to_string_pretty(&config).context("failed to serialise config")?;

        fs::write(&config_path, content).context("failed to write config file")?;

        Ok(())
    }

    /// get all cities including current and home
    pub fn all_cities(&self) -> Vec<&City> {
        let mut cities = vec![&self.current_city, &self.home_city];
        cities.extend(self.tracked_cities.iter());
        cities
    }

    /// get all city codes for time conversion cycling
    pub fn all_city_codes(&self) -> Vec<String> {
        self.all_cities().iter().map(|c| c.code.clone()).collect()
    }

    pub fn effective_currency_settings(&self) -> CurrencyConfig {
        self.currency.clone().unwrap_or_default()
    }

    pub fn effective_map_settings(&self) -> MapConfig {
        let mut map = self.map.clone().unwrap_or_default();

        if map.focal_country_code.is_none()
            && let Some(country) = lookup_country(&self.current_city.country)
        {
            map.focal_country_code = Some(country.code.to_string());
        }

        map
    }

    pub fn effective_default_currency_pair(&self) -> (String, String) {
        let settings = self.effective_currency_settings();
        let from = settings
            .default_from
            .clone()
            .unwrap_or_else(|| self.current_city.currency.clone());
        let targets = self.effective_currency_targets(&from, &settings);
        let fallback_to = settings
            .default_to
            .unwrap_or_else(|| self.home_city.currency.clone());
        let to = targets.into_iter().next().unwrap_or(fallback_to);
        (normalise_currency_code(&from), normalise_currency_code(&to))
    }

    pub fn effective_currency_pairs(&self) -> Vec<(String, String)> {
        let settings = self.effective_currency_settings();
        let from = settings
            .default_from
            .clone()
            .unwrap_or_else(|| self.current_city.currency.clone());
        let from = normalise_currency_code(&from);

        let mut pairs: Vec<(String, String)> = self
            .effective_currency_targets(&from, &settings)
            .into_iter()
            .map(|to| (from.clone(), to))
            .collect();

        if pairs.is_empty() {
            let fallback_to = settings
                .default_to
                .unwrap_or_else(|| self.home_city.currency.clone());
            let fallback_to = normalise_currency_code(&fallback_to);
            if fallback_to != from {
                pairs.push((from.clone(), fallback_to));
            }
        }

        if pairs.is_empty() {
            pairs.push((from.clone(), from));
        }

        pairs
    }

    fn ensure_tracked_city(&mut self, city: City) -> bool {
        if self.tracked_cities.iter().any(|c| {
            c.code.eq_ignore_ascii_case(&city.code) || c.name.eq_ignore_ascii_case(&city.name)
        }) {
            return false;
        }
        self.tracked_cities.push(city);
        true
    }

    fn normalize_city_name_and_code_to_boston(city: &mut City) -> bool {
        if city.code.eq_ignore_ascii_case("NYC") || city.name.eq_ignore_ascii_case("New York") {
            if city.code != "BOS" || city.name != "Boston" {
                city.code = "BOS".to_string();
                city.name = "Boston".to_string();
                return true;
            }
        }
        false
    }

    fn dedupe_tracked_cities(&mut self) -> bool {
        let mut seen = HashSet::new();
        let original_len = self.tracked_cities.len();

        self.tracked_cities
            .retain(|city| seen.insert(city.code.to_uppercase()));

        self.tracked_cities.len() != original_len
    }

    fn normalize_legacy_cities(&mut self) -> bool {
        let mut updated = false;

        updated |= Self::normalize_city_name_and_code_to_boston(&mut self.home_city);

        for city in &mut self.tracked_cities {
            updated |= Self::normalize_city_name_and_code_to_boston(city);
        }

        updated |= self.dedupe_tracked_cities();

        updated
    }

    fn normalize(&mut self) -> bool {
        let mut updated = false;

        updated |= Self::normalize_city(&mut self.current_city);
        updated |= Self::normalize_city(&mut self.home_city);
        for city in &mut self.tracked_cities {
            updated |= Self::normalize_city(city);
        }

        if let Some(currency) = &mut self.currency {
            updated |=
                Self::normalize_optional_code(&mut currency.default_from, normalise_currency_code);
            updated |=
                Self::normalize_optional_code(&mut currency.default_to, normalise_currency_code);
            updated |=
                Self::normalize_code_list(&mut currency.pinned_codes, normalise_currency_code);
        }

        if let Some(map) = &mut self.map {
            updated |= Self::normalize_optional_code(&mut map.focus_city_code, |value| {
                value.trim().to_uppercase()
            });
            updated |=
                Self::normalize_optional_code(&mut map.focal_country_code, normalise_country_code);
            updated |=
                Self::normalize_code_list(&mut map.focus_country_codes, normalise_country_code);
        }

        updated
    }

    fn normalize_city(city: &mut City) -> bool {
        let mut updated = false;

        let code = city.code.trim().to_uppercase();
        if city.code != code {
            city.code = code;
            updated = true;
        }

        let currency = normalise_currency_code(&city.currency);
        if city.currency != currency {
            city.currency = currency;
            updated = true;
        }

        let country = city.country.trim().to_string();
        if city.country != country {
            city.country = country;
            updated = true;
        }

        let timezone = city.timezone.trim().to_string();
        if city.timezone != timezone {
            city.timezone = timezone;
            updated = true;
        }

        updated
    }

    fn normalize_optional_code<F>(value: &mut Option<String>, normalise: F) -> bool
    where
        F: Fn(&str) -> String,
    {
        let Some(current) = value.as_ref() else {
            return false;
        };

        let normalized = normalise(current);
        if normalized == *current {
            return false;
        }

        *value = Some(normalized);
        true
    }

    fn normalize_code_list<F>(values: &mut Vec<String>, normalise: F) -> bool
    where
        F: Fn(&str) -> String,
    {
        let original = values.clone();
        let mut seen = HashSet::new();
        values.retain_mut(|value| {
            *value = normalise(value);
            seen.insert(value.clone())
        });
        *values != original
    }

    fn effective_currency_targets(
        &self,
        from_currency: &str,
        settings: &CurrencyConfig,
    ) -> Vec<String> {
        let from_currency = normalise_currency_code(from_currency);
        let mut targets = Vec::new();

        if let Some(default_to) = &settings.default_to {
            Self::push_unique_currency(&mut targets, default_to, &from_currency);
        } else {
            Self::push_unique_currency(&mut targets, &self.home_city.currency, &from_currency);
        }

        if settings.sync_with_cities {
            for city in self.all_cities() {
                Self::push_unique_currency(&mut targets, &city.currency, &from_currency);
            }
        }

        for code in &settings.pinned_codes {
            Self::push_unique_currency(&mut targets, code, &from_currency);
        }

        targets
    }

    fn push_unique_currency(targets: &mut Vec<String>, code: &str, from_currency: &str) {
        let code = normalise_currency_code(code);
        if code != from_currency && !targets.iter().any(|value| value == &code) {
            targets.push(code);
        }
    }

    fn validate(&self) -> Result<()> {
        let mut seen = HashSet::new();

        for city in self.all_cities() {
            let code = city.code.trim().to_uppercase();
            if code.is_empty() {
                bail!("city code cannot be empty");
            }
            if !seen.insert(code.clone()) {
                bail!("duplicate city code: {}", code);
            }

            city.timezone.parse::<chrono_tz::Tz>().with_context(|| {
                format!("invalid timezone for {}: {}", city.name, city.timezone)
            })?;

            if !is_valid_currency_code(&city.currency) {
                bail!("invalid currency code for {}: {}", city.name, city.currency);
            }
        }

        if let Some(currency) = &self.currency {
            if let Some(default_from) = &currency.default_from
                && !is_valid_currency_code(default_from)
            {
                bail!("invalid currency.default_from: {}", default_from);
            }
            if let Some(default_to) = &currency.default_to
                && !is_valid_currency_code(default_to)
            {
                bail!("invalid currency.default_to: {}", default_to);
            }
            for code in &currency.pinned_codes {
                if !is_valid_currency_code(code) {
                    bail!("invalid currency.pinned_codes entry: {}", code);
                }
            }
        }

        if let Some(map) = &self.map {
            if let Some(city_code) = &map.focus_city_code
                && !self
                    .all_city_codes()
                    .iter()
                    .any(|code| code.eq_ignore_ascii_case(city_code))
            {
                bail!("unknown map.focus_city_code: {}", city_code);
            }

            if let Some(country_code) = &map.focal_country_code
                && !is_valid_country_code(country_code)
            {
                bail!("invalid map.focal_country_code: {}", country_code);
            }

            for country_code in &map.focus_country_codes {
                if !is_valid_country_code(country_code) {
                    bail!("invalid map.focus_country_codes entry: {}", country_code);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_new_york_city() -> City {
        let mut city = City::boston();
        city.name = "New York".to_string();
        city.code = "NYC".to_string();
        city
    }

    #[test]
    fn normalises_legacy_home_city_to_boston() {
        let mut config = Config::default();
        config.home_city = legacy_new_york_city();

        let updated = config.normalize_legacy_cities();
        assert!(updated);
        assert_eq!(config.home_city.code, "BOS");
        assert_eq!(config.home_city.name, "Boston");
    }

    #[test]
    fn preserves_boston_without_changing() {
        let mut config = Config::default();

        let updated = config.normalize_legacy_cities();
        assert!(!updated);
        assert_eq!(config.home_city.code, "BOS");
        assert_eq!(config.home_city.name, "Boston");
    }

    #[test]
    fn normalises_legacy_tracked_cities_and_dedupes() {
        let mut config = Config::default();
        config.tracked_cities.push(legacy_new_york_city());
        config.tracked_cities.push(City::boston());

        let updated = config.normalize_legacy_cities();
        assert!(updated);
        assert!(
            !config
                .tracked_cities
                .iter()
                .any(|city| city.code.eq_ignore_ascii_case("NYC"))
        );
        let bos_count = config
            .tracked_cities
            .iter()
            .filter(|city| city.code.eq_ignore_ascii_case("BOS"))
            .count();
        assert_eq!(bos_count, 1);
    }

    #[test]
    fn derives_currency_pairs_from_city_set_and_pins() {
        let mut config = Config::default();
        config.currency = Some(CurrencyConfig {
            sync_with_cities: true,
            pinned_codes: vec!["cad".to_string()],
            default_from: Some("nzd".to_string()),
            default_to: Some("sgd".to_string()),
        });
        config.normalize();

        let pairs = config.effective_currency_pairs();

        assert_eq!(
            pairs.first(),
            Some(&(String::from("NZD"), String::from("SGD")))
        );
        assert!(pairs.contains(&(String::from("NZD"), String::from("USD"))));
        assert!(pairs.contains(&(String::from("NZD"), String::from("CAD"))));
    }

    #[test]
    fn validates_map_focus_city_against_known_cities() {
        let mut config = Config::default();
        config.map = Some(MapConfig {
            mode: MapMode::Cities,
            focus_city_code: Some("XXX".to_string()),
            focus_country_codes: Vec::new(),
            focal_country_code: None,
        });

        let err = config.validate().expect_err("expected validation failure");
        assert!(err.to_string().contains("unknown map.focus_city_code"));
    }

    #[test]
    fn derives_default_focal_country_from_current_city() {
        let config = Config::default();
        let map = config.effective_map_settings();

        assert_eq!(map.focal_country_code.as_deref(), Some("NZL"));
    }
}
