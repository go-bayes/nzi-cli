//! exchange rate fetching and conversion module
//! supports any currency pair with caching

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// cached exchange rate data
#[derive(Debug, Clone)]
struct CachedRate {
    rate: f64,
    last_updated: Instant,
}

impl CachedRate {
    fn is_stale(&self) -> bool {
        self.last_updated.elapsed() > Duration::from_secs(600)
    }
}

/// exchange rate service with caching
pub struct ExchangeService {
    cache: HashMap<String, CachedRate>,
    client: reqwest::Client,
    // fallback rates when offline (approximate rates as of 2024)
    fallback_rates: HashMap<String, f64>,
}

impl ExchangeService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        // fallback rates relative to NZD (approximate)
        let mut fallback_rates = HashMap::new();
        fallback_rates.insert("NZD".to_string(), 1.0);
        fallback_rates.insert("USD".to_string(), 0.60);
        fallback_rates.insert("EUR".to_string(), 0.55);
        fallback_rates.insert("GBP".to_string(), 0.47);
        fallback_rates.insert("AUD".to_string(), 0.92);
        fallback_rates.insert("JPY".to_string(), 90.0);

        Self {
            cache: HashMap::new(),
            client,
            fallback_rates,
        }
    }

    /// get the cache key for a currency pair
    fn cache_key(from: &str, to: &str) -> String {
        format!("{}_{}", from.to_uppercase(), to.to_uppercase())
    }

    /// get exchange rate, using cache if available and not stale
    pub async fn get_rate(&mut self, from: &str, to: &str) -> Result<f64> {
        let key = Self::cache_key(from, to);

        // check cache first
        if let Some(cached) = self.cache.get(&key) {
            if !cached.is_stale() {
                return Ok(cached.rate);
            }
        }

        // try to fetch fresh rate
        match self.fetch_rate(from, to).await {
            Ok(rate) => {
                self.cache.insert(
                    key,
                    CachedRate {
                        rate,
                        last_updated: Instant::now(),
                    },
                );
                Ok(rate)
            }
            Err(_) => {
                // use fallback rates if API fails
                self.get_fallback_rate(from, to)
            }
        }
    }

    /// fetch rate from the API
    async fn fetch_rate(&self, from: &str, to: &str) -> Result<f64> {
        // using the free exchangerate-api
        let url = format!(
            "https://api.exchangerate-api.com/v4/latest/{}",
            from.to_uppercase()
        );

        let response: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("failed to fetch exchange rate")?
            .json()
            .await
            .context("failed to parse exchange rate response")?;

        let rates = response["rates"]
            .as_object()
            .context("invalid response format")?;

        let to_upper = to.to_uppercase();
        rates
            .get(&to_upper)
            .and_then(|v| v.as_f64())
            .context("currency not found in response")
    }

    /// get fallback rate when offline
    fn get_fallback_rate(&self, from: &str, to: &str) -> Result<f64> {
        let from_upper = from.to_uppercase();
        let to_upper = to.to_uppercase();

        // convert through NZD as the base
        let from_to_nzd = self
            .fallback_rates
            .get(&from_upper)
            .map(|r| 1.0 / r)
            .unwrap_or(1.0);

        let nzd_to_to = self.fallback_rates.get(&to_upper).copied().unwrap_or(1.0);

        Ok(from_to_nzd * nzd_to_to)
    }
}

impl Default for ExchangeService {
    fn default() -> Self {
        Self::new()
    }
}

/// available currency pairs (NZD to world currencies)
pub const CURRENCY_PAIRS: &[(&str, &str)] = &[
    ("NZD", "USD"),
    ("NZD", "EUR"),
    ("NZD", "GBP"),
    ("NZD", "AUD"),
    ("NZD", "JPY"),
];

/// currency converter widget state
#[derive(Debug, Clone)]
pub struct CurrencyConverter {
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount: f64,
    pub to_amount: f64,
    pub rate: Option<f64>,
    pub input_buffer: String,
    pub editing: bool,
    pub pair_index: usize,
    pub needs_refresh: bool,
}

impl Default for CurrencyConverter {
    fn default() -> Self {
        Self {
            from_currency: "NZD".to_string(),
            to_currency: "USD".to_string(),
            from_amount: 100.0,
            to_amount: 0.0,
            rate: None,
            input_buffer: "100".to_string(),
            editing: false,
            pair_index: 0,
            needs_refresh: true,
        }
    }
}

impl CurrencyConverter {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from_currency: from.to_uppercase(),
            to_currency: to.to_uppercase(),
            needs_refresh: true,
            ..Default::default()
        }
    }

    pub fn update_rate(&mut self, rate: f64) {
        self.rate = Some(rate);
        self.needs_refresh = false;
        self.recalculate();
    }

    pub fn set_amount(&mut self, amount: f64) {
        self.from_amount = amount;
        self.recalculate();
    }

    fn recalculate(&mut self) {
        self.to_amount = self.rate.map(|r| self.from_amount * r).unwrap_or(0.0);
    }

    pub fn swap_currencies(&mut self) {
        std::mem::swap(&mut self.from_currency, &mut self.to_currency);
        if let Some(rate) = self.rate {
            self.rate = Some(1.0 / rate);
            self.recalculate();
        } else {
            self.to_amount = 0.0;
            self.needs_refresh = true;
        }
    }

    pub fn handle_input(&mut self, c: char) {
        if c.is_ascii_digit() || (c == '.' && !self.input_buffer.contains('.')) {
            self.input_buffer.push(c);
            if let Ok(amount) = self.input_buffer.parse::<f64>() {
                self.set_amount(amount);
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        self.input_buffer.pop();
        if self.input_buffer.is_empty() {
            self.set_amount(0.0);
        } else if let Ok(amount) = self.input_buffer.parse::<f64>() {
            self.set_amount(amount);
        }
    }

    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.set_amount(0.0);
    }

    /// cycle to the next currency pair
    pub fn cycle_pair(&mut self) {
        self.pair_index = (self.pair_index + 1) % CURRENCY_PAIRS.len();
        let (from, to) = CURRENCY_PAIRS[self.pair_index];
        self.from_currency = from.to_string();
        self.to_currency = to.to_string();
        self.rate = None;
        self.to_amount = 0.0;
        self.needs_refresh = true;
        self.recalculate();
    }

    /// check if rate refresh is needed
    pub fn needs_rate_refresh(&self) -> bool {
        self.needs_refresh || self.rate.is_none()
    }

    /// clear the refresh flag
    pub fn clear_refresh_flag(&mut self) {
        self.needs_refresh = false;
    }
}
