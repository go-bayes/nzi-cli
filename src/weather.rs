//! weather fetching module using open-meteo api (free, fast, no api key)
//! faster than wttr.in with better caching

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::{Duration, Instant};

/// weather condition icons
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeatherIcon {
    Sunny,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    HeavyRain,
    Snow,
    Thunderstorm,
    Unknown,
}

impl WeatherIcon {
    /// get an icon character for display
    pub fn icon(&self, is_day: bool) -> &'static str {
        match (self, is_day) {
            (Self::Sunny, true) => "☀",
            (Self::Sunny, false) => "☾",
            (Self::PartlyCloudy, true) => "⛅",
            (Self::PartlyCloudy, false) => "☁️",
            (Self::Cloudy, _) => "☁️",
            (Self::Fog, _) => "🌫",
            (Self::Drizzle, _) => "🌦",
            (Self::Rain, _) => "🌧",
            (Self::HeavyRain, _) => "🌧",
            (Self::Snow, _) => "❄",
            (Self::Thunderstorm, _) => "⛈",
            (Self::Unknown, _) => "?",
        }
    }

    /// parse from wmo weather code (open-meteo uses wmo codes)
    pub fn from_wmo_code(code: i32) -> Self {
        match code {
            0 => Self::Sunny,
            1 | 2 => Self::PartlyCloudy,
            3 => Self::Cloudy,
            45 | 48 => Self::Fog,
            51 | 53 | 55 | 56 | 57 => Self::Drizzle,
            61 | 63 | 80 | 81 => Self::Rain,
            65 | 66 | 67 | 82 => Self::HeavyRain,
            71 | 73 | 75 | 77 | 85 | 86 => Self::Snow,
            95 | 96 | 99 => Self::Thunderstorm,
            _ => Self::Unknown,
        }
    }
}

/// time of day period
#[derive(Debug, Clone, Copy)]
pub enum TimeOfDay {
    Morning, // 6-12
    Noon,    // 12-18
    Evening, // 18-24
    Night,   // 0-6
}

impl TimeOfDay {
    pub fn hour_range(&self) -> (usize, usize) {
        match self {
            TimeOfDay::Night => (0, 6),
            TimeOfDay::Morning => (6, 12),
            TimeOfDay::Noon => (12, 18),
            TimeOfDay::Evening => (18, 24),
        }
    }
}

/// period forecast (morning/noon/evening/night)
#[derive(Debug, Clone)]
pub struct PeriodForecast {
    pub period: TimeOfDay,
    pub temp: i32,
    pub wind: i32,
    pub wind_dir: String,
    pub icon: WeatherIcon,
}

/// daily forecast data with period breakdowns
#[derive(Debug, Clone)]
pub struct DayForecast {
    pub date: String,
    pub temp_max: i32,
    pub temp_min: i32,
    pub wind_max: i32,
    pub icon: WeatherIcon,
    pub periods: Vec<PeriodForecast>,
}

/// current weather data
#[derive(Debug, Clone)]
pub struct CurrentWeather {
    pub temp_c: i32,
    pub feels_like_c: i32,
    pub humidity: i32,
    pub wind_kmph: i32,
    pub wind_dir: String,
    pub description: String,
    pub icon: WeatherIcon,
    pub is_day: bool,
    pub last_updated: Instant,
    pub forecast: Vec<DayForecast>,
}

impl CurrentWeather {
    /// check if data is stale (older than 10 minutes)
    pub fn is_stale(&self) -> bool {
        self.last_updated.elapsed() > Duration::from_secs(600)
    }

    /// format temperature
    pub fn temp_string(&self) -> String {
        format!("{}°C", self.temp_c)
    }

    /// format feels like
    pub fn feels_like_string(&self) -> String {
        format!("{}°C", self.feels_like_c)
    }
}

/// open-meteo api response
#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
    daily: Option<OpenMeteoDaily>,
    hourly: Option<OpenMeteoHourly>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: i32,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
    weather_code: i32,
    is_day: i32,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoDaily {
    time: Vec<String>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    wind_speed_10m_max: Vec<f64>,
    weather_code: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoHourly {
    #[allow(dead_code)]
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    wind_speed_10m: Vec<f64>,
    wind_direction_10m: Vec<f64>,
    weather_code: Vec<i32>,
}

/// city coordinates for weather lookup
pub struct CityCoords {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
}

/// known city coordinates
pub const CITY_COORDS: &[CityCoords] = &[
    CityCoords {
        name: "wellington",
        lat: -41.2865,
        lon: 174.7762,
    },
    CityCoords {
        name: "auckland",
        lat: -36.8485,
        lon: 174.7633,
    },
    CityCoords {
        name: "christchurch",
        lat: -43.5321,
        lon: 172.6362,
    },
    CityCoords {
        name: "dunedin",
        lat: -45.8788,
        lon: 170.5028,
    },
    CityCoords {
        name: "hamilton",
        lat: -37.7870,
        lon: 175.2793,
    },
    CityCoords {
        name: "tauranga",
        lat: -37.6878,
        lon: 176.1651,
    },
    CityCoords {
        name: "new plymouth",
        lat: -39.0556,
        lon: 174.0752,
    },
    CityCoords {
        name: "nelson",
        lat: -41.2706,
        lon: 173.2840,
    },
    CityCoords {
        name: "queenstown",
        lat: -45.0312,
        lon: 168.6626,
    },
    CityCoords {
        name: "new york",
        lat: 40.7128,
        lon: -74.0060,
    },
    CityCoords {
        name: "london",
        lat: 51.5074,
        lon: -0.1278,
    },
    CityCoords {
        name: "sydney",
        lat: -33.8688,
        lon: 151.2093,
    },
    CityCoords {
        name: "tokyo",
        lat: 35.6762,
        lon: 139.6503,
    },
    CityCoords {
        name: "singapore",
        lat: 1.3521,
        lon: 103.8198,
    },
    CityCoords {
        name: "los angeles",
        lat: 34.0522,
        lon: -118.2437,
    },
    CityCoords {
        name: "san francisco",
        lat: 37.7749,
        lon: -122.4194,
    },
    CityCoords {
        name: "paris",
        lat: 48.8566,
        lon: 2.3522,
    },
    CityCoords {
        name: "austin",
        lat: 30.2672,
        lon: -97.7431,
    },
];

/// get coordinates for a city name
fn get_city_coords(city_name: &str) -> Option<(f64, f64)> {
    let name_lower = city_name.to_lowercase();
    CITY_COORDS
        .iter()
        .find(|c| name_lower.contains(c.name))
        .map(|c| (c.lat, c.lon))
}

/// wind direction from degrees
fn wind_direction(degrees: f64) -> &'static str {
    let dirs = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let idx = ((degrees + 22.5) / 45.0) as usize % 8;
    dirs[idx]
}

/// average a collection of wind directions (degrees) safely on a circle
fn average_wind_direction(degrees: &[f64]) -> Option<f64> {
    if degrees.is_empty() {
        return None;
    }

    let (sin_sum, cos_sum) = degrees.iter().fold((0.0, 0.0), |(s, c), deg| {
        let rad = deg.to_radians();
        (s + rad.sin(), c + rad.cos())
    });

    if sin_sum == 0.0 && cos_sum == 0.0 {
        return None;
    }

    let mut mean = sin_sum.atan2(cos_sum).to_degrees();
    if mean < 0.0 {
        mean += 360.0;
    }
    Some(mean)
}

/// weather description from wmo code
fn weather_description(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Foggy",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snow",
        77 => "Snow grains",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

/// weather service with caching
pub struct WeatherService {
    client: reqwest::Client,
    cache: std::collections::HashMap<String, CurrentWeather>,
}

impl WeatherService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            cache: std::collections::HashMap::new(),
        }
    }

    /// get weather for a location (city name)
    pub async fn get_weather(&mut self, location: &str) -> Result<CurrentWeather> {
        let cache_key = location.to_lowercase();

        // check cache
        if let Some(cached) = self.cache.get(&cache_key)
            && !cached.is_stale()
        {
            return Ok(cached.clone());
        }

        // fetch from open-meteo
        let weather = self.fetch_weather(location).await?;
        self.cache.insert(cache_key, weather.clone());
        Ok(weather)
    }

    async fn fetch_weather(&self, location: &str) -> Result<CurrentWeather> {
        let (lat, lon) =
            get_city_coords(location).context("unknown city - add coordinates to CITY_COORDS")?;

        // open-meteo api - fast and free, with 3-day forecast + hourly for period breakdown
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,wind_direction_10m,weather_code,is_day&daily=temperature_2m_max,temperature_2m_min,wind_speed_10m_max,weather_code&hourly=temperature_2m,wind_speed_10m,wind_direction_10m,weather_code&timezone=auto&forecast_days=3",
            lat, lon
        );

        let response: OpenMeteoResponse = self
            .client
            .get(&url)
            .send()
            .await
            .context("failed to fetch weather")?
            .json()
            .await
            .context("failed to parse weather response")?;

        let current = &response.current;

        // parse hourly data into periods for each day
        let hourly_periods = if let Some(hourly) = &response.hourly {
            parse_hourly_to_periods(hourly)
        } else {
            Vec::new()
        };

        // parse 3-day forecast with period breakdowns
        let forecast = if let Some(daily) = &response.daily {
            daily
                .time
                .iter()
                .enumerate()
                .take(3)
                .map(|(i, date)| {
                    // get periods for this day
                    let day_periods = if i < hourly_periods.len() {
                        hourly_periods[i].clone()
                    } else {
                        Vec::new()
                    };

                    DayForecast {
                        date: date.clone(),
                        temp_max: daily
                            .temperature_2m_max
                            .get(i)
                            .map(|t| t.round() as i32)
                            .unwrap_or(0),
                        temp_min: daily
                            .temperature_2m_min
                            .get(i)
                            .map(|t| t.round() as i32)
                            .unwrap_or(0),
                        wind_max: daily
                            .wind_speed_10m_max
                            .get(i)
                            .map(|w| w.round() as i32)
                            .unwrap_or(0),
                        icon: WeatherIcon::from_wmo_code(
                            daily.weather_code.get(i).copied().unwrap_or(0),
                        ),
                        periods: day_periods,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(CurrentWeather {
            temp_c: current.temperature_2m.round() as i32,
            feels_like_c: current.apparent_temperature.round() as i32,
            humidity: current.relative_humidity_2m,
            wind_kmph: current.wind_speed_10m.round() as i32,
            wind_dir: wind_direction(current.wind_direction_10m).to_string(),
            description: weather_description(current.weather_code).to_string(),
            icon: WeatherIcon::from_wmo_code(current.weather_code),
            is_day: current.is_day == 1,
            last_updated: Instant::now(),
            forecast,
        })
    }
}

/// parse hourly data into period forecasts (4 periods per day for 3 days)
fn parse_hourly_to_periods(hourly: &OpenMeteoHourly) -> Vec<Vec<PeriodForecast>> {
    let periods = [
        TimeOfDay::Morning,
        TimeOfDay::Noon,
        TimeOfDay::Evening,
        TimeOfDay::Night,
    ];
    let mut result = Vec::new();

    // 3 days * 24 hours = 72 hourly entries
    for day in 0..3 {
        let mut day_periods = Vec::new();
        for period in &periods {
            let (start, end) = period.hour_range();
            let day_offset = day * 24;

            // average temperature and max wind for the period
            let mut temps = Vec::new();
            let mut winds = Vec::new();
            let mut wind_dirs = Vec::new();
            let mut codes = Vec::new();

            for hour in start..end {
                let idx = day_offset + hour;
                if idx < hourly.temperature_2m.len()
                    && idx < hourly.wind_speed_10m.len()
                    && idx < hourly.wind_direction_10m.len()
                    && idx < hourly.weather_code.len()
                {
                    temps.push(hourly.temperature_2m[idx]);
                    winds.push(hourly.wind_speed_10m[idx]);
                    wind_dirs.push(hourly.wind_direction_10m[idx]);
                    codes.push(hourly.weather_code[idx]);
                }
            }

            if !temps.is_empty() {
                let avg_temp = temps.iter().sum::<f64>() / temps.len() as f64;
                let max_wind = winds.iter().cloned().fold(0.0_f64, f64::max);
                let avg_wind_dir = average_wind_direction(&wind_dirs);
                // use most common weather code in period
                let mode_code = codes
                    .iter()
                    .max_by_key(|c| codes.iter().filter(|x| *x == *c).count())
                    .copied()
                    .unwrap_or(0);

                day_periods.push(PeriodForecast {
                    period: *period,
                    temp: avg_temp.round() as i32,
                    wind: max_wind.round() as i32,
                    wind_dir: avg_wind_dir.map(wind_direction).unwrap_or("?").to_string(),
                    icon: WeatherIcon::from_wmo_code(mode_code),
                });
            }
        }
        result.push(day_periods);
    }

    result
}

impl Default for WeatherService {
    fn default() -> Self {
        Self::new()
    }
}
