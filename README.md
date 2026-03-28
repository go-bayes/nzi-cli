# nzi

A terminal dashboard for thinking about New Zealand and its place in the world, with local weather, world clocks, currency conversion, and a configurable world map focus.

![nzi static](images/nzi.png)

## Features

- **NZ Weather** - Current conditions and 3-day forecast for NZ cities (Auckland, Wellington, Christchurch, Dunedin) with wttr-style grid view
- **World Clocks** - Track time across Wellington and your home city (London, BOS, LA, Austin, Paris, Sydney, Tokyo, Singapore, Dhaka, Beijing)
- **Currency Converter** - Live exchange rates with config-driven default pairs and cycle lists
- **Time Converter** - Convert times between NZ and overseas cities (Useful for arranging meetings)
- **Configurable World Map** - Route, city, country, and mixed map modes with focal-country control

Of course, you can get this information from a browser, but it's much nicer from the comfort of the terminal (just type 'nzi'). 

## Installation

### If you have Rust, install or update using cargo

Run this: 

```bash
cargo install --git https://github.com/go-bayes/nzi-cli
```

### Install Rust (one-time setup)

If you do not already have Rust, run this:

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# then restart your terminal, or run:
source ~/.cargo/env
```

For Windows, download the installer from [rustup.rs](https://rustup.rs).

### Building from Source

```bash
git clone https://github.com/go-bayes/nzi-cli
cd nzi-cli
cargo build --release
./target/release/nzi
```
## Usage

After installing, from your terminal, type `nzi` and then hit return/enter: the interface will spring forth to life.

Use it to

- Sketch an informal plan: travel/grants/communications (time zones)
- Retain a mental model of the weather for friends/family overseas.
- Imagine the life routines of others around the planet. 

```bash
# launch the dashboard
nzi

# show help overlay
/h

# leave help 
Esc

# quit interface/return to terminal 
q
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

### Panel Controls (depending on focus)


| Key | Action |
|-----|--------|
| `Space` | Cycle city/currency |
| `s` | Swap (time/currency) / toggle weather view |
| `e` | Edit (time/currency) |
| `0-9` | Direct entry (time in normal mode, amount in currency) |

### Picker Controls

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Move through picker results |
| `Enter` | Select highlighted option |
| `Esc` | Close picker |

### Slash Commands

| Command | Action |
|---------|--------|
| `/help` or `/h` | Show help overlay |
| `/edit` or `/e` | Edit config in $EDITOR |
| `/quit` or `/q` | Quit application |
| `/reload` (or `/r`) | Reload config from disk |
| `/country` or `/focus` | Open focal-country picker |
| `/country <query>` | Set focal country by name, alias, or ISO-3 code |
| `/currency` | Open currency pair picker |
| `/currency <from> -> <to>` | Set default currency pair |
| `/currency pin <query>` | Add a pinned currency to the effective cycle list |
| `/currency sync on|off` | Toggle derive-from-cities behaviour |
| `/map` | Open map-mode picker |
| `/map <route\|cities\|countries\|both>` | Set map mode directly |

The bare `/country`, `/currency`, and `/map` commands open interactive search overlays. The typed forms remain useful for quick direct changes and for scripting your config edits through the terminal.

## Configuration

Configuration is stored in `~/.config/nzi-cli/config.toml` and is created automatically on first run.

Change the defaults to suit. `[home_city]` is the default overseas paired city. `currency` and `map` are optional sections, so older configs still load.


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
country = "United Kingdom"
timezone = "Europe/London"
currency = "GBP"

[[tracked_cities]]
name = "Boston"
code = "BOS"
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

[currency]
sync_with_cities = true
pinned_codes = ["CAD", "JPY"]
default_from = "NZD"
default_to = "GBP"

[map]
mode = "countries"
focal_country_code = "GBR"
# focus_city_code = "BOS"
# focus_country_codes = ["USA", "GBR"]
```

## Data Sources

- **Weather**: [Open-Meteo](https://open-meteo.com/) (free, no API key required)
- **Exchange Rates**: [ExchangeRate-API](https://www.exchangerate-api.com/) (free tier)

### Default Cities (change configure to suit using `/edit`)

### NZ Cities (Weather)
Auckland, Wellington, Christchurch, Dunedin

### World Cities (Time/Currency)
London, Boston, Los Angeles, Austin, Paris, Berlin, Sydney, Tokyo, Singapore, Kuala Lumpur, Rio, Addis Ababa, Dhaka, Beijing

## Requirements

- Internet connection (for live weather and exchange rates)

### Weaknesses

-  Time/DST handling uses chrono-tz for DST-aware conversion of entered times (good), but assumes "today" as the date. Be aware there's no way to pick arbitrary dates yet; ambiguous/invalid local times are silently dropped.
-  Weather is still explicitly NZ-only. The broader customisation work currently targets map and currency behaviour, not arbitrary-country weather.
-  The picker flow currently covers focal country, currency pair, and map mode, but not full city editing, pin removal, or multi-country map management.
-  Weather lacks staleness checks, so make sure you refresh with /r if it's been sitting around a while.

## Licence

MIT
