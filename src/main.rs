//! nzi-cli - new zealand around the world
//!
//! a beautiful tui connecting new zealand to the world
//!
//! features:
//! - live weather for NZ cities with emoji forecasts
//! - world clock with time zone conversion
//! - currency converter with live exchange rates
//! - beautiful braille map of aotearoa with kiwi birds
//! - catppuccin mocha theme with animations
//!
//! configuration is stored in ~/.config/nzi-cli/config.toml

mod app;
mod config;
mod exchange;
mod http;
mod map;
mod reference;
mod theme;
mod timezone;
mod ui;
mod weather;

use std::io;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::App;
use config::Config;

/// main entry point
#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::load()?;
    install_panic_hook();
    let mut session = TuiSession::default();
    session.enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    app.refresh_exchange_rate().await;
    app.refresh_weather().await;

    run_app(&mut terminal, &mut app, &mut session).await?;
    session.leave()
}

#[derive(Default)]
struct TuiSession {
    active: bool,
}

impl TuiSession {
    fn enter(&mut self) -> Result<()> {
        enable_raw_mode()?;
        // arm cleanup before the remaining setup can fail
        self.active = true;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }

    fn leave(&mut self) -> Result<()> {
        if self.active {
            self.active = false;
            restore_terminal()?;
        }
        Ok(())
    }
}

impl Drop for TuiSession {
    fn drop(&mut self) {
        // the panic hook restores the terminal before printing the error
        if !std::thread::panicking() {
            let _ = self.leave();
        }
    }
}

fn restore_terminal() -> io::Result<()> {
    let mut stdout = io::stdout();
    // attempt every step even if writing to the terminal fails. on windows,
    // disabling mouse capture restores the mode saved while raw mode was on
    let screen = execute!(stdout, LeaveAlternateScreen);
    let mouse = execute!(stdout, DisableMouseCapture);
    let cursor = execute!(stdout, Show);
    let raw = disable_raw_mode();
    screen.and(mouse).and(cursor).and(raw)
}

/// restore the terminal before the default panic report is printed
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        default_hook(panic_info);
    }));
}

/// main event loop
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    session: &mut TuiSession,
) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_data_refresh = std::time::Instant::now();
    let data_refresh_interval = Duration::from_secs(300); // 5 minutes

    loop {
        // draw ui
        terminal.draw(|f| ui::draw(f, app))?;

        // handle events with timeout for animation
        if crossterm::event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code);
        }

        // tick for animations and time updates
        if app.should_tick() {
            app.tick();
            app.reset_tick();
        }

        // check for pending weather refresh (e.g., city changed)
        if app.needs_weather_refresh() {
            app.refresh_weather().await;
        }

        // check for pending currency refresh (e.g., pair changed)
        if app.needs_currency_refresh() {
            app.currency_converter.clear_refresh_flag();
            app.refresh_exchange_rate().await;
        }

        // check for edit config request
        if app.needs_edit_config() {
            app.clear_edit_request();
            // temporarily exit TUI to open editor
            open_editor_for_config(terminal, app, session).await?;
        }

        // periodic data refresh (exchange rate + weather)
        if last_data_refresh.elapsed() > data_refresh_interval {
            app.refresh_exchange_rate().await;
            app.refresh_weather().await;
            last_data_refresh = std::time::Instant::now();
        }

        // check if we should quit
        if !app.running {
            break;
        }
    }

    Ok(())
}

/// open the config file in the user's editor
async fn open_editor_for_config(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    session: &mut TuiSession,
) -> Result<()> {
    let editor = app.get_editor();
    let config_path = Config::config_path();

    // exit alternate screen so editor can use the terminal
    session.leave()?;

    // spawn editor and wait for it to finish
    let status = Command::new(&editor).arg(&config_path).status();

    // re-enter TUI mode
    session.enter()?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    match status {
        Ok(exit_status) if exit_status.success() => {
            // reload config after successful edit
            if let Err(e) = app.reload_config() {
                app.set_status(format!("Config reload failed: {}", e));
            }
        }
        Ok(exit_status) => {
            app.set_status(format!("Editor exited with: {}", exit_status));
        }
        Err(e) => {
            app.set_status(format!("Failed to open {}: {}", editor, e));
        }
    }

    Ok(())
}
