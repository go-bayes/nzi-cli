//! configuration management for nzi-cli
//! handles loading and saving user preferences from ~/.config/nzi-cli/config.toml
//! follows margo-style config: simple toml with manual parsing

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

    pub fn new_york() -> Self {
        Self {
            name: "New York".to_string(),
            code: "NYC".to_string(),
            country: "USA".to_string(),
            timezone: "America/New_York".to_string(),
            currency: "USD".to_string(),
        }
    }

    pub fn london() -> Self {
        Self {
            name: "London".to_string(),
            code: "LDN".to_string(),
            country: "UK".to_string(),
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

    pub fn paris() -> Self {
        Self {
            name: "Paris".to_string(),
            code: "PAR".to_string(),
            country: "France".to_string(),
            timezone: "Europe/Paris".to_string(),
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
            // new york as primary world city (for "around the world")
            home_city: City::new_york(),
            // track other world cities for world clock
            tracked_cities: vec![
                City::london(),
                City::los_angeles(),
                City::austin(),
                City::paris(),
                City::sydney(),
                City::tokyo(),
                City::singapore(),
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
            let config: Config = toml::from_str(&content).context("failed to parse config file")?;
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
}
