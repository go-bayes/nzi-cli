//! exchange rate fetching and conversion module
//! supports any currency pair with caching

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// cached exchange rate data
#[derive(Debug, Clone)]
struct CachedRate {
    rate: f64,
    last_updated: Instant,
}

/// bounds for a believable rate between two currencies. real cross rates span
/// roughly 1e-8 to 1e8, so anything outside these bounds is a bad response
const MIN_PLAUSIBLE_RATE: f64 = 1e-12;
const MAX_PLAUSIBLE_RATE: f64 = 1e12;

/// accept only a finite, positive rate within the plausible range
fn validate_rate(rate: f64) -> Result<f64> {
    if !rate.is_finite() || !(MIN_PLAUSIBLE_RATE..=MAX_PLAUSIBLE_RATE).contains(&rate) {
        bail!("exchange rate in response is out of range");
    }
    Ok(rate)
}

/// extract and validate the rate for `to` from an api response
fn rate_from_response(response: &serde_json::Value, to: &str) -> Result<f64> {
    let rates = response["rates"]
        .as_object()
        .context("invalid response format")?;

    let rate = rates
        .get(&to.to_uppercase())
        .and_then(|v| v.as_f64())
        .context("currency not found in response")?;

    validate_rate(rate)
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
}

impl ExchangeService {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            client: crate::http::build_client(None),
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
        if let Some(cached) = self.cache.get(&key)
            && !cached.is_stale()
        {
            return Ok(cached.rate);
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
                // use cached/identity fallback if API fails
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

        let response: serde_json::Value = crate::http::get_json(&self.client, &url)
            .await
            .context("failed to fetch exchange rate")?;

        rate_from_response(&response, to)
    }

    /// get fallback rate when offline
    fn get_fallback_rate(&self, from: &str, to: &str) -> Result<f64> {
        let from_upper = from.to_uppercase();
        let to_upper = to.to_uppercase();

        if from_upper == to_upper {
            return Ok(1.0);
        }

        let key = Self::cache_key(&from_upper, &to_upper);
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached.rate);
        }

        let reverse_key = Self::cache_key(&to_upper, &from_upper);
        if let Some(cached) = self.cache.get(&reverse_key) {
            if cached.rate == 0.0 {
                bail!(
                    "cached exchange rate is zero for {} -> {}",
                    to_upper,
                    from_upper
                );
            }
            return Ok(1.0 / cached.rate);
        }

        bail!("rate unavailable (offline, no cache)")
    }
}

impl Default for ExchangeService {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub available_pairs: Vec<(String, String)>,
}

impl Default for CurrencyConverter {
    fn default() -> Self {
        let available_pairs = vec![
            ("NZD".to_string(), "USD".to_string()),
            ("NZD".to_string(), "EUR".to_string()),
            ("NZD".to_string(), "GBP".to_string()),
            ("NZD".to_string(), "AUD".to_string()),
            ("NZD".to_string(), "MYR".to_string()),
            ("NZD".to_string(), "JPY".to_string()),
            ("NZD".to_string(), "BRL".to_string()),
            ("NZD".to_string(), "ETB".to_string()),
            ("NZD".to_string(), "BDT".to_string()),
            ("NZD".to_string(), "CNY".to_string()),
        ];
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
            available_pairs,
        }
    }
}

impl CurrencyConverter {
    pub fn new_with_pairs(from: &str, to: &str, available_pairs: Vec<(String, String)>) -> Self {
        let from_currency = from.to_uppercase();
        let to_currency = to.to_uppercase();
        let available_pairs = Self::normalise_pairs(available_pairs, &from_currency, &to_currency);
        let pair_index = available_pairs
            .iter()
            .position(|(pair_from, pair_to)| pair_from == &from_currency && pair_to == &to_currency)
            .unwrap_or(0);

        Self {
            from_currency,
            to_currency,
            pair_index,
            available_pairs,
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

    pub fn set_pair(&mut self, from: &str, to: &str) {
        self.from_currency = from.trim().to_uppercase();
        self.to_currency = to.trim().to_uppercase();
        self.pair_index = self
            .available_pairs
            .iter()
            .position(|(pair_from, pair_to)| {
                pair_from.eq_ignore_ascii_case(&self.from_currency)
                    && pair_to.eq_ignore_ascii_case(&self.to_currency)
            })
            .unwrap_or(0);
        self.rate = None;
        self.to_amount = 0.0;
        self.needs_refresh = true;
        self.recalculate();
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

    /// check if rate refresh is needed
    pub fn needs_rate_refresh(&self) -> bool {
        self.needs_refresh || self.rate.is_none()
    }

    /// clear the refresh flag
    pub fn clear_refresh_flag(&mut self) {
        self.needs_refresh = false;
    }

    fn normalise_pairs(
        available_pairs: Vec<(String, String)>,
        default_from: &str,
        default_to: &str,
    ) -> Vec<(String, String)> {
        let mut pairs = Vec::new();

        for (from, to) in available_pairs {
            let from = from.trim().to_uppercase();
            let to = to.trim().to_uppercase();

            if from.is_empty() || to.is_empty() {
                continue;
            }

            let pair = (from, to);
            if !pairs.contains(&pair) {
                pairs.push(pair);
            }
        }

        let default_pair = (default_from.to_string(), default_to.to_string());
        if !pairs.contains(&default_pair) {
            pairs.insert(0, default_pair);
        }

        pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate_from_json(body: &str, to: &str) -> Result<f64> {
        let response: serde_json::Value =
            serde_json::from_str(body).expect("test body should be json");
        rate_from_response(&response, to)
    }

    #[test]
    fn valid_rate_is_accepted() {
        let rate = rate_from_json(r#"{"base":"NZD","rates":{"USD":0.58}}"#, "usd")
            .expect("ordinary rate should be accepted");
        assert_eq!(rate, 0.58);
    }

    #[test]
    fn extreme_real_world_rates_are_accepted() {
        // widest real cross rates are near 1e8 and 1e-8
        assert!(rate_from_json(r#"{"rates":{"IRR":70612375.87}}"#, "IRR").is_ok());
        assert!(rate_from_json(r#"{"rates":{"KWD":0.0000000142}}"#, "KWD").is_ok());
    }

    #[test]
    fn zero_and_negative_rates_are_rejected() {
        assert!(rate_from_json(r#"{"rates":{"USD":0}}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"rates":{"USD":0.0}}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"rates":{"USD":-0.58}}"#, "USD").is_err());
    }

    #[test]
    fn absurd_rates_are_rejected() {
        assert!(rate_from_json(r#"{"rates":{"USD":1e300}}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"rates":{"USD":1e-300}}"#, "USD").is_err());
    }

    #[test]
    fn non_finite_rates_are_rejected() {
        assert!(validate_rate(f64::NAN).is_err());
        assert!(validate_rate(f64::INFINITY).is_err());
        assert!(validate_rate(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn malformed_responses_are_rejected() {
        assert!(rate_from_json(r#"{"rates":{"USD":"0.58"}}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"rates":{"USD":null}}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"rates":{"EUR":0.5}}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"rates":[0.58]}"#, "USD").is_err());
        assert!(rate_from_json(r#"{"error":"nope"}"#, "USD").is_err());
        assert!(rate_from_json(r#"[]"#, "USD").is_err());
    }

    #[test]
    fn swapping_a_validated_rate_stays_finite() {
        let mut converter = CurrencyConverter::default();
        for rate in [MIN_PLAUSIBLE_RATE, MAX_PLAUSIBLE_RATE] {
            converter.update_rate(validate_rate(rate).expect("bound should be valid"));
            converter.swap_currencies();
            assert!(converter.rate.is_some_and(f64::is_finite));
            assert!(converter.to_amount.is_finite());
        }
    }
}
