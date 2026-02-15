//! configuration management for nzi-cli
//! handles loading and saving user preferences from ~/.config/nzi-cli/config.toml
//! follows margo-style config: simple toml with manual parsing

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

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
            updated |= config.ensure_tracked_city(City::rio());
            updated |= config.ensure_tracked_city(City::addis_ababa());
            updated |= config.ensure_tracked_city(City::kuala_lumpur());
            updated |= config.ensure_tracked_city(City::berlin());
            updated |= config.ensure_tracked_city(City::dhaka());
            updated |= config.ensure_tracked_city(City::beijing());
            if updated {
                config.save()?;
            }
            Ok(config)
        } else {
            // create default config
            let config = Config::default();
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

        let content = toml::to_string_pretty(self).context("failed to serialise config")?;

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

    fn ensure_tracked_city(&mut self, city: City) -> bool {
        if self
            .tracked_cities
            .iter()
            .any(|c| c.code.eq_ignore_ascii_case(&city.code) || c.name.eq_ignore_ascii_case(&city.name))
        {
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

        self.tracked_cities.retain(|city| seen.insert(city.code.to_uppercase()));

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
        assert!(!config
            .tracked_cities
            .iter()
            .any(|city| city.code.eq_ignore_ascii_case("NYC")));
        let bos_count = config
            .tracked_cities
            .iter()
            .filter(|city| city.code.eq_ignore_ascii_case("BOS"))
            .count();
        assert_eq!(bos_count, 1);
    }
}
