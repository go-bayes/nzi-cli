//! time zone handling and conversion module
//! supports any timezone via chrono-tz

use chrono::{DateTime, FixedOffset, Local, LocalResult, Offset, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

use crate::config::City;

/// time information for a city
#[derive(Debug, Clone)]
pub struct CityTime {
    pub city_name: String,
    pub city_code: String,
    pub datetime: DateTime<Tz>,
    pub offset_hours: f32,
}

impl CityTime {
    /// create a new city time from a city configuration
    pub fn from_city(city: &City) -> Option<Self> {
        let tz: Tz = city.timezone.parse().ok()?;
        let utc_now = Utc::now();
        let datetime = utc_now.with_timezone(&tz);

        // calculate offset in hours using the fixed offset
        let fixed: FixedOffset = datetime.offset().fix();
        let offset_secs = fixed.local_minus_utc();
        let offset_hours = offset_secs as f32 / 3600.0;

        Some(Self {
            city_name: city.name.clone(),
            city_code: city.code.clone(),
            datetime,
            offset_hours,
        })
    }

    /// get the time formatted for display
    pub fn time_string(&self, use_24_hour: bool, show_seconds: bool) -> String {
        let format = match (use_24_hour, show_seconds) {
            (true, true) => "%H:%M:%S",
            (true, false) => "%H:%M",
            (false, true) => "%I:%M:%S %p",
            (false, false) => "%I:%M %p",
        };
        self.datetime.format(format).to_string()
    }

    /// get the hour for clock display (0-23)
    pub fn hour(&self) -> u32 {
        self.datetime.hour()
    }

    /// check if it's daytime (between 6am and 6pm)
    pub fn is_daytime(&self) -> bool {
        let hour = self.hour();
        (6..18).contains(&hour)
    }
}

/// time zone service for managing multiple city times
pub struct TimezoneService {
    cities: Vec<CityTime>,
}

impl TimezoneService {
    pub fn new() -> Self {
        Self { cities: Vec::new() }
    }

    /// update all city times
    pub fn update(&mut self, cities: &[&City]) {
        self.cities = cities
            .iter()
            .filter_map(|city| CityTime::from_city(city))
            .collect();
    }

    /// get time for a specific city by code
    fn get_city_time(&self, code: &str) -> Option<&CityTime> {
        self.cities.iter().find(|c| c.city_code == code)
    }

    /// convert a time from one city to another (DST-aware for the current date)
    pub fn convert_time(
        &self,
        from_city_code: &str,
        to_city_code: &str,
        hour: u32,
        minute: u32,
    ) -> Option<(u32, u32, i32)> {
        let from_city = self.get_city_time(from_city_code)?;
        let to_city = self.get_city_time(to_city_code)?;

        let from_tz = from_city.datetime.timezone();
        let to_tz = to_city.datetime.timezone();
        let from_date = from_city.datetime.date_naive();
        let naive_local = from_date.and_hms_opt(hour, minute, 0)?;

        let from_datetime = match from_tz.from_local_datetime(&naive_local) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(first, second) => {
                // prefer the earlier (usually standard) offset when ambiguous
                let first_offset = first.offset().fix().local_minus_utc();
                let second_offset = second.offset().fix().local_minus_utc();
                if first_offset <= second_offset {
                    first
                } else {
                    second
                }
            }
            LocalResult::None => return None, // skipped hour (spring forward)
        };

        let target = from_datetime.with_timezone(&to_tz);
        let day_offset = target
            .date_naive()
            .signed_duration_since(from_datetime.date_naive())
            .num_days() as i32;

        Some((target.hour(), target.minute(), day_offset))
    }
}

impl Default for TimezoneService {
    fn default() -> Self {
        Self::new()
    }
}

/// time converter widget state
#[derive(Debug, Clone)]
pub struct TimeConverter {
    pub from_city_code: String,
    pub to_city_code: String,
    pub input_hour: u32,
    pub input_minute: u32,
    pub result_hour: u32,
    pub result_minute: u32,
    pub day_offset: i32,
    /// buffer for direct time input (e.g. "1430" for 14:30)
    pub input_buffer: String,
}

impl Default for TimeConverter {
    fn default() -> Self {
        let now = Local::now();
        Self {
            from_city_code: "WLG".to_string(),
            to_city_code: "NYC".to_string(),
            input_hour: now.hour(),
            input_minute: now.minute(),
            result_hour: 0,
            result_minute: 0,
            day_offset: 0,
            input_buffer: String::new(),
        }
    }
}

impl TimeConverter {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from_city_code: from.to_string(),
            to_city_code: to.to_string(),
            ..Default::default()
        }
    }

    pub fn update_result(&mut self, hour: u32, minute: u32, day_offset: i32) {
        self.result_hour = hour;
        self.result_minute = minute;
        self.day_offset = day_offset;
    }

    pub fn swap_cities(&mut self) {
        std::mem::swap(&mut self.from_city_code, &mut self.to_city_code);
    }

    /// cycle the "to" city through available cities
    pub fn cycle_to_city(&mut self, city_codes: &[String]) {
        if city_codes.is_empty() {
            return;
        }
        // find current position
        let current_idx = city_codes
            .iter()
            .position(|c| c == &self.to_city_code)
            .unwrap_or(0);
        // move to next, skipping if it matches from_city
        let mut next_idx = (current_idx + 1) % city_codes.len();
        // skip the from city if we land on it
        if city_codes[next_idx] == self.from_city_code {
            next_idx = (next_idx + 1) % city_codes.len();
        }
        self.to_city_code = city_codes[next_idx].clone();
    }

    pub fn increment_hour(&mut self) {
        self.input_hour = (self.input_hour + 1) % 24;
    }

    pub fn decrement_hour(&mut self) {
        self.input_hour = if self.input_hour == 0 {
            23
        } else {
            self.input_hour - 1
        };
    }

    pub fn increment_minute(&mut self) {
        self.input_minute = (self.input_minute + 1) % 60;
    }

    pub fn decrement_minute(&mut self) {
        self.input_minute = if self.input_minute == 0 {
            59
        } else {
            self.input_minute - 1
        };
    }

    pub fn format_input_time(&self) -> String {
        format!("{:02}:{:02}", self.input_hour, self.input_minute)
    }

    pub fn format_result_time(&self) -> String {
        let time = format!("{:02}:{:02}", self.result_hour, self.result_minute);
        match self.day_offset {
            -1 => format!("{} (yesterday)", time),
            1 => format!("{} (tomorrow)", time),
            _ => time,
        }
    }

    pub fn set_to_now(&mut self) {
        let now = Local::now();
        self.input_hour = now.hour();
        self.input_minute = now.minute();
    }

    /// reset to midnight (00:00)
    pub fn reset(&mut self) {
        self.input_hour = 0;
        self.input_minute = 0;
        self.input_buffer.clear();
    }

    /// handle a digit input for direct time entry
    pub fn handle_digit(&mut self, digit: char) {
        if self.input_buffer.len() < 4 {
            self.input_buffer.push(digit);
            self.parse_input_buffer();
        }
    }

    /// handle backspace for direct time entry
    pub fn handle_backspace(&mut self) {
        self.input_buffer.pop();
        self.parse_input_buffer();
    }

    /// clear input buffer
    pub fn clear_input_buffer(&mut self) {
        self.input_buffer.clear();
    }

    /// parse input buffer into hour and minute
    fn parse_input_buffer(&mut self) {
        if self.input_buffer.is_empty() {
            return;
        }

        let digits: Vec<u32> = self
            .input_buffer
            .chars()
            .filter_map(|c| c.to_digit(10))
            .collect();

        match digits.len() {
            1 => {
                self.input_hour = digits[0].min(23);
                self.input_minute = 0;
            }
            2 => {
                let hour = digits[0] * 10 + digits[1];
                self.input_hour = hour.min(23);
                self.input_minute = 0;
            }
            3 => {
                let hour = digits[0];
                let minute = digits[1] * 10 + digits[2];
                if minute <= 59 {
                    self.input_hour = hour.min(23);
                    self.input_minute = minute;
                } else {
                    let hour = digits[0] * 10 + digits[1];
                    self.input_hour = hour.min(23);
                    self.input_minute = digits[2];
                }
            }
            4 => {
                let hour = digits[0] * 10 + digits[1];
                let minute = digits[2] * 10 + digits[3];
                self.input_hour = hour.min(23);
                self.input_minute = minute.min(59);
            }
            _ => {}
        }
    }

    /// check if currently typing time
    pub fn is_typing(&self) -> bool {
        !self.input_buffer.is_empty()
    }

    /// format input display (shows buffer if typing)
    pub fn format_input_display(&self) -> String {
        if self.input_buffer.is_empty() {
            self.format_input_time()
        } else {
            match self.input_buffer.len() {
                1 => format!("{}█:__", self.input_buffer),
                2 => format!("{}█:__", self.input_buffer),
                3 => format!(
                    "{}:{}█_",
                    &self.input_buffer[0..2],
                    &self.input_buffer[2..3]
                ),
                4 => format!("{}:{}", &self.input_buffer[0..2], &self.input_buffer[2..4]),
                _ => self.format_input_time(),
            }
        }
    }
}
