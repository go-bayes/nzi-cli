//! application state and logic for nzi-cli

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use chrono::Timelike;

use crate::config::{City, Config, MapConfig, TimeConfig};
use crate::exchange::{CurrencyConverter, ExchangeService};
use crate::map::NZ_CITIES;
use crate::reference::{
    country_by_code, focal_country_code_for_currency, lookup_country, lookup_currency,
    representative_city_by_city_code, search_countries, search_currencies,
    search_representative_cities,
};
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
    pub config_draft: Option<Config>,
    pub config_editor: Option<ConfigEditorState>,
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

    // interactive search picker
    pub picker: Option<PickerState>,
}

/// input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingCurrency,
    EditingTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerState {
    pub query: String,
    pub selected: usize,
    kind: PickerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    Places,
    Actions,
}

impl ConfigTab {
    fn next(self) -> Self {
        match self {
            Self::Places => Self::Actions,
            Self::Actions => Self::Places,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Places => Self::Actions,
            Self::Actions => Self::Places,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Places => "Places",
            Self::Actions => "Actions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEditorState {
    pub tab: ConfigTab,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PickerKind {
    Country,
    MapMode,
    AnchorCity,
    TargetCity,
    PlaceCurrency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerOption {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PickerChoice {
    Country {
        code: String,
        name: String,
    },
    Currency {
        code: String,
        name: String,
    },
    MapEnabled {
        enabled: bool,
        label: String,
    },
    City {
        code: String,
        name: String,
        country: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandAction {
    EnterConfigDraft,
    ShowHelp,
    EditConfig,
    Quit,
    Reload,
    ApplyDraft,
    DiscardDraft,
    ResetDraft,
    RestoreDraft,
    Refresh,
    SetFocalCountry { code: String, name: String },
    AddPlaceCurrency { code: String, name: String },
    SetMapEnabled { enabled: bool },
    OpenCountryPicker,
    OpenPlaceCurrencyPicker,
    OpenMapPicker,
}

fn parse_command(input: &str) -> std::result::Result<CommandAction, String> {
    let trimmed = input.trim();
    let lowered = trimmed.to_lowercase();

    match lowered.as_str() {
        "/config" => return Ok(CommandAction::EnterConfigDraft),
        "/help" | "/h" => return Ok(CommandAction::ShowHelp),
        "/edit" | "/e" => return Ok(CommandAction::EditConfig),
        "/quit" | "/q" => return Ok(CommandAction::Quit),
        "/reload" | "/r" => return Ok(CommandAction::Reload),
        "/apply" => return Ok(CommandAction::ApplyDraft),
        "/discard" => return Ok(CommandAction::DiscardDraft),
        "/reset" => return Ok(CommandAction::ResetDraft),
        "/restore" => return Ok(CommandAction::RestoreDraft),
        "/refresh" => return Ok(CommandAction::Refresh),
        "/country" | "/focus" => return Ok(CommandAction::OpenCountryPicker),
        "/currency" => return Ok(CommandAction::OpenPlaceCurrencyPicker),
        "/map" => return Ok(CommandAction::OpenMapPicker),
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
        return Err("usage: /currency <currency>".to_string());
    }

    if query.contains("->")
        || query.to_lowercase().starts_with("pin ")
        || query.to_lowercase().starts_with("sync ")
    {
        return Err(
            "currency is now place-driven: use /currency <code-or-name> to add a place".to_string(),
        );
    }

    let currency =
        lookup_currency(query).ok_or_else(|| format!("currency not found: {}", query))?;

    Ok(CommandAction::AddPlaceCurrency {
        code: currency.code.to_string(),
        name: currency.name.to_string(),
    })
}

fn resolve_map_command(query: &str) -> std::result::Result<CommandAction, String> {
    match query.trim().to_lowercase().as_str() {
        "on" | "show" => Ok(CommandAction::SetMapEnabled { enabled: true }),
        "off" | "hide" => Ok(CommandAction::SetMapEnabled { enabled: false }),
        "" => Err("usage: /map <on|off>".to_string()),
        other => Err(format!("unknown map option: {}", other)),
    }
}

fn apply_command_action_to_config(
    config: &mut Config,
    action: &CommandAction,
) -> std::result::Result<Option<String>, String> {
    match action {
        CommandAction::SetFocalCountry { code, name } => {
            let city = config
                .representative_city_for_country_code(code)
                .ok_or_else(|| format!("no representative city configured for {}", name))?;
            ensure_city_in_config_catalogue(config, &city);
            let time = config.time.get_or_insert_with(TimeConfig::default);
            time.anchor_city_code = Some(city.code.clone());
            time.city_codes.clear();
            time.target_city_codes
                .retain(|value| !value.eq_ignore_ascii_case(&city.code));
            Ok(Some(format!("{} -> {} set as focal city", name, city.name)))
        }
        CommandAction::AddPlaceCurrency { code, name } => {
            let country_code = focal_country_code_for_currency(code)
                .ok_or_else(|| format!("no focal country configured for {}", name))?;
            let country_name = country_by_code(country_code)
                .map(|country| country.name.to_string())
                .unwrap_or_else(|| country_code.to_string());
            let city = config
                .representative_city_for_currency_code(code)
                .ok_or_else(|| format!("no representative city configured for {}", name))?;
            ensure_city_in_config_catalogue(config, &city);
            let anchor_code = config.effective_anchor_city_code();
            let time = config.time.get_or_insert_with(TimeConfig::default);
            time.anchor_city_code
                .get_or_insert_with(|| config.current_city.code.clone());
            time.city_codes.clear();
            if !time
                .target_city_codes
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&city.code))
                && !city.code.eq_ignore_ascii_case(&anchor_code)
            {
                time.target_city_codes.push(city.code.clone());
            }
            Ok(Some(format!(
                "{} -> {} -> {} added to target cities",
                code, country_name, city.name
            )))
        }
        CommandAction::SetMapEnabled { enabled } => {
            let map = config.map.get_or_insert_with(MapConfig::default);
            map.enabled = *enabled;
            Ok(Some(format!(
                "Map {}",
                if *enabled { "enabled" } else { "disabled" }
            )))
        }
        CommandAction::EnterConfigDraft
        | CommandAction::ShowHelp
        | CommandAction::EditConfig
        | CommandAction::Quit
        | CommandAction::Reload
        | CommandAction::ApplyDraft
        | CommandAction::DiscardDraft
        | CommandAction::ResetDraft
        | CommandAction::RestoreDraft
        | CommandAction::Refresh
        | CommandAction::OpenCountryPicker
        | CommandAction::OpenPlaceCurrencyPicker
        | CommandAction::OpenMapPicker => Ok(None),
    }
}

fn ensure_city_in_config_catalogue(config: &mut Config, city: &City) {
    if config.current_city.code.eq_ignore_ascii_case(&city.code)
        || config.home_city.code.eq_ignore_ascii_case(&city.code)
        || config
            .tracked_cities
            .iter()
            .any(|tracked| tracked.code.eq_ignore_ascii_case(&city.code))
    {
        return;
    }

    config.tracked_cities.push(city.clone());
}

fn reset_places_to_package_defaults(config: &mut Config) {
    let default_config = Config::default();
    config.current_city = default_config.current_city.clone();
    config.home_city = default_config.home_city.clone();
    config.tracked_cities = default_config.tracked_cities.clone();
    config.time = Some(TimeConfig {
        anchor_city_code: Some(default_config.effective_anchor_city_code()),
        target_city_codes: default_config.effective_target_city_codes(),
        city_codes: Vec::new(),
    });
}

impl App {
    pub fn new(config: Config) -> Self {
        let tick_rate = Duration::from_millis(config.display.animation_speed_ms);

        // initialise converters with config values
        let currency_pairs = config.effective_currency_pairs();
        let (from_currency, to_currency) = config.effective_default_currency_pair();
        let currency_converter =
            CurrencyConverter::new_with_pairs(&from_currency, &to_currency, currency_pairs);

        let (from_city_code, to_city_code) = config.effective_default_time_pair();
        let time_converter = TimeConverter::new(&from_city_code, &to_city_code);

        // start on Wellington for weather by default
        let wellington_index = NZ_CITIES.iter().position(|c| c.code == "WLG").unwrap_or(0);

        Self {
            config,
            config_draft: None,
            config_editor: None,
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
            picker: None,
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

        // update target city times
        self.world_city_times = self
            .config
            .effective_target_cities()
            .into_iter()
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
                let error_message = format!("{:#}", e);
                if let Some(cached) = self.weather_service.cached_weather(&city_name) {
                    self.current_weather = Some(cached);
                    self.weather_error = Some(error_message);
                    self.is_online = false;
                    self.set_status(format!(
                        "Weather fetch failed for {}; showing cached data",
                        city_name
                    ));
                    return;
                }

                self.weather_error = Some(error_message);
                self.is_online = false;
                self.set_status(format!("Weather error for {} (offline)", city_name));
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

    fn target_cities(&self) -> Vec<&City> {
        self.config
            .effective_target_city_codes()
            .into_iter()
            .filter_map(|code| self.city_by_code(&code))
            .collect()
    }

    fn anchor_city(&self) -> Option<&City> {
        let anchor_code = self.config.effective_anchor_city_code();
        self.city_by_code(&anchor_code)
    }

    fn current_target_city_code(&self) -> Option<String> {
        let target_cities = self.target_cities();
        if target_cities.is_empty() {
            return None;
        }

        target_cities
            .iter()
            .find(|city| {
                city.code
                    .eq_ignore_ascii_case(&self.time_converter.to_city_code)
            })
            .map(|city| city.code.clone())
            .or_else(|| target_cities.first().map(|city| city.code.clone()))
    }

    fn set_current_target_city(&mut self, city_code: &str) {
        let anchor = self
            .anchor_city()
            .cloned()
            .unwrap_or_else(|| self.config.current_city.clone());
        let Some(target_city) = self.city_by_code(city_code).cloned() else {
            return;
        };

        self.time_converter.from_city_code = anchor.code.clone();
        self.time_converter.to_city_code = target_city.code.clone();
        self.currency_converter
            .set_pair(&anchor.currency, &target_city.currency);
        self.update_time_conversion();
    }

    fn cycle_current_target_city(&mut self) {
        let target_codes: Vec<String> = self
            .target_cities()
            .iter()
            .map(|city| city.code.clone())
            .collect();
        if target_codes.is_empty() {
            return;
        }

        let current_code = self.current_target_city_code();
        let current_index = current_code
            .as_deref()
            .and_then(|code| {
                target_codes
                    .iter()
                    .position(|entry| entry.eq_ignore_ascii_case(code))
            })
            .unwrap_or(0);
        let next_index = (current_index + 1) % target_codes.len();
        self.set_current_target_city(&target_codes[next_index]);
    }

    fn sync_currency_to_time_selection(&mut self) {
        let anchor = self
            .anchor_city()
            .cloned()
            .unwrap_or_else(|| self.config.current_city.clone());
        let Some(target_city) = self
            .city_by_code(&self.time_converter.to_city_code)
            .cloned()
        else {
            return;
        };

        self.currency_converter
            .set_pair(&anchor.currency, &target_city.currency);
    }

    pub fn map_enabled(&self) -> bool {
        self.config.effective_map_settings().enabled
    }

    fn next_visible_focus(&self, focus: Focus) -> Focus {
        if self.map_enabled() {
            return focus.next();
        }

        match focus {
            Focus::Weather => Focus::TimeConvert,
            Focus::TimeConvert => Focus::Currency,
            Focus::Currency | Focus::Map => Focus::Weather,
        }
    }

    fn prev_visible_focus(&self, focus: Focus) -> Focus {
        if self.map_enabled() {
            return focus.prev();
        }

        match focus {
            Focus::Weather | Focus::Map => Focus::Currency,
            Focus::TimeConvert => Focus::Weather,
            Focus::Currency => Focus::TimeConvert,
        }
    }

    fn up_visible_focus(&self, focus: Focus) -> Focus {
        if self.map_enabled() {
            return focus.up();
        }

        match focus {
            Focus::TimeConvert | Focus::Currency | Focus::Map => Focus::Weather,
            Focus::Weather => Focus::Weather,
        }
    }

    fn down_visible_focus(&self, focus: Focus) -> Focus {
        if self.map_enabled() {
            return focus.down();
        }

        match focus {
            Focus::Weather | Focus::Map => Focus::TimeConvert,
            Focus::TimeConvert | Focus::Currency => focus,
        }
    }

    fn left_visible_focus(&self, focus: Focus) -> Focus {
        if self.map_enabled() {
            return focus.left();
        }

        match focus {
            Focus::Currency => Focus::TimeConvert,
            Focus::Weather | Focus::TimeConvert | Focus::Map => focus,
        }
    }

    fn right_visible_focus(&self, focus: Focus) -> Focus {
        if self.map_enabled() {
            return focus.right();
        }

        match focus {
            Focus::TimeConvert => Focus::Currency,
            Focus::Map => Focus::Weather,
            Focus::Weather | Focus::Currency => focus,
        }
    }

    fn set_focus(&mut self, focus: Focus) {
        let focus = if self.map_enabled() || focus != Focus::Map {
            focus
        } else {
            Focus::Weather
        };

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

        if self.picker.is_some() {
            self.handle_picker_input(key);
            return;
        }

        if self.config_editor.is_some() {
            self.handle_config_editor_input(key);
            return;
        }

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

    fn handle_config_editor_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => {
                self.close_config_editor();
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.cycle_config_tab(true);
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.cycle_config_tab(false);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(editor) = &mut self.config_editor {
                    editor.selected = editor.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let row_count = self.config_editor_row_count();
                if row_count == 0 {
                    return;
                }
                if let Some(editor) = &mut self.config_editor {
                    editor.selected = (editor.selected + 1).min(row_count.saturating_sub(1));
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Err(err) = self.activate_config_editor_row() {
                    self.set_status(err.to_string());
                }
            }
            KeyCode::Char('a') => {
                if let Err(err) = self.add_config_editor_item() {
                    self.set_status(err.to_string());
                }
            }
            KeyCode::Char('K') => {
                if let Err(err) = self.move_config_editor_item(-1) {
                    self.set_status(err.to_string());
                }
            }
            KeyCode::Char('J') => {
                if let Err(err) = self.move_config_editor_item(1) {
                    self.set_status(err.to_string());
                }
            }
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('x') => {
                if let Err(err) = self.remove_config_editor_item() {
                    self.set_status(err.to_string());
                }
            }
            _ => {}
        }
    }

    fn handle_normal_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Char('q') => self.running = false,

            // arrow keys move between panels
            KeyCode::Up => self.set_focus(self.up_visible_focus(self.focus)),
            KeyCode::Down => self.set_focus(self.down_visible_focus(self.focus)),
            KeyCode::Left => self.set_focus(self.left_visible_focus(self.focus)),
            KeyCode::Right => self.set_focus(self.right_visible_focus(self.focus)),
            KeyCode::Tab => self.set_focus(self.next_visible_focus(self.focus)),
            KeyCode::BackTab => self.set_focus(self.prev_visible_focus(self.focus)),

            KeyCode::Enter => self.enter_edit_mode(),
            KeyCode::Char('e') => self.enter_edit_mode(),

            // hjkl for panel navigation (vim-style, same as arrows)
            KeyCode::Char('h') => self.set_focus(self.left_visible_focus(self.focus)),
            KeyCode::Char('l') => self.set_focus(self.right_visible_focus(self.focus)),
            KeyCode::Char('j') => self.set_focus(self.down_visible_focus(self.focus)),
            KeyCode::Char('k') => self.set_focus(self.up_visible_focus(self.focus)),

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
                self.cycle_current_target_city();
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
                        self.cycle_current_target_city();
                    }
                    Focus::Currency => {
                        self.cycle_current_target_city();
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

    fn handle_picker_input(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        match key {
            KeyCode::Esc => {
                self.picker = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(picker) = &mut self.picker {
                    picker.selected = picker.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let option_count = self.picker_options().len();
                if option_count == 0 {
                    return;
                }
                if let Some(picker) = &mut self.picker {
                    picker.selected = (picker.selected + 1).min(option_count.saturating_sub(1));
                }
            }
            KeyCode::Enter => {
                if let Err(err) = self.submit_picker_selection() {
                    self.set_status(err.to_string());
                }
            }
            KeyCode::Backspace => {
                if let Some(picker) = &mut self.picker {
                    picker.query.pop();
                    picker.selected = 0;
                }
            }
            KeyCode::Char(c) if !c.is_control() => {
                if let Some(picker) = &mut self.picker {
                    picker.query.push(c);
                    picker.selected = 0;
                }
            }
            _ => {}
        }
    }

    fn submit_picker_selection(&mut self) -> Result<()> {
        let Some(choice) = self.current_picker_choice() else {
            return Ok(());
        };

        let Some(picker) = self.picker.clone() else {
            return Ok(());
        };

        match (picker.kind, choice) {
            (PickerKind::Country, PickerChoice::Country { code, name }) => {
                self.picker = None;
                self.apply_config_command(CommandAction::SetFocalCountry { code, name })
            }
            (PickerKind::PlaceCurrency, PickerChoice::Currency { code, name }) => {
                self.picker = None;
                self.apply_config_command(CommandAction::AddPlaceCurrency { code, name })
            }
            (PickerKind::MapMode, PickerChoice::MapEnabled { enabled, .. }) => {
                self.picker = None;
                if self.config_editor.is_some() {
                    self.apply_config_command(CommandAction::SetMapEnabled { enabled })
                } else {
                    self.apply_immediate_config_command(CommandAction::SetMapEnabled { enabled })
                }
            }
            (PickerKind::AnchorCity, PickerChoice::City { code, .. }) => {
                self.picker = None;
                self.set_anchor_city_in_draft(&code)?;
                self.open_picker(PickerKind::TargetCity);
                self.set_status(
                    "Draft updated: anchor city set. Pick one or more target cities".to_string(),
                );
                Ok(())
            }
            (PickerKind::TargetCity, PickerChoice::City { code, .. }) => {
                self.picker = None;
                self.add_target_city_to_draft(&code)
            }
            _ => Ok(()),
        }
    }

    fn open_picker(&mut self, kind: PickerKind) {
        self.show_help = false;
        self.command_buffer.clear();
        self.picker = Some(PickerState {
            query: String::new(),
            selected: 0,
            kind,
        });
    }

    fn open_config_editor(&mut self) {
        self.begin_config_draft();
        if self.config_editor.is_none() {
            self.config_editor = Some(ConfigEditorState {
                tab: ConfigTab::Places,
                selected: 0,
            });
        }
        self.show_help = false;
        self.command_buffer.clear();
    }

    fn close_config_editor(&mut self) {
        self.config_editor = None;
        self.set_status(
            "Config editor closed. Draft kept open; use /apply or /discard".to_string(),
        );
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
            CommandAction::EnterConfigDraft => {
                self.open_config_editor();
            }
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
                if let Err(e) = self.reload_config_state() {
                    self.set_status(format!("Failed to reload config: {}", e));
                }
            }
            CommandAction::ApplyDraft => {
                if let Err(e) = self.apply_config_draft() {
                    self.set_status(format!("Failed to apply draft: {}", e));
                }
            }
            CommandAction::DiscardDraft => {
                self.discard_config_draft();
            }
            CommandAction::ResetDraft => {
                let editing_config = self.config_editor.is_some();
                self.reset_config_draft();
                if !editing_config && let Err(e) = self.apply_config_draft() {
                    self.set_status(format!("Failed to apply reset draft: {}", e));
                }
            }
            CommandAction::RestoreDraft => {
                if let Err(e) = self.restore_config_draft() {
                    self.set_status(format!("Failed to restore draft: {}", e));
                }
            }
            CommandAction::Refresh => {
                self.weather_refresh_pending = true;
                self.set_status("Refreshing...".to_string());
            }
            CommandAction::OpenCountryPicker => {
                self.open_picker(PickerKind::Country);
            }
            CommandAction::OpenPlaceCurrencyPicker => {
                self.open_picker(PickerKind::PlaceCurrency);
            }
            CommandAction::OpenMapPicker => {
                self.open_picker(PickerKind::MapMode);
            }
            other => {
                let result = if matches!(other, CommandAction::SetMapEnabled { .. })
                    && self.config_editor.is_none()
                {
                    self.apply_immediate_config_command(other)
                } else {
                    self.apply_config_command(other)
                };
                if let Err(err) = result {
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
                self.sync_currency_to_time_selection();
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

    fn reload_config_state(&mut self) -> Result<()> {
        if self.config_draft.is_some() {
            self.config_draft = Some(Config::load()?);
            self.picker = None;
            if let Some(editor) = self.config_editor.as_mut() {
                editor.tab = ConfigTab::Places;
                editor.selected = 0;
            }
            self.clamp_config_editor_selection();
            self.set_status("Saved preferences reloaded from config.toml".to_string());
            Ok(())
        } else {
            self.reload_config()
        }
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
        self.city_by_code(&self.time_converter.from_city_code)
            .map(|city| city.name.as_str())
            .unwrap_or(&self.time_converter.from_city_code)
    }

    /// get the to city name for time conversion
    pub fn get_time_convert_to_name(&self) -> &str {
        self.city_by_code(&self.time_converter.to_city_code)
            .map(|city| city.name.as_str())
            .unwrap_or(&self.time_converter.to_city_code)
    }

    pub fn active_map_focus(&self) -> Focus {
        if self.focus == Focus::Map {
            Focus::Map
        } else {
            self.focus
        }
    }

    pub fn has_config_draft(&self) -> bool {
        self.config_draft.is_some()
    }

    pub fn picker_title(&self) -> Option<String> {
        let picker = self.picker.as_ref()?;
        let title = match &picker.kind {
            PickerKind::Country => "Set focal city via country".to_string(),
            PickerKind::PlaceCurrency => "Add currency via country".to_string(),
            PickerKind::MapMode => "Map visibility".to_string(),
            PickerKind::AnchorCity => "Pick anchor city".to_string(),
            PickerKind::TargetCity => "Add target city".to_string(),
        };
        Some(title)
    }

    pub fn picker_prompt(&self) -> Option<&'static str> {
        let picker = self.picker.as_ref()?;
        let prompt = match picker.kind {
            PickerKind::Country => "Pick a country and resolve to its representative focal city",
            PickerKind::PlaceCurrency => {
                "Pick a currency and resolve it through country to a representative city"
            }
            PickerKind::MapMode => "Choose whether the map is shown",
            PickerKind::AnchorCity => "Search by city code, name, or country",
            PickerKind::TargetCity => "Search by city code, name, or country",
        };
        Some(prompt)
    }

    pub fn picker_options(&self) -> Vec<PickerOption> {
        self.picker_choices()
            .into_iter()
            .map(|choice| match choice {
                PickerChoice::Country { code, name } => PickerOption {
                    label: name,
                    detail: code,
                },
                PickerChoice::Currency { code, name } => PickerOption {
                    label: name,
                    detail: code,
                },
                PickerChoice::MapEnabled { enabled: _, label } => PickerOption {
                    detail: "map visibility".to_string(),
                    label,
                },
                PickerChoice::City {
                    code,
                    name,
                    country,
                } => PickerOption {
                    label: format!("{} ({})", name, code),
                    detail: country,
                },
            })
            .collect()
    }

    fn current_picker_choice(&self) -> Option<PickerChoice> {
        let picker = self.picker.as_ref()?;
        let choices = self.picker_choices();
        let index = picker.selected.min(choices.len().saturating_sub(1));
        choices.get(index).cloned()
    }

    fn picker_choices(&self) -> Vec<PickerChoice> {
        let Some(picker) = self.picker.as_ref() else {
            return Vec::new();
        };

        match &picker.kind {
            PickerKind::Country => search_countries(&picker.query)
                .into_iter()
                .map(|country| PickerChoice::Country {
                    code: country.code.to_string(),
                    name: country.name.to_string(),
                })
                .collect(),
            PickerKind::PlaceCurrency => search_currencies(&picker.query)
                .into_iter()
                .map(|currency| PickerChoice::Currency {
                    code: currency.code.to_string(),
                    name: currency.name.to_string(),
                })
                .collect(),
            PickerKind::MapMode => {
                let query = picker.query.trim().to_lowercase();
                vec![
                    PickerChoice::MapEnabled {
                        enabled: true,
                        label: "Show map".to_string(),
                    },
                    PickerChoice::MapEnabled {
                        enabled: false,
                        label: "Hide map".to_string(),
                    },
                ]
                .into_iter()
                .filter(|choice| match choice {
                    PickerChoice::MapEnabled { label, .. } => {
                        query.is_empty() || label.to_lowercase().contains(&query)
                    }
                    _ => false,
                })
                .collect()
            }
            PickerKind::AnchorCity => self.search_config_cities(&picker.query),
            PickerKind::TargetCity => self.search_config_cities(&picker.query),
        }
    }

    fn search_config_cities(&self, query: &str) -> Vec<PickerChoice> {
        let trimmed = query.trim().to_lowercase();
        let mut seen_codes = std::collections::HashSet::new();
        let mut choices = Vec::new();

        let mut config_cities: Vec<&City> = self.active_config().representative_cities();
        config_cities.sort_by(|left, right| left.name.cmp(&right.name));
        for city in config_cities {
            let matches_query = trimmed.is_empty()
                || city.code.to_lowercase().contains(&trimmed)
                || city.name.to_lowercase().contains(&trimmed)
                || city.country.to_lowercase().contains(&trimmed)
                || city.currency.to_lowercase().contains(&trimmed);
            if matches_query && seen_codes.insert(city.code.clone()) {
                choices.push(PickerChoice::City {
                    code: city.code.clone(),
                    name: city.name.clone(),
                    country: city.country.clone(),
                });
            }
        }

        for city in search_representative_cities(query) {
            if seen_codes.insert(city.city_code.to_string()) {
                choices.push(PickerChoice::City {
                    code: city.city_code.to_string(),
                    name: city.city_name.to_string(),
                    country: city.country_name.to_string(),
                });
            }
        }

        choices
    }

    fn active_config(&self) -> &Config {
        self.config_draft.as_ref().unwrap_or(&self.config)
    }

    fn active_config_mut(&mut self) -> &mut Config {
        if let Some(draft) = self.config_draft.as_mut() {
            draft
        } else {
            &mut self.config
        }
    }

    pub fn config_editor_state(&self) -> Option<&ConfigEditorState> {
        self.config_editor.as_ref()
    }

    pub fn config_editor_config(&self) -> Option<&Config> {
        self.config_editor.as_ref()?;
        Some(self.active_config())
    }

    fn cycle_config_tab(&mut self, forward: bool) {
        if let Some(editor) = &mut self.config_editor {
            editor.tab = if forward {
                editor.tab.next()
            } else {
                editor.tab.prev()
            };
            editor.selected = 0;
        }
    }

    fn config_editor_row_count(&self) -> usize {
        let Some(editor) = self.config_editor.as_ref() else {
            return 0;
        };

        match editor.tab {
            ConfigTab::Places => 2 + self.active_config().effective_target_city_codes().len(),
            ConfigTab::Actions => 6,
        }
    }

    fn clamp_config_editor_selection(&mut self) {
        let row_count = self.config_editor_row_count();
        if let Some(editor) = &mut self.config_editor {
            editor.selected = if row_count == 0 {
                0
            } else {
                editor.selected.min(row_count - 1)
            };
        }
    }

    fn activate_config_editor_row(&mut self) -> Result<()> {
        let Some(editor) = self.config_editor.as_ref() else {
            return Ok(());
        };

        match editor.tab {
            ConfigTab::Places => self.activate_places_editor_row(editor.selected),
            ConfigTab::Actions => self.activate_actions_editor_row(editor.selected),
        }
    }

    fn activate_places_editor_row(&mut self, selected: usize) -> Result<()> {
        let list_len = self.active_config().effective_target_city_codes().len();
        match selected {
            0 => {
                self.open_picker(PickerKind::AnchorCity);
                Ok(())
            }
            index if index == 1 + list_len => {
                self.open_picker(PickerKind::TargetCity);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn activate_actions_editor_row(&mut self, selected: usize) -> Result<()> {
        match selected {
            0 => self.apply_config_draft(),
            1 => {
                self.discard_config_draft();
                Ok(())
            }
            2 => {
                self.reset_config_draft();
                Ok(())
            }
            3 => self.reload_config_state(),
            4 => self.restore_config_draft(),
            5 => {
                let enabled = !self.active_config().effective_map_settings().enabled;
                self.apply_config_command(CommandAction::SetMapEnabled { enabled })
            }
            _ => Ok(()),
        }
    }

    fn add_config_editor_item(&mut self) -> Result<()> {
        let Some(editor) = self.config_editor.as_ref() else {
            return Ok(());
        };

        match editor.tab {
            ConfigTab::Places => {
                self.open_picker(PickerKind::TargetCity);
                Ok(())
            }
            ConfigTab::Actions => Ok(()),
        }
    }

    fn remove_config_editor_item(&mut self) -> Result<()> {
        let Some(editor) = self.config_editor.as_ref() else {
            return Ok(());
        };

        match editor.tab {
            ConfigTab::Places => {
                if editor.selected == 0 {
                    return Ok(());
                }

                let codes = self.active_config().effective_target_city_codes();
                let index = editor.selected - 1;
                if let Some(code) = codes.get(index) {
                    self.remove_target_city_from_draft(code)?;
                }
                Ok(())
            }
            ConfigTab::Actions => Ok(()),
        }
    }

    fn move_config_editor_item(&mut self, direction: isize) -> Result<()> {
        let Some(editor) = self.config_editor.as_ref() else {
            return Ok(());
        };

        match editor.tab {
            ConfigTab::Places => {
                if editor.selected == 0 {
                    return Ok(());
                }
                let index = editor.selected - 1;
                self.reorder_target_city_in_draft(index, direction)
            }
            ConfigTab::Actions => Ok(()),
        }
    }

    fn set_anchor_city_in_draft(&mut self, code: &str) -> Result<()> {
        let city = self
            .active_config()
            .all_cities()
            .into_iter()
            .find(|city| city.code.eq_ignore_ascii_case(code))
            .cloned()
            .or_else(|| {
                representative_city_by_city_code(code).map(|city| City {
                    name: city.city_name.to_string(),
                    code: city.city_code.to_string(),
                    country: city.country_name.to_string(),
                    timezone: city.timezone.to_string(),
                    currency: city.currency_code.to_string(),
                })
            })
            .ok_or_else(|| anyhow!("unknown city: {}", code))?;
        let city_code = city.code.clone();

        self.ensure_city_in_active_catalogue(&city);

        let target = self.active_config_mut();
        let time = target.time.get_or_insert_with(TimeConfig::default);
        time.anchor_city_code = Some(city_code.clone());
        time.city_codes.clear();
        time.target_city_codes
            .retain(|entry| !entry.eq_ignore_ascii_case(&city_code));
        self.clamp_config_editor_selection();
        self.set_status(format!(
            "Draft updated: anchor city set to {}. Use /apply to save",
            city_code
        ));
        Ok(())
    }

    fn add_target_city_to_draft(&mut self, code: &str) -> Result<()> {
        let city = self
            .active_config()
            .all_cities()
            .into_iter()
            .find(|city| city.code.eq_ignore_ascii_case(code))
            .cloned()
            .or_else(|| {
                representative_city_by_city_code(code).map(|city| City {
                    name: city.city_name.to_string(),
                    code: city.city_code.to_string(),
                    country: city.country_name.to_string(),
                    timezone: city.timezone.to_string(),
                    currency: city.currency_code.to_string(),
                })
            })
            .ok_or_else(|| anyhow!("unknown city: {}", code))?;
        let city_code = city.code.clone();

        let anchor_code = self.active_config().effective_anchor_city_code();
        if city_code.eq_ignore_ascii_case(&anchor_code) {
            self.set_status(format!("{} is already the anchor city", city_code));
            return Ok(());
        }

        self.ensure_city_in_active_catalogue(&city);

        let target = self.active_config_mut();
        let time = target.time.get_or_insert_with(TimeConfig::default);
        time.city_codes.clear();
        if time
            .target_city_codes
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(&city_code))
        {
            self.set_status(format!("{} is already in the target list", city_code));
            return Ok(());
        }
        time.target_city_codes.push(city_code.clone());
        self.clamp_config_editor_selection();
        self.set_status(format!(
            "Draft updated: added {} to target cities. Use /apply to save",
            city_code
        ));
        Ok(())
    }

    fn ensure_city_in_active_catalogue(&mut self, city: &City) {
        let target = self.active_config_mut();
        ensure_city_in_config_catalogue(target, city);
    }

    fn remove_target_city_from_draft(&mut self, code: &str) -> Result<()> {
        let target = self.active_config_mut();
        let time = target.time.get_or_insert_with(TimeConfig::default);
        time.city_codes.clear();
        let original_len = time.target_city_codes.len();
        time.target_city_codes
            .retain(|entry| !entry.eq_ignore_ascii_case(code));
        if time.target_city_codes.len() < original_len {
            self.clamp_config_editor_selection();
            self.set_status(format!(
                "Draft updated: removed {} from target cities. Use /apply to save",
                code
            ));
        }
        Ok(())
    }

    fn reorder_target_city_in_draft(&mut self, index: usize, direction: isize) -> Result<()> {
        let moved_code = {
            let target = self.active_config_mut();
            let time = target.time.get_or_insert_with(TimeConfig::default);
            time.city_codes.clear();

            if index >= time.target_city_codes.len() {
                return Ok(());
            }

            let Some(next_index) = index.checked_add_signed(direction) else {
                return Ok(());
            };
            if next_index >= time.target_city_codes.len() {
                return Ok(());
            }

            time.target_city_codes.swap(index, next_index);
            time.target_city_codes[next_index].clone()
        };

        if let Some(editor) = self.config_editor.as_mut() {
            editor.selected = (index.checked_add_signed(direction).unwrap_or(index)) + 1;
        }

        self.set_status(format!(
            "Draft updated: moved {} in target cities. Use /apply to save",
            moved_code
        ));
        Ok(())
    }

    fn apply_config_command(&mut self, action: CommandAction) -> Result<()> {
        let editing_draft = self.config_draft.is_some();
        let target = if let Some(draft) = self.config_draft.as_mut() {
            draft
        } else {
            &mut self.config
        };

        let status =
            apply_command_action_to_config(target, &action).map_err(|message| anyhow!(message))?;

        if editing_draft {
            if let Some(status) = status {
                self.set_status(format!("Draft updated: {}. Use /apply to save", status));
            }
        } else {
            self.config.save()?;
            self.sync_runtime_to_config();

            if let Some(status) = status {
                self.set_status(status);
            }
        }

        Ok(())
    }

    fn apply_immediate_config_command(&mut self, action: CommandAction) -> Result<()> {
        let status = apply_command_action_to_config(&mut self.config, &action)
            .map_err(|message| anyhow!(message))?;

        if let Some(draft) = self.config_draft.as_mut() {
            apply_command_action_to_config(draft, &action).map_err(|message| anyhow!(message))?;
        }

        self.config.save()?;
        self.sync_runtime_to_config();

        if let Some(status) = status {
            self.set_status(status);
        }

        Ok(())
    }

    fn begin_config_draft(&mut self) {
        if self.config_draft.is_none() {
            self.config_draft = Some(self.config.clone());
        }
        self.clamp_config_editor_selection();
        self.set_status(
            "Config draft opened. Use /apply, /discard, /reset, or /restore".to_string(),
        );
    }

    fn apply_config_draft(&mut self) -> Result<()> {
        let Some(draft) = self.config_draft.take() else {
            self.set_status("No config draft to apply".to_string());
            return Ok(());
        };

        self.config.save_snapshot()?;
        self.config = draft;
        self.config.save()?;
        self.sync_runtime_to_config();
        self.config_editor = None;
        self.set_status("Config draft applied".to_string());
        Ok(())
    }

    fn discard_config_draft(&mut self) {
        if self.config_draft.take().is_some() {
            self.config_editor = None;
            self.set_status("Config draft discarded".to_string());
        } else {
            self.set_status("No config draft to discard".to_string());
        }
    }

    fn reset_config_draft(&mut self) {
        let was_editing = self.config_draft.is_some();
        let mut draft = self
            .config_draft
            .take()
            .unwrap_or_else(|| self.config.clone());
        reset_places_to_package_defaults(&mut draft);
        self.config_draft = Some(draft);
        self.picker = None;
        if let Some(editor) = self.config_editor.as_mut() {
            editor.tab = ConfigTab::Places;
            editor.selected = 0;
        }
        self.clamp_config_editor_selection();
        self.set_status(if was_editing {
            "Places reset in draft. Use Apply to write config.toml".to_string()
        } else {
            "Package default places draft opened. Use Apply to write config.toml".to_string()
        });
    }

    fn restore_config_draft(&mut self) -> Result<()> {
        let restored = Config::load_latest_snapshot()?;
        self.config_draft = Some(restored);
        self.picker = None;
        if let Some(editor) = self.config_editor.as_mut() {
            editor.tab = ConfigTab::Places;
            editor.selected = 0;
        }
        self.clamp_config_editor_selection();
        self.set_status(
            "Latest saved preferences loaded into draft. Use /apply to save".to_string(),
        );
        Ok(())
    }

    fn sync_runtime_to_config(&mut self) {
        let currency_pairs = self.config.effective_currency_pairs();
        let (from_currency, to_currency) = self.config.effective_default_currency_pair();
        self.currency_converter =
            CurrencyConverter::new_with_pairs(&from_currency, &to_currency, currency_pairs);
        let (from_city_code, to_city_code) = self.config.effective_default_time_pair();
        self.time_converter = TimeConverter::new(&from_city_code, &to_city_code);

        self.weather_city_index = NZ_CITIES
            .iter()
            .position(|c| c.code == self.config.current_city.code)
            .unwrap_or(0);
        self.current_weather = None;
        self.weather_error = None;
        self.weather_expanded = true;
        self.weather_refresh_pending = true;
        if !self.map_enabled() && self.focus == Focus::Map {
            self.focus = Focus::Weather;
            self.map_context = Focus::Weather;
        }

        self.update_times();
        self.update_time_conversion();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::with_temp_config_dir_for_test;

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
    fn parses_currency_command_to_place_add() {
        let action = parse_command("/currency yen").expect("command should parse");

        assert_eq!(
            action,
            CommandAction::AddPlaceCurrency {
                code: "JPY".to_string(),
                name: "Japanese yen".to_string(),
            }
        );
    }

    #[test]
    fn parses_bare_country_command_to_picker() {
        let action = parse_command("/country").expect("command should parse");

        assert_eq!(action, CommandAction::OpenCountryPicker);
    }

    #[test]
    fn parses_config_command_to_draft_mode() {
        let action = parse_command("/config").expect("command should parse");

        assert_eq!(action, CommandAction::EnterConfigDraft);
    }

    #[test]
    fn applies_currency_command_to_places_config() {
        let mut config = Config::default();
        let action = parse_command("/currency yen").expect("command should parse");

        let status = apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        assert_eq!(
            status.as_deref(),
            Some("JPY -> Japan -> Tokyo added to target cities")
        );
        assert_eq!(
            config
                .time
                .as_ref()
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["TYO".to_string()])
        );
    }

    #[test]
    fn currency_command_adds_missing_representative_city_to_catalogue() {
        let mut config = Config::default();
        config.tracked_cities.clear();
        let action = parse_command("/currency yen").expect("command should parse");

        apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        assert!(
            config
                .tracked_cities
                .iter()
                .any(|city| city.code.eq_ignore_ascii_case("TYO"))
        );
        assert_eq!(
            config
                .time
                .as_ref()
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["TYO".to_string()])
        );
    }

    #[test]
    fn applies_map_enabled_command_to_config() {
        let mut config = Config::default();
        let action = parse_command("/map on").expect("command should parse");

        apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        assert_eq!(config.map.as_ref().map(|map| map.enabled), Some(true));
    }

    #[test]
    fn map_command_applies_immediately_even_with_saved_draft() {
        with_temp_config_dir_for_test(|| {
            let mut app = App::new(Config::default());
            app.open_config_editor();
            app.close_config_editor();
            app.command_buffer = "/map on".to_string();

            app.execute_command();

            assert_eq!(app.config.map.as_ref().map(|map| map.enabled), Some(true));
            assert_eq!(
                app.config_draft
                    .as_ref()
                    .and_then(|draft| draft.map.as_ref())
                    .map(|map| map.enabled),
                Some(true)
            );

            let saved = Config::load().expect("config should reload");
            assert_eq!(saved.map.as_ref().map(|map| map.enabled), Some(true));
        });
    }

    #[test]
    fn map_picker_can_disable_map() {
        let mut app = App::new(Config::default());
        app.open_picker(PickerKind::MapMode);
        if let Some(picker) = app.picker.as_mut() {
            picker.query = "hide".to_string();
        }

        let choice = app
            .current_picker_choice()
            .expect("picker should return a choice");
        let mut config = Config::default();
        match choice {
            PickerChoice::MapEnabled { enabled, .. } => {
                apply_command_action_to_config(
                    &mut config,
                    &CommandAction::SetMapEnabled { enabled },
                )
                .expect("config mutation should succeed");
            }
            other => panic!("unexpected picker choice: {other:?}"),
        }

        assert_eq!(config.map.as_ref().map(|map| map.enabled), Some(false));
    }

    #[test]
    fn map_focus_uses_configured_map_when_panel_is_focused() {
        let mut app = App::new(Config::default());
        app.focus = Focus::Map;
        app.map_context = Focus::Weather;

        assert_eq!(app.active_map_focus(), Focus::Map);
    }

    #[test]
    fn hidden_map_is_skipped_in_focus_navigation() {
        let mut config = Config::default();
        config.map = Some(MapConfig {
            enabled: false,
            ..MapConfig::default()
        });
        let mut app = App::new(config);
        app.focus = Focus::Currency;

        app.handle_normal_input(crossterm::event::KeyCode::Tab);
        assert_eq!(app.focus, Focus::Weather);

        app.handle_normal_input(crossterm::event::KeyCode::BackTab);
        assert_eq!(app.focus, Focus::Currency);
    }

    #[test]
    fn actions_tab_apply_saves_and_closes_editor() {
        with_temp_config_dir_for_test(|| {
            let mut app = App::new(Config::default());
            app.open_config_editor();
            if let Some(draft) = app.config_draft.as_mut() {
                draft.time = Some(TimeConfig {
                    anchor_city_code: Some("WLG".to_string()),
                    target_city_codes: vec!["TYO".to_string()],
                    city_codes: Vec::new(),
                });
            }
            if let Some(editor) = app.config_editor.as_mut() {
                editor.tab = ConfigTab::Actions;
                editor.selected = 0;
            }

            app.handle_config_editor_input(crossterm::event::KeyCode::Enter);

            assert!(app.config_editor.is_none());
            assert!(app.config_draft.is_none());
            assert_eq!(
                app.config.effective_target_city_codes(),
                vec!["TYO".to_string()]
            );
        });
    }

    #[test]
    fn actions_tab_reset_replaces_draft() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        if let Some(draft) = app.config_draft.as_mut() {
            draft.current_city.code = "AKL".to_string();
            draft.current_city.name = "Auckland".to_string();
            draft.home_city.code = "CPH".to_string();
            draft.home_city.name = "Copenhagen".to_string();
            draft.tracked_cities.clear();
            draft.time = Some(TimeConfig {
                anchor_city_code: Some("WLG".to_string()),
                target_city_codes: vec!["PAR".to_string()],
                city_codes: Vec::new(),
            });
        }
        if let Some(editor) = app.config_editor.as_mut() {
            editor.tab = ConfigTab::Actions;
            editor.selected = 2;
        }

        app.handle_config_editor_input(crossterm::event::KeyCode::Enter);

        assert!(app.config_editor.is_some());
        assert!(app.config_draft.is_some());
        assert_eq!(
            app.config_draft
                .as_ref()
                .map(|draft| draft.effective_anchor_city_code()),
            Some(Config::default().effective_anchor_city_code())
        );
        assert_eq!(
            app.config_draft
                .as_ref()
                .map(|draft| draft.effective_target_city_codes()),
            Some(Config::default().effective_target_city_codes())
        );
    }

    #[test]
    fn actions_tab_reset_preserves_map_visibility() {
        let mut config = Config::default();
        config.map = Some(MapConfig {
            enabled: true,
            ..MapConfig::default()
        });
        let mut app = App::new(config);
        app.open_config_editor();
        if let Some(draft) = app.config_draft.as_mut() {
            draft.time = Some(TimeConfig {
                anchor_city_code: Some("BOS".to_string()),
                target_city_codes: vec!["TYO".to_string()],
                city_codes: Vec::new(),
            });
        }
        if let Some(editor) = app.config_editor.as_mut() {
            editor.tab = ConfigTab::Actions;
            editor.selected = 2;
        }

        app.handle_config_editor_input(crossterm::event::KeyCode::Enter);

        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.map.as_ref())
                .map(|map| map.enabled),
            Some(true)
        );
        assert_eq!(
            app.config_draft
                .as_ref()
                .map(|draft| draft.effective_anchor_city_code()),
            Some(Config::default().effective_anchor_city_code())
        );
    }

    #[test]
    fn actions_tab_reload_restores_saved_preferences_from_disk() {
        with_temp_config_dir_for_test(|| {
            let mut saved = Config::default();
            saved.time = Some(TimeConfig {
                anchor_city_code: Some("WLG".to_string()),
                target_city_codes: vec!["CPH".to_string(), "TYO".to_string()],
                city_codes: Vec::new(),
            });
            saved.tracked_cities.push(City {
                name: "Copenhagen".to_string(),
                code: "CPH".to_string(),
                country: "Denmark".to_string(),
                timezone: "Europe/Copenhagen".to_string(),
                currency: "DKK".to_string(),
            });
            saved.save().expect("saved config should write");

            let mut app = App::new(saved.clone());
            app.open_config_editor();
            if let Some(draft) = app.config_draft.as_mut() {
                draft.time = Some(TimeConfig {
                    anchor_city_code: Some("TYO".to_string()),
                    target_city_codes: vec!["PAR".to_string()],
                    city_codes: Vec::new(),
                });
            }
            if let Some(editor) = app.config_editor.as_mut() {
                editor.tab = ConfigTab::Actions;
                editor.selected = 3;
            }

            app.handle_config_editor_input(crossterm::event::KeyCode::Enter);

            assert_eq!(
                app.config_editor_state().map(|editor| editor.tab),
                Some(ConfigTab::Places)
            );
            assert_eq!(
                app.config_draft
                    .as_ref()
                    .map(|draft| draft.effective_anchor_city_code()),
                Some("WLG".to_string())
            );
            assert_eq!(
                app.config_draft
                    .as_ref()
                    .map(|draft| draft.effective_target_city_codes()),
                Some(vec!["CPH".to_string(), "TYO".to_string()])
            );
        });
    }

    #[test]
    fn actions_tab_can_toggle_map() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        if let Some(editor) = app.config_editor.as_mut() {
            editor.tab = ConfigTab::Actions;
            editor.selected = 5;
        }

        app.handle_config_editor_input(crossterm::event::KeyCode::Enter);

        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.map.as_ref())
                .map(|map| map.enabled),
            Some(true)
        );
    }

    #[test]
    fn cycling_time_keeps_currency_aligned_to_same_target_city() {
        let mut app = App::new(Config::default());
        app.focus = Focus::TimeConvert;

        app.handle_normal_input(crossterm::event::KeyCode::Char(' '));

        assert_eq!(app.time_converter.to_city_code, "LDN");
        assert_eq!(app.currency_converter.to_currency, "GBP");
    }

    #[test]
    fn cycling_currency_keeps_time_aligned_to_same_target_city() {
        let mut app = App::new(Config::default());
        app.focus = Focus::Currency;

        app.handle_normal_input(crossterm::event::KeyCode::Char(' '));

        assert_eq!(app.time_converter.to_city_code, "LDN");
        assert_eq!(app.currency_converter.to_currency, "GBP");
    }

    #[test]
    fn swapping_time_keeps_currency_aligned() {
        let mut app = App::new(Config::default());
        app.focus = Focus::TimeConvert;

        app.handle_normal_input(crossterm::event::KeyCode::Char('s'));

        assert_eq!(app.time_converter.to_city_code, "WLG");
        assert_eq!(app.currency_converter.to_currency, "NZD");
    }

    #[test]
    fn picker_can_apply_country_selection() {
        let mut app = App::new(Config::default());
        app.open_picker(PickerKind::Country);
        if let Some(picker) = &mut app.picker {
            picker.query = "uk".to_string();
        }

        let choice = app
            .current_picker_choice()
            .expect("picker should return a choice");
        let mut config = Config::default();
        let action = match choice {
            PickerChoice::Country { code, name } => CommandAction::SetFocalCountry { code, name },
            other => panic!("unexpected picker choice: {other:?}"),
        };
        apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        assert_eq!(
            config
                .time
                .as_ref()
                .and_then(|time| time.anchor_city_code.as_deref()),
            Some("LDN")
        );
    }

    #[test]
    fn picker_can_apply_place_currency_selection() {
        let mut app = App::new(Config::default());
        app.open_picker(PickerKind::PlaceCurrency);
        if let Some(picker) = &mut app.picker {
            picker.query = "yen".to_string();
        }

        let choice = app
            .current_picker_choice()
            .expect("picker should return a choice");

        let mut config = Config::default();
        let action = match choice {
            PickerChoice::Currency { code, name } => CommandAction::AddPlaceCurrency { code, name },
            other => panic!("unexpected picker choice: {other:?}"),
        };
        apply_command_action_to_config(&mut config, &action)
            .expect("config mutation should succeed");

        let time = config.time.as_ref().expect("time config should exist");
        assert_eq!(time.target_city_codes, vec!["TYO".to_string()]);
    }

    #[test]
    fn config_draft_can_be_opened_reset_and_discarded() {
        let mut app = App::new(Config::default());

        app.begin_config_draft();
        assert!(app.has_config_draft());

        if let Some(draft) = app.config_draft.as_mut() {
            draft.time = Some(TimeConfig {
                anchor_city_code: Some("BOS".to_string()),
                target_city_codes: vec!["TYO".to_string()],
                city_codes: Vec::new(),
            });
        }

        app.reset_config_draft();
        assert_eq!(
            app.config_draft
                .as_ref()
                .map(|draft| draft.effective_anchor_city_code()),
            Some("WLG".to_string())
        );

        app.discard_config_draft();
        assert!(!app.has_config_draft());
    }

    #[test]
    fn config_command_opens_editor_and_draft() {
        let mut app = App::new(Config::default());
        app.command_buffer = "/config".to_string();

        app.execute_command();

        assert!(app.has_config_draft());
        assert_eq!(
            app.config_editor_state().map(|editor| editor.tab),
            Some(ConfigTab::Places)
        );
    }

    #[test]
    fn reset_command_applies_package_defaults_immediately() {
        with_temp_config_dir_for_test(|| {
            let mut config = Config::default();
            config.time = Some(TimeConfig {
                anchor_city_code: Some("TYO".to_string()),
                target_city_codes: vec!["PAR".to_string()],
                city_codes: Vec::new(),
            });
            config.save().expect("config should save");

            let mut app = App::new(config);
            app.command_buffer = "/reset".to_string();
            app.execute_command();

            let reloaded = Config::load().expect("config should reload");
            assert_eq!(
                reloaded.effective_anchor_city_code(),
                Config::default().effective_anchor_city_code()
            );
            assert_eq!(
                reloaded.effective_target_city_codes(),
                Config::default().effective_target_city_codes()
            );
            assert!(app.config_draft.is_none());
        });
    }

    #[test]
    fn config_editor_can_add_and_remove_target_city() {
        let mut app = App::new(Config::default());
        app.open_config_editor();

        if let Some(editor) = app.config_editor.as_mut() {
            editor.tab = ConfigTab::Places;
        }

        app.add_target_city_to_draft("TYO")
            .expect("should add target city");
        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["TYO".to_string()])
        );

        if let Some(editor) = app.config_editor.as_mut() {
            editor.selected = 1;
        }
        app.remove_config_editor_item()
            .expect("should remove selected target city");

        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(Vec::new())
        );
    }

    #[test]
    fn config_editor_can_reorder_target_cities() {
        let mut app = App::new(Config::default());
        app.open_config_editor();

        if let Some(draft) = app.config_draft.as_mut() {
            draft.time = Some(TimeConfig {
                anchor_city_code: Some("WLG".to_string()),
                target_city_codes: vec!["BOS".to_string(), "TYO".to_string()],
                city_codes: Vec::new(),
            });
        }
        if let Some(editor) = app.config_editor.as_mut() {
            editor.tab = ConfigTab::Places;
            editor.selected = 1;
        }

        app.move_config_editor_item(1)
            .expect("should reorder selected target city");

        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["TYO".to_string(), "BOS".to_string()])
        );
        assert_eq!(
            app.config_editor_state().map(|editor| editor.selected),
            Some(2)
        );
    }

    #[test]
    fn target_city_search_resolves_country_name_to_representative_city() {
        let mut app = App::new(Config::default());
        app.open_config_editor();

        let city_code = app
            .search_config_cities("Japan")
            .into_iter()
            .find_map(|choice| match choice {
                PickerChoice::City { code, .. } => Some(code),
                _ => None,
            })
            .expect("country search should resolve to a city");
        app.add_target_city_to_draft(&city_code)
            .expect("target add should resolve");

        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["TYO".to_string()])
        );
    }

    #[test]
    fn target_city_search_can_use_generated_representative_city() {
        let mut config = Config::default();
        config.tracked_cities.clear();
        let mut app = App::new(config);
        app.open_config_editor();

        let city_code = app
            .search_config_cities("France")
            .into_iter()
            .find_map(|choice| match choice {
                PickerChoice::City { code, .. } => Some(code),
                _ => None,
            })
            .expect("country search should resolve to a city");
        app.add_target_city_to_draft(&city_code)
            .expect("target add should resolve");

        assert!(
            app.config_draft
                .as_ref()
                .map(|draft| draft
                    .tracked_cities
                    .iter()
                    .any(|city| city.code.eq_ignore_ascii_case("PAR")))
                .unwrap_or(false)
        );
        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["PAR".to_string()])
        );
    }

    #[test]
    fn places_currency_helper_resolves_to_representative_city() {
        let mut app = App::new(Config::default());
        app.open_config_editor();

        app.apply_config_command(CommandAction::AddPlaceCurrency {
            code: "JPY".to_string(),
            name: "Japanese yen".to_string(),
        })
        .expect("currency helper should resolve");

        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["TYO".to_string()])
        );
    }

    #[test]
    fn city_picker_uses_representative_cities() {
        let mut config = Config::default();
        config.tracked_cities.push(City {
            name: "Honolulu".to_string(),
            code: "HNL".to_string(),
            country: "USA".to_string(),
            timezone: "Pacific/Honolulu".to_string(),
            currency: "USD".to_string(),
        });

        let mut app = App::new(config);
        app.open_picker(PickerKind::TargetCity);

        let choices = app.picker_choices();
        let city_codes: Vec<String> = choices
            .into_iter()
            .filter_map(|choice| match choice {
                PickerChoice::City { code, .. } => Some(code),
                _ => None,
            })
            .collect();

        assert!(city_codes.iter().any(|code| code == "BOS"));
        assert!(city_codes.iter().any(|code| code == "THR"));
        assert!(city_codes.iter().any(|code| code == "HNL"));
    }

    #[test]
    fn target_city_picker_can_add_generated_city_outside_current_catalogue() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        app.add_target_city_to_draft("THR")
            .expect("generated representative city should add");

        assert!(
            app.config_draft
                .as_ref()
                .map(|draft| draft
                    .tracked_cities
                    .iter()
                    .any(|city| city.code.eq_ignore_ascii_case("THR")))
                .unwrap_or(false)
        );
        assert_eq!(
            app.config_draft
                .as_ref()
                .and_then(|draft| draft.time.as_ref())
                .map(|time| time.target_city_codes.clone()),
            Some(vec!["THR".to_string()])
        );
    }

    #[test]
    fn applying_generated_city_with_fixed_offset_timezone_saves_successfully() {
        with_temp_config_dir_for_test(|| {
            let config = Config::default();
            config.save().expect("default config should save");

            let mut app = App::new(config);
            app.open_config_editor();
            app.add_target_city_to_draft("KOR")
                .expect("generated representative city should add");
            app.apply_config_draft()
                .expect("draft with fixed offset timezone should save");

            let saved = Config::load().expect("saved config should reload");
            assert!(
                saved
                    .tracked_cities
                    .iter()
                    .any(|city| city.code.eq_ignore_ascii_case("KOR"))
            );
            assert_eq!(saved.effective_target_city_codes(), vec!["KOR".to_string()]);
        });
    }

    #[test]
    fn selecting_anchor_city_opens_target_picker() {
        let mut app = App::new(Config::default());
        app.open_config_editor();
        app.open_picker(PickerKind::AnchorCity);
        if let Some(picker) = app.picker.as_mut() {
            picker.query = "tokyo".to_string();
        }

        app.submit_picker_selection()
            .expect("anchor selection should succeed");

        assert_eq!(
            app.config_draft
                .as_ref()
                .map(|draft| draft.effective_anchor_city_code()),
            Some("TYO".to_string())
        );
        assert!(matches!(
            app.picker.as_ref().map(|picker| &picker.kind),
            Some(PickerKind::TargetCity)
        ));
    }
}
