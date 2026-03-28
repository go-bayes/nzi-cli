//! application state and logic for nzi-cli

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use chrono::Timelike;

use crate::config::{City, Config, CurrencyConfig, MapConfig, MapMode};
use crate::exchange::{CurrencyConverter, ExchangeService};
use crate::map::NZ_CITIES;
use crate::reference::{lookup_country, lookup_currency};
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
    pub map_context: Focus,

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
    pub world_city_times: Vec<CityTime>, // tracked world cities

    // cached weather - now supports multiple cities
    pub current_weather: Option<CurrentWeather>,
    pub weather_city_index: usize, // index into NZ_CITIES for weather display
    pub weather_error: Option<String>, // last weather fetch error
    pub weather_refresh_pending: bool, // flag to request weather refresh
    pub weather_expanded: bool,    // toggle between compact and expanded grid view

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandAction {
    ShowHelp,
    EditConfig,
    Quit,
    Reload,
    Refresh,
    SetFocalCountry { code: String, name: String },
    SetCurrencyPair { from_code: String, to_code: String },
    SetCurrencySync { enabled: bool },
    PinCurrency { code: String },
    SetMapMode { mode: MapMode },
}

fn parse_command(input: &str) -> std::result::Result<CommandAction, String> {
    let trimmed = input.trim();
    let lowered = trimmed.to_lowercase();

    match lowered.as_str() {
        "/help" | "/h" => return Ok(CommandAction::ShowHelp),
        "/edit" | "/e" => return Ok(CommandAction::EditConfig),
        "/quit" | "/q" => return Ok(CommandAction::Quit),
        "/reload" | "/r" => return Ok(CommandAction::Reload),
        "/refresh" => return Ok(CommandAction::Refresh),
        _ => {}
    }

    if let Some(rest) = trimmed.strip_prefix("/country ") {
        return resolve_country_command(rest);
    }

    if let Some(rest) = trimmed.strip_prefix("/focus ") {
        return resolve_country_command(rest);
    }

    if let Some(rest) = trimmed.strip_prefix("/currency ") {
        return resolve_currency_command(rest);
    }

    if let Some(rest) = trimmed.strip_prefix("/map ") {
        return resolve_map_command(rest);
    }

    Err(format!("unknown command: {}", trimmed))
}

fn resolve_country_command(query: &str) -> std::result::Result<CommandAction, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("usage: /country <country>".to_string());
    }

    let country = lookup_country(query).ok_or_else(|| format!("country not found: {}", query))?;

    Ok(CommandAction::SetFocalCountry {
        code: country.code.to_string(),
        name: country.name.to_string(),
    })
}

fn resolve_currency_command(query: &str) -> std::result::Result<CommandAction, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("usage: /currency <from> -> <to>".to_string());
    }

    let lowered = query.to_lowercase();
    if let Some(sync_value) = lowered.strip_prefix("sync ") {
        let enabled = match sync_value.trim() {
            "on" | "true" => true,
            "off" | "false" => false,
            other => return Err(format!("invalid sync value: {}", other)),
        };
        return Ok(CommandAction::SetCurrencySync { enabled });
    }

    if let Some(pin_query) = query.strip_prefix("pin ") {
        let pin_query = pin_query.trim();
        if pin_query.is_empty() {
            return Err("usage: /currency pin <currency>".to_string());
        }
        let currency = lookup_currency(pin_query)
            .ok_or_else(|| format!("currency not found: {}", pin_query))?;
        return Ok(CommandAction::PinCurrency {
            code: currency.code.to_string(),
        });
    }

    let (from_query, to_query) = if let Some((from, to)) = query.split_once("->") {
        (from.trim(), to.trim())
    } else {
        let mut parts = query.split_whitespace();
        let from = parts
            .next()
            .ok_or_else(|| "usage: /currency <from> <to>".to_string())?;
        let to = parts
            .next()
            .ok_or_else(|| "usage: /currency <from> <to>".to_string())?;
        if parts.next().is_some() {
            return Err("use /currency <from> -> <to> for names with spaces".to_string());
        }
        (from, to)
    };

    if from_query.is_empty() || to_query.is_empty() {
        return Err("usage: /currency <from> -> <to>".to_string());
    }

    let from_currency =
        lookup_currency(from_query).ok_or_else(|| format!("currency not found: {}", from_query))?;
    let to_currency =
        lookup_currency(to_query).ok_or_else(|| format!("currency not found: {}", to_query))?;

    Ok(CommandAction::SetCurrencyPair {
        from_code: from_currency.code.to_string(),
        to_code: to_currency.code.to_string(),
    })
}

fn resolve_map_command(query: &str) -> std::result::Result<CommandAction, String> {
    let mode = match query.trim().to_lowercase().as_str() {
        "route" => MapMode::Route,
        "cities" => MapMode::Cities,
        "countries" => MapMode::Countries,
        "both" => MapMode::Both,
        "" => return Err("usage: /map <route|cities|countries|both>".to_string()),
        other => return Err(format!("unknown map mode: {}", other)),
    };

    Ok(CommandAction::SetMapMode { mode })
}

fn apply_command_action_to_config(
    config: &mut Config,
    action: &CommandAction,
) -> std::result::Result<Option<String>, String> {
    match action {
        CommandAction::SetFocalCountry { code, name } => {
            let map = config.map.get_or_insert_with(MapConfig::default);
            map.focal_country_code = Some(code.clone());
            Ok(Some(format!("Focal country set to {} ({})", name, code)))
        }
        CommandAction::SetCurrencyPair { from_code, to_code } => {
            let currency = config.currency.get_or_insert_with(CurrencyConfig::default);
            currency.default_from = Some(from_code.clone());
            currency.default_to = Some(to_code.clone());
            Ok(Some(format!(
                "Default currency pair set to {} -> {}",
                from_code, to_code
            )))
        }
        CommandAction::SetCurrencySync { enabled } => {
            let currency = config.currency.get_or_insert_with(CurrencyConfig::default);
            currency.sync_with_cities = *enabled;
            Ok(Some(format!(
                "Currency sync with cities {}",
                if *enabled { "enabled" } else { "disabled" }
            )))
        }
        CommandAction::PinCurrency { code } => {
            let currency = config.currency.get_or_insert_with(CurrencyConfig::default);
            if !currency.pinned_codes.iter().any(|value| value == code) {
                currency.pinned_codes.push(code.clone());
            }
            Ok(Some(format!("Pinned currency {}", code)))
        }
        CommandAction::SetMapMode { mode } => {
            let map = config.map.get_or_insert_with(MapConfig::default);
            map.mode = *mode;
            Ok(Some(format!(
                "Map mode set to {}",
                match mode {
                    MapMode::Route => "route",
                    MapMode::Cities => "cities",
                    MapMode::Countries => "countries",
                    MapMode::Both => "both",
                }
            )))
        }
        CommandAction::ShowHelp
        | CommandAction::EditConfig
        | CommandAction::Quit
        | CommandAction::Reload
        | CommandAction::Refresh => Ok(None),
    }
}

impl App {
    pub fn new(config: Config) -> Self {
        let tick_rate = Duration::from_millis(config.display.animation_speed_ms);

        // initialise converters with config values
        let currency_pairs = config.effective_currency_pairs();
        let (from_currency, to_currency) = config.effective_default_currency_pair();
        let currency_converter =
            CurrencyConverter::new_with_pairs(&from_currency, &to_currency, currency_pairs);

        let time_converter = TimeConverter::new(&config.current_city.code, &config.home_city.code);

        // start on Wellington for weather by default
        let wellington_index = NZ_CITIES.iter().position(|c| c.code == "WLG").unwrap_or(0);

        Self {
            config,
            running: true,
            focus: Focus::Map,
            map_context: Focus::Weather,
            exchange_service: ExchangeService::new(),
            timezone_service: TimezoneService::new(),
            weather_service: WeatherService::new(),
            currency_converter,
            time_converter,
            current_city_time: None,
            home_city_time: None,
            world_city_times: Vec::new(),
            current_weather: None,
            weather_city_index: wellington_index,
            weather_error: None,
            weather_refresh_pending: true, // fetch on startup
            weather_expanded: true,        // start expanded grid
            animation_frame: 0,
            last_tick: Instant::now(),
            tick_rate,
            status_message: None,
            input_mode: InputMode::Normal,
            is_online: false, // assume offline until proven otherwise
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
        if let Some((_, timestamp)) = &self.status_message
            && timestamp.elapsed() > Duration::from_secs(5)
        {
            self.status_message = None;
        }
    }

    /// update all city times
    fn update_times(&mut self) {
        // update current city time
        self.current_city_time = CityTime::from_city(&self.config.current_city);

        // update home city time
        self.home_city_time = CityTime::from_city(&self.config.home_city);

        // update world city times (tracked cities)
        self.world_city_times = self
            .config
            .tracked_cities
            .iter()
            .filter_map(CityTime::from_city)
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
        } else {
            self.time_converter.invalid_input = true;
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
                self.set_status(format!("Rate: 1 {} = {:.4} {}", from, rate, to));
            }
            Err(e) => {
                self.is_online = false;
                self.currency_converter.needs_refresh = true;
                self.set_status(e.to_string());
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

    pub fn city_by_code(&self, code: &str) -> Option<&City> {
        self.config
            .all_cities()
            .into_iter()
            .find(|city| city.code.eq_ignore_ascii_case(code))
    }

    fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        if focus != Focus::Map {
            self.map_context = focus;
        }
    }

    /// set a status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some((message, Instant::now()));
    }

    /// handle keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        // if help is showing, Esc closes it
        if self.show_help {
            if matches!(key, KeyCode::Esc) {
                self.show_help = false;
            }
            return;
        }

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
            KeyCode::Up => self.set_focus(self.focus.up()),
            KeyCode::Down => self.set_focus(self.focus.down()),
            KeyCode::Left => self.set_focus(self.focus.left()),
            KeyCode::Right => self.set_focus(self.focus.right()),
            KeyCode::Tab => self.set_focus(self.focus.next()),
            KeyCode::BackTab => self.set_focus(self.focus.prev()),

            KeyCode::Enter => self.enter_edit_mode(),
            KeyCode::Char('e') => self.enter_edit_mode(),

            // hjkl for panel navigation (vim-style, same as arrows)
            KeyCode::Char('h') => self.set_focus(self.focus.left()),
            KeyCode::Char('l') => self.set_focus(self.focus.right()),
            KeyCode::Char('j') => self.set_focus(self.focus.down()),
            KeyCode::Char('k') => self.set_focus(self.focus.up()),

            // swap/toggle shortcut
            KeyCode::Char('s') => self.handle_swap(),

            // now shortcut for time converter
            KeyCode::Char('n') if self.focus == Focus::TimeConvert => {
                self.time_converter.set_to_now();
                self.update_time_conversion();
            }

            // 'r' - refresh weather or reset time converter
            KeyCode::Char('r') => match self.focus {
                Focus::Weather => {
                    self.weather_refresh_pending = true;
                    self.set_status("Refreshing weather...".to_string());
                }
                Focus::TimeConvert => {
                    self.time_converter.reset();
                    self.update_time_conversion();
                }
                _ => {}
            },

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
            KeyCode::Backspace
                if self.focus == Focus::TimeConvert && self.time_converter.is_typing() =>
            {
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

            // '?' toggles help overlay
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }

            // 'R' (shift+r) reloads config from disk
            KeyCode::Char('R') => {
                if let Err(e) = self.reload_config() {
                    self.set_status(format!("Failed to reload config: {}", e));
                }
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
        let raw_command = self.command_buffer.trim();

        let action = match parse_command(raw_command) {
            Ok(action) => action,
            Err(message) => {
                self.set_status(message);
                return;
            }
        };

        match action {
            CommandAction::ShowHelp => {
                self.show_help = true;
            }
            CommandAction::EditConfig => {
                self.edit_config_requested = true;
            }
            CommandAction::Quit => {
                self.running = false;
            }
            CommandAction::Reload => {
                if let Err(e) = self.reload_config() {
                    self.set_status(format!("Failed to reload config: {}", e));
                }
            }
            CommandAction::Refresh => {
                self.weather_refresh_pending = true;
                self.set_status("Refreshing...".to_string());
            }
            other => {
                if let Err(err) = self.apply_config_command(other) {
                    self.set_status(err.to_string());
                }
            }
        }
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
            Focus::Weather => {
                self.weather_expanded = !self.weather_expanded;
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
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                if let Some(ct) = &self.current_city_time {
                    self.time_converter.input_hour = ct.datetime.hour();
                    self.time_converter.input_minute = ct.datetime.minute();
                } else {
                    self.time_converter.set_to_now();
                }
                self.time_converter.clear_input_buffer();
                self.update_time_conversion();
            }
            KeyCode::Enter => {
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
                if self.input_mode == InputMode::EditingCurrency {
                    self.input_mode = InputMode::Normal;
                    self.currency_converter.editing = false;
                } else {
                    self.input_mode = InputMode::EditingCurrency;
                    self.currency_converter.editing = true;
                }
            }
            Focus::TimeConvert => {
                if self.input_mode == InputMode::EditingTime {
                    self.input_mode = InputMode::Normal;
                } else {
                    self.input_mode = InputMode::EditingTime;
                }
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

    /// reload config from disk and refresh dependent state
    pub fn reload_config(&mut self) -> Result<()> {
        self.config = Config::load()?;
        self.sync_runtime_to_config();

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

    pub fn active_map_focus(&self) -> Focus {
        if self.focus == Focus::Map {
            Focus::Map
        } else {
            self.focus
        }
    }

    fn apply_config_command(&mut self, action: CommandAction) -> Result<()> {
        let status = apply_command_action_to_config(&mut self.config, &action)
            .map_err(|message| anyhow!(message))?;
        self.config.save()?;
        self.sync_runtime_to_config();

        if let Some(status) = status {
            self.set_status(status);
        }

        Ok(())
    }

    fn sync_runtime_to_config(&mut self) {
        let currency_pairs = self.config.effective_currency_pairs();
        let (from_currency, to_currency) = self.config.effective_default_currency_pair();
        self.currency_converter =
            CurrencyConverter::new_with_pairs(&from_currency, &to_currency, currency_pairs);
        self.time_converter =
            TimeConverter::new(&self.config.current_city.code, &self.config.home_city.code);

        self.weather_city_index = NZ_CITIES
            .iter()
            .position(|c| c.code == self.config.current_city.code)
            .unwrap_or(0);
        self.current_weather = None;
        self.weather_error = None;
        self.weather_expanded = true;
        self.weather_refresh_pending = true;

        self.update_times();
        self.update_time_conversion();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_country_alias_command() {
        let action = parse_command("/country uk").expect("command should parse");

        assert_eq!(
            action,
            CommandAction::SetFocalCountry {
                code: "GBR".to_string(),
                name: "United Kingdom".to_string(),
            }
        );
    }

    #[test]
    fn parses_currency_pair_with_arrow_syntax() {
        let action =
            parse_command("/currency new zealand dollar -> yen").expect("command should parse");

        assert_eq!(
            action,
            CommandAction::SetCurrencyPair {
                from_code: "NZD".to_string(),
                to_code: "JPY".to_string(),
            }
        );
    }

    #[test]
    fn applies_currency_pin_command_to_config() {
        let mut config = Config::default();
        let action = parse_command("/currency pin cad").expect("command should parse");

        let status = apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        assert_eq!(status.as_deref(), Some("Pinned currency CAD"));
        assert_eq!(
            config
                .currency
                .as_ref()
                .map(|currency| currency.pinned_codes.clone()),
            Some(vec!["CAD".to_string()])
        );
    }

    #[test]
    fn applies_map_mode_command_to_config() {
        let mut config = Config::default();
        let action = parse_command("/map countries").expect("command should parse");

        apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        assert_eq!(
            config.map.as_ref().map(|map| map.mode),
            Some(MapMode::Countries)
        );
    }

    #[test]
    fn map_focus_uses_configured_map_when_panel_is_focused() {
        let mut app = App::new(Config::default());
        app.focus = Focus::Map;
        app.map_context = Focus::Weather;

        assert_eq!(app.active_map_focus(), Focus::Map);
    }
}
