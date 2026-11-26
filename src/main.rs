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
    // make sure a panic cannot leave the terminal in raw mode
    install_panic_hook();

    // set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run
    let mut app = App::load()?;

    // initial data fetch
    app.refresh_exchange_rate().await;
    app.refresh_weather().await;

    let result = run_app(&mut terminal, &mut app).await;

    // restore terminal
    leave_tui(&mut terminal)?;

    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

/// leave the tui: alternate screen and mouse capture first, raw mode last.
/// on windows, disabling mouse capture restores the console mode that was
/// saved while raw mode was on, so disabling raw mode first would leave the
/// console in raw mode
fn leave_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let screen = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    // always attempt this, even if the step above failed
    let raw = disable_raw_mode();
    screen?;
    raw?;
    terminal.show_cursor()?;
    Ok(())
}

/// restore the terminal before the default panic report is printed
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // best effort: the panic report matters more than a failed restore.
        // raw mode goes last, because on windows disabling mouse capture
        // restores the console mode that was saved while raw mode was on
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        );
        let _ = disable_raw_mode();
        default_hook(panic_info);
    }));
}

/// main event loop
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
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
            if let Err(e) = open_editor_for_config(terminal, app).await {
                app.set_status(format!("Editor error: {}", e));
            }
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
) -> Result<()> {
    let editor = app.get_editor();
    let config_path = Config::config_path();

    // exit alternate screen so editor can use the terminal
    leave_tui(terminal)?;

    // spawn editor and wait for it to finish
    let status = Command::new(&editor).arg(&config_path).status();

    // re-enter TUI mode
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
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
