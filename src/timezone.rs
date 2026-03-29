//! time zone handling and conversion module
//! supports iana timezones and fixed utc offsets

use chrono::{
    DateTime, FixedOffset, Local, LocalResult, NaiveDateTime, Offset, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;

use crate::config::City;

#[derive(Debug, Clone)]
pub(crate) enum ParsedTimezone {
    Iana(Tz),
    Fixed(FixedOffset),
}

impl ParsedTimezone {
    fn current_datetime(&self) -> DateTime<FixedOffset> {
        let utc_now = Utc::now();
        match self {
            Self::Iana(timezone) => utc_now.with_timezone(timezone).fixed_offset(),
            Self::Fixed(offset) => utc_now.with_timezone(offset),
        }
    }

    fn from_local_datetime(
        &self,
        naive_local: &NaiveDateTime,
    ) -> LocalResult<DateTime<FixedOffset>> {
        match self {
            Self::Iana(timezone) => match timezone.from_local_datetime(naive_local) {
                LocalResult::Single(datetime) => LocalResult::Single(datetime.fixed_offset()),
                LocalResult::Ambiguous(first, second) => {
                    LocalResult::Ambiguous(first.fixed_offset(), second.fixed_offset())
                }
                LocalResult::None => LocalResult::None,
            },
            Self::Fixed(offset) => offset.from_local_datetime(naive_local),
        }
    }

    fn convert_datetime(&self, datetime: &DateTime<FixedOffset>) -> DateTime<FixedOffset> {
        match self {
            Self::Iana(timezone) => datetime.with_timezone(timezone).fixed_offset(),
            Self::Fixed(offset) => datetime.with_timezone(offset),
        }
    }
}

pub(crate) fn parse_city_timezone(value: &str) -> Option<ParsedTimezone> {
    let value = value.trim();
    value
        .parse::<Tz>()
        .map(ParsedTimezone::Iana)
        .ok()
        .or_else(|| parse_fixed_utc_offset(value).map(ParsedTimezone::Fixed))
}

fn parse_fixed_utc_offset(value: &str) -> Option<FixedOffset> {
    if value == "UTC" {
        return FixedOffset::east_opt(0);
    }

    let suffix = value.strip_prefix("UTC")?;
    let (sign, remainder) = match suffix.chars().next()? {
        '+' => (1, &suffix[1..]),
        '-' => (-1, &suffix[1..]),
        _ => return None,
    };

    let (hours, minutes) = remainder.split_once(':')?;
    let hours: i32 = hours.parse().ok()?;
    let minutes: i32 = minutes.parse().ok()?;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }

    let total_seconds = sign * (hours * 3600 + minutes * 60);
    FixedOffset::east_opt(total_seconds)
}

/// time information for a city
#[derive(Debug, Clone)]
pub struct CityTime {
    pub city_name: String,
    pub city_code: String,
    timezone: ParsedTimezone,
    pub datetime: DateTime<FixedOffset>,
    pub offset_hours: f32,
}

impl CityTime {
    /// create a new city time from a city configuration
    pub fn from_city(city: &City) -> Option<Self> {
        let timezone = parse_city_timezone(&city.timezone)?;
        let datetime = timezone.current_datetime();

        // calculate offset in hours using the fixed offset
        let fixed: FixedOffset = datetime.offset().fix();
        let offset_secs = fixed.local_minus_utc();
        let offset_hours = offset_secs as f32 / 3600.0;

        Some(Self {
            city_name: city.name.clone(),
            city_code: city.code.clone(),
            timezone,
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

        let from_date = from_city.datetime.date_naive();
        let naive_local = from_date.and_hms_opt(hour, minute, 0)?;

        let from_datetime = match from_city.timezone.from_local_datetime(&naive_local) {
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

        let target = to_city.timezone.convert_datetime(&from_datetime);
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
    pub invalid_input: bool,
    /// buffer for direct time input (e.g. "1430" for 14:30)
    pub input_buffer: String,
}

impl Default for TimeConverter {
    fn default() -> Self {
        let now = Local::now();
        Self {
            from_city_code: "WLG".to_string(),
            to_city_code: "BOS".to_string(),
            input_hour: now.hour(),
            input_minute: now.minute(),
            result_hour: 0,
            result_minute: 0,
            day_offset: 0,
            invalid_input: false,
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
        self.invalid_input = false;
    }

    pub fn swap_cities(&mut self) {
        std::mem::swap(&mut self.from_city_code, &mut self.to_city_code);
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
        if self.invalid_input {
            return "invalid local time".to_string();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_city(code: &str, name: &str, timezone: &str) -> City {
        City {
            name: name.to_string(),
            code: code.to_string(),
            country: "Test".to_string(),
            timezone: timezone.to_string(),
            currency: "TST".to_string(),
        }
    }

    #[test]
    fn city_time_supports_fixed_utc_offsets() {
        let city = test_city("KOR", "Seoul", "UTC+09:00");

        let city_time = CityTime::from_city(&city).expect("fixed offset should parse");

        assert_eq!(city_time.datetime.offset().local_minus_utc(), 9 * 3600);
        assert_eq!(city_time.offset_hours, 9.0);
    }

    #[test]
    fn timezone_service_converts_fixed_offset_cities() {
        let seoul = test_city("KOR", "Seoul", "UTC+09:00");
        let london = test_city("UTC", "UTC", "UTC");
        let mut service = TimezoneService::new();
        service.update(&[&seoul, &london]);

        let converted = service.convert_time("KOR", "UTC", 9, 30);

        assert_eq!(converted, Some((0, 30, 0)));
    }
}
