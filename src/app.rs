//! application state and logic for nzi-cli

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::{City, Config};
use crate::exchange::{CurrencyConverter, ExchangeService};
use crate::map::NZ_CITIES;
use crate::timezone::{CityTime, TimeConverter, TimezoneService};
use crate::weather::{CurrentWeather, WeatherService};

/// which panel is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Map,
    Weather,
    TimeConvert,
    Currency,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Focus::Map => Focus::Weather,
            Focus::Weather => Focus::TimeConvert,
            Focus::TimeConvert => Focus::Currency,
            Focus::Currency => Focus::Map,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Focus::Map => Focus::Currency,
            Focus::Weather => Focus::Map,
            Focus::TimeConvert => Focus::Weather,
            Focus::Currency => Focus::TimeConvert,
        }
    }

    /// move focus up in the layout
    /// layout: Map(left) | Weather(top-right) / Time+Currency(bottom-right)
    pub fn up(self) -> Self {
        match self {
            Focus::TimeConvert | Focus::Currency => Focus::Weather,
            _ => self,
        }
    }

    /// move focus down in the layout
    pub fn down(self) -> Self {
        match self {
            Focus::Weather => Focus::TimeConvert,
            Focus::Map => Focus::TimeConvert,
            _ => self,
        }
    }

    /// move focus left in the layout
    pub fn left(self) -> Self {
        match self {
            Focus::Weather | Focus::TimeConvert => Focus::Map,
            Focus::Currency => Focus::TimeConvert,
            _ => self,
        }
    }

    /// move focus right in the layout
    pub fn right(self) -> Self {
        match self {
            Focus::Map => Focus::Weather,
            Focus::TimeConvert => Focus::Currency,
            _ => self,
        }
    }
}

/// main application state
pub struct App {
    pub config: Config,
    pub running: bool,
    pub focus: Focus,

    // services
    pub exchange_service: ExchangeService,
    pub timezone_service: TimezoneService,
    pub weather_service: WeatherService,

    // widget states
    pub currency_converter: CurrencyConverter,
    pub time_converter: TimeConverter,

    // cached city times
    pub current_city_time: Option<CityTime>,
    pub home_city_time: Option<CityTime>,
    pub world_city_times: Vec<CityTime>,  // tracked world cities

    // cached weather - now supports multiple cities
    pub current_weather: Option<CurrentWeather>,
    pub weather_city_index: usize,  // index into NZ_CITIES for weather display
    pub weather_error: Option<String>,  // last weather fetch error
    pub weather_refresh_pending: bool,  // flag to request weather refresh
    pub weather_expanded: bool,  // toggle between compact and expanded grid view

    // animation state
    pub animation_frame: usize,
    pub last_tick: Instant,
    pub tick_rate: Duration,

    // status message
    pub status_message: Option<(String, Instant)>,

    // input mode
    pub input_mode: InputMode,

    // data source status
    pub is_online: bool,

    // help overlay
    pub show_help: bool,

    // request to open config in editor
    pub edit_config_requested: bool,

    // command input buffer (for /help, /edit, etc.)
    pub command_buffer: String,
}

/// input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingCurrency,
    EditingTime,
}

impl App {
    pub fn new(config: Config) -> Self {
        let tick_rate = Duration::from_millis(config.display.animation_speed_ms);

        // initialise converters with config values
        let currency_converter = CurrencyConverter::new(
            &config.current_city.currency,
            &config.home_city.currency,
        );

        let time_converter = TimeConverter::new(
            &config.current_city.code,
            &config.home_city.code,
        );

        Self {
            config,
            running: true,
            focus: Focus::Map,
            exchange_service: ExchangeService::new(),
            timezone_service: TimezoneService::new(),
            weather_service: WeatherService::new(),
            currency_converter,
            time_converter,
            current_city_time: None,
            home_city_time: None,
            world_city_times: Vec::new(),
            current_weather: None,
            weather_city_index: 0,
            weather_error: None,
            weather_refresh_pending: true,  // fetch on startup
            weather_expanded: false,  // start compact
            animation_frame: 0,
            last_tick: Instant::now(),
            tick_rate,
            status_message: None,
            input_mode: InputMode::Normal,
            is_online: false,  // assume offline until proven otherwise
            show_help: false,
            edit_config_requested: false,
            command_buffer: String::new(),
        }
    }

    /// load application with default or saved config
    pub fn load() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self::new(config))
    }

    /// update the application state (called on each tick)
    pub fn tick(&mut self) {
        // update animation frame
        self.animation_frame = self.animation_frame.wrapping_add(1);

        // update times
        self.update_times();

        // update time converter result
        self.update_time_conversion();

        // clear old status messages
        if let Some((_, timestamp)) = &self.status_message {
            if timestamp.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }
    }

    /// update all city times
    fn update_times(&mut self) {
        // update current city time
        self.current_city_time = CityTime::from_city(&self.config.current_city);

        // update home city time
        self.home_city_time = CityTime::from_city(&self.config.home_city);

        // update world city times (tracked cities)
        self.world_city_times = self.config.tracked_cities
            .iter()
            .filter_map(|city| CityTime::from_city(city))
            .collect();

        // update timezone service with all cities
        let cities: Vec<&City> = self.config.all_cities();
        self.timezone_service.update(&cities);
    }

    /// update time conversion result
    fn update_time_conversion(&mut self) {
        if let Some((hour, minute, day_offset)) = self.timezone_service.convert_time(
            &self.time_converter.from_city_code,
            &self.time_converter.to_city_code,
            self.time_converter.input_hour,
            self.time_converter.input_minute,
        ) {
            self.time_converter.update_result(hour, minute, day_offset);
        }
    }

    /// fetch exchange rate asynchronously
    pub async fn refresh_exchange_rate(&mut self) {
        let from = self.currency_converter.from_currency.clone();
        let to = self.currency_converter.to_currency.clone();

        match self.exchange_service.get_rate(&from, &to).await {
            Ok(rate) => {
                self.currency_converter.update_rate(rate);
                self.is_online = true;
                self.set_status(format!(
                    "Rate: 1 {} = {:.4} {}",
                    from, rate, to
                ));
            }
            Err(e) => {
                // may still have fallback rate, but mark not fully online
                self.set_status(format!("Rate error: {}", e));
            }
        }
    }

    /// fetch weather for currently selected NZ city
    pub async fn refresh_weather(&mut self) {
        self.weather_refresh_pending = false; // clear the flag
        let city = &NZ_CITIES[self.weather_city_index];
        let city_name = city.name.to_string();

        // fetch weather for selected city
        match self.weather_service.get_weather(&city_name).await {
            Ok(weather) => {
                self.current_weather = Some(weather);
                self.weather_error = None;
                self.is_online = true;
                self.set_status(format!("Weather updated for {}", city_name));
            }
            Err(e) => {
                self.weather_error = Some(e.to_string());
                self.is_online = false;
                self.set_status(format!("Weather error: {} (offline)", e));
            }
        }
    }

    /// check if weather refresh is needed
    pub fn needs_weather_refresh(&self) -> bool {
        self.weather_refresh_pending
    }

    /// get current weather city name
    pub fn get_weather_city_name(&self) -> &str {
        NZ_CITIES[self.weather_city_index].name
    }

    /// get current weather city code
    pub fn get_weather_city_code(&self) -> &str {
        NZ_CITIES[self.weather_city_index].code
    }

    /// set a status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some((message, Instant::now()));
    }

    /// handle keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) {
        // if typing a command, handle that first
        if !self.command_buffer.is_empty() {
            self.handle_command_input(key);
            return;
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_input(key),
            InputMode::EditingCurrency => self.handle_currency_input(key),
            InputMode::EditingTime => self.handle_time_input(key),
        }
    }

    fn handle_normal_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Char('q') => self.running = false,

            // arrow keys move between panels
            KeyCode::Up => self.focus = self.focus.up(),
            KeyCode::Down => self.focus = self.focus.down(),
            KeyCode::Left => self.focus = self.focus.left(),
            KeyCode::Right => self.focus = self.focus.right(),
            KeyCode::Tab => self.focus = self.focus.next(),
            KeyCode::BackTab => self.focus = self.focus.prev(),

            KeyCode::Enter => self.enter_edit_mode(),

            // hjkl for panel navigation (vim-style, same as arrows)
            KeyCode::Char('h') => self.focus = self.focus.left(),
            KeyCode::Char('l') => self.focus = self.focus.right(),
            KeyCode::Char('j') => self.focus = self.focus.down(),
            KeyCode::Char('k') => self.focus = self.focus.up(),

            // swap shortcut
            KeyCode::Char('s') => self.handle_swap(),

            // now shortcut for time converter
            KeyCode::Char('n') if self.focus == Focus::TimeConvert => {
                self.time_converter.set_to_now();
                self.update_time_conversion();
            }

            // 'r' - refresh weather or reset time converter
            KeyCode::Char('r') => {
                match self.focus {
                    Focus::Weather => {
                        self.weather_refresh_pending = true;
                        self.set_status("Refreshing weather...".to_string());
                    }
                    Focus::TimeConvert => {
                        self.time_converter.reset();
                        self.update_time_conversion();
                    }
                    _ => {}
                }
            }

            // numeric input for currency when focused
            KeyCode::Char(c) if c.is_ascii_digit() && self.focus == Focus::Currency => {
                self.input_mode = InputMode::EditingCurrency;
                self.currency_converter.clear_input();
                self.currency_converter.handle_input(c);
            }

            // numeric input for time converter when focused
            KeyCode::Char(c) if c.is_ascii_digit() && self.focus == Focus::TimeConvert => {
                self.time_converter.handle_digit(c);
                self.update_time_conversion();
            }

            // backspace for time converter when typing
            KeyCode::Backspace if self.focus == Focus::TimeConvert && self.time_converter.is_typing() => {
                self.time_converter.handle_backspace();
                self.update_time_conversion();
            }

            // escape clears time input buffer
            KeyCode::Esc if self.focus == Focus::TimeConvert && self.time_converter.is_typing() => {
                self.time_converter.clear_input_buffer();
            }

            // 'c' cycles currency pair when on currency panel
            KeyCode::Char('c') if self.focus == Focus::Currency => {
                self.currency_converter.cycle_pair();
            }

            // space - context-dependent action
            KeyCode::Char(' ') => {
                match self.focus {
                    Focus::Weather => {
                        // cycle NZ cities
                        self.weather_city_index = (self.weather_city_index + 1) % NZ_CITIES.len();
                        self.current_weather = None;
                        self.weather_error = None;
                        self.weather_refresh_pending = true;
                    }
                    Focus::TimeConvert => {
                        // cycle through destination cities
                        let city_codes = self.config.all_city_codes();
                        self.time_converter.cycle_to_city(&city_codes);
                        self.update_time_conversion();
                    }
                    Focus::Currency => {
                        // cycle currency pair
                        self.currency_converter.cycle_pair();
                    }
                    _ => {}
                }
            }

            // 'e' toggles expanded weather view when on weather panel
            KeyCode::Char('e') if self.focus == Focus::Weather => {
                self.weather_expanded = !self.weather_expanded;
            }

            // '?' toggles help overlay
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }

            // 'R' (shift+r) resets config to defaults
            KeyCode::Char('R') => {
                self.reset_to_defaults();
            }

            // 'E' (shift+e) opens config in editor
            KeyCode::Char('E') => {
                self.edit_config_requested = true;
            }

            // '/' starts command input
            KeyCode::Char('/') => {
                self.command_buffer.push('/');
            }

            _ => {}
        }
    }

    fn handle_command_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => {
                self.command_buffer.clear();
            }
            KeyCode::Enter => {
                self.execute_command();
                self.command_buffer.clear();
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim().to_lowercase();
        match cmd.as_str() {
            "/help" | "/h" => {
                self.show_help = true;
            }
            "/edit" | "/e" => {
                self.edit_config_requested = true;
            }
            "/quit" | "/q" => {
                self.running = false;
            }
            "/reset" | "/r" => {
                self.reset_to_defaults();
            }
            "/refresh" => {
                self.weather_refresh_pending = true;
                self.set_status("Refreshing...".to_string());
            }
            _ => {
                self.set_status(format!("Unknown command: {}", self.command_buffer));
            }
        }
    }

    /// reset config to defaults and save
    pub fn reset_to_defaults(&mut self) {
        self.config = Config::default();
        if let Err(e) = self.config.save() {
            self.set_status(format!("Failed to save defaults: {}", e));
        } else {
            self.set_status("Config reset to defaults".to_string());
        }
        // reset converters to match new config
        self.currency_converter = CurrencyConverter::new(
            &self.config.current_city.currency,
            &self.config.home_city.currency,
        );
        self.time_converter = TimeConverter::new(
            &self.config.current_city.code,
            &self.config.home_city.code,
        );
        // trigger refresh
        self.weather_refresh_pending = true;
    }

    /// check if currency rate refresh is needed
    pub fn needs_currency_refresh(&self) -> bool {
        self.currency_converter.needs_rate_refresh()
    }

    /// handle swap action for focused panel
    fn handle_swap(&mut self) {
        match self.focus {
            Focus::Currency => {
                // swap_currencies already handles rate inversion
                self.currency_converter.swap_currencies();
            }
            Focus::TimeConvert => {
                self.time_converter.swap_cities();
                self.update_time_conversion();
            }
            _ => {}
        }
    }

    fn handle_currency_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc | KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                self.currency_converter.handle_input(c);
            }
            KeyCode::Backspace => {
                self.currency_converter.handle_backspace();
            }
            _ => {}
        }
    }

    fn handle_time_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc | KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.time_converter.increment_hour();
                self.update_time_conversion();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.time_converter.decrement_hour();
                self.update_time_conversion();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.time_converter.increment_minute();
                self.update_time_conversion();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.time_converter.decrement_minute();
                self.update_time_conversion();
            }
            _ => {}
        }
    }

    fn enter_edit_mode(&mut self) {
        match self.focus {
            Focus::Currency => {
                self.input_mode = InputMode::EditingCurrency;
                self.currency_converter.editing = true;
            }
            Focus::TimeConvert => {
                self.input_mode = InputMode::EditingTime;
            }
            _ => {}
        }
    }

    /// check if edit config was requested
    pub fn needs_edit_config(&self) -> bool {
        self.edit_config_requested
    }

    /// clear edit config request
    pub fn clear_edit_request(&mut self) {
        self.edit_config_requested = false;
    }

    /// reload config from file (after editing)
    pub fn reload_config(&mut self) -> Result<()> {
        self.config = Config::load()?;
        // reset converters to match new config
        self.currency_converter = CurrencyConverter::new(
            &self.config.current_city.currency,
            &self.config.home_city.currency,
        );
        self.time_converter = TimeConverter::new(
            &self.config.current_city.code,
            &self.config.home_city.code,
        );
        self.set_status("Config reloaded".to_string());
        Ok(())
    }

    /// get editor command from config
    pub fn get_editor(&self) -> String {
        self.config.display.get_editor()
    }

    /// check if it's time for a tick
    pub fn should_tick(&self) -> bool {
        self.last_tick.elapsed() >= self.tick_rate
    }

    /// reset the tick timer
    pub fn reset_tick(&mut self) {
        self.last_tick = Instant::now();
    }

    /// get the from city name for time conversion
    pub fn get_time_convert_from_name(&self) -> &str {
        if self.time_converter.from_city_code == self.config.current_city.code {
            &self.config.current_city.name
        } else if self.time_converter.from_city_code == self.config.home_city.code {
            &self.config.home_city.name
        } else {
            &self.time_converter.from_city_code
        }
    }

    /// get the to city name for time conversion
    pub fn get_time_convert_to_name(&self) -> &str {
        if self.time_converter.to_city_code == self.config.current_city.code {
            &self.config.current_city.name
        } else if self.time_converter.to_city_code == self.config.home_city.code {
            &self.config.home_city.name
        } else {
            &self.time_converter.to_city_code
        }
    }
}
