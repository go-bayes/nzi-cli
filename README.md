# nzi-cli

A terminal dashboard with useful information for thinking about New Zealand and its place in the world: local weather, world clocks, and currency conversion with a (beautiful) catppuccin mocha theme.

## Features

- **NZ Weather** - Current conditions and 3-day forecast for NZ cities (Auckland, Wellington, Christchurch, Dunedin) with wttr-style grid view
- **World Clocks** - Track time across Wellington and your home city (London, NYC, LA, Austin, Paris, Sydney, Tokyo, Singapore)
- **Currency Converter** - Live exchange rates between NZD and major currencies
- **Time Converter** - Convert times between NZ and overseas cities (Useful for arranging calls)

Of course you can get this information from a browser but it's much nicer to have the key information at your fingertips, from the terminal.


## Installation

### Install using cargo

If you have Rust installed:

```bash
cargo install --git https://github.com/go-bayes/nzi-cli
```

### Installing Rust

If you don't have Rust, install it first:

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# then restart your terminal, or run:
source ~/.cargo/env
```

For Windows, download the installer from [rustup.rs](https://rustup.rs).

Once Rust is installed, run the `cargo install` command above.

## Usage

```bash
# launch the dashboard
nzi-cli

# show help overlay
# press ? while running
```

## Keybindings

Type `/help` to show the help overlay.

### Navigation

| Key | Action |
|-----|--------|
| `Tab` / `↑↓←→` | Cycle between panels |
| `h/j/k/l` | Cycle between panels (vim-style) |
| `Esc` | Close help / cancel |
| `q` | Quit application |

### Panel Controls (when focused)

| Key | Action |
|-----|--------|
| `Space` | Cycle city/currency |
| `s` | Swap (time/currency) |
| `0-9` | Enter time/amount |
| `e` | Toggle weather grid view |

### Slash Commands

| Command | Action |
|---------|--------|
| `/help` or `/h` | Show help overlay |
| `/edit` or `/e` | Edit config in $EDITOR |
| `/quit` or `/q` | Quit application |
| `/reset` or `/r` | Reset config to defaults |

## Configuration

Configuration is stored in `~/.config/nzi-cli/config.toml` and is created automatically on first run.

```toml
[current_city]
name = "Wellington"
code = "WLG"
country = "New Zealand"
timezone = "Pacific/Auckland"
currency = "NZD"

[home_city]
name = "London"
code = "LDN"
country = "UK"
timezone = "Europe/London"
currency = "GBP"

[[tracked_cities]]
name = "New York"
code = "NYC"
country = "USA"
timezone = "America/New_York"
currency = "USD"

# ... more cities

[display]
show_seconds = true
use_24_hour = true
show_animations = true
animation_speed_ms = 100
# editor = "nvim"  # defaults to $EDITOR or nvim
```

## Data Sources

- **Weather**: [Open-Meteo](https://open-meteo.com/) (free, no API key required)
- **Exchange Rates**: [ExchangeRate-API](https://www.exchangerate-api.com/) (free tier)

## Available Cities

### NZ Cities (Weather)
Auckland, Wellington, Christchurch, Dunedin

### World Cities (Time/Currency)
London, New York, Los Angeles, Austin, Paris, Sydney, Tokyo, Singapore

## Requirements

- Terminal with Unicode support (for braille map and icons)
- Internet connection (for live weather and exchange rates; fallback rates available offline)

## Building from Source

```bash
git clone https://github.com/go-bayes/nzi-cli
cd nzi-cli
cargo build --release
./target/release/nzi-cli
```

## Licence

MIT
