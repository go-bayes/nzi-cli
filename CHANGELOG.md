# Changelog

## 0.2.1
- Addis Ababa (ADD) with Ethiopia and Africa/Addis_Ababa added to tracked cities; ETB added to currency cycle with map marker support.
- Weather and map lookups now resolve Addis Ababa coordinates.
- TUI version label now follows the package version.

## 0.2.0
- Map now tracks the active panel: NZ weather highlights the selected city, time and currency show a world map route.
- NZ highlighted city labels show the full city name with star markers.
- Currency map uses currency-linked country centroids (NZD/AUD/USD/EUR/GBP/JPY).
- Time edit now exits with Esc and restores the current NZ time.
- Footer now shows a persistent time conversion or FX summary when those panels are focused.

## 0.1.7
- Exchange rates now avoid approximate offline values; cache-only fallback with clearer messaging.
- Weather shows stale/offline tags, uses location dates for weekday headers, and retries briefly on startup.
- Time conversion gives invalid local times instead of silently keeping the previous result.

## 0.1.6
- `/reload` replaces `/reset`; config reload no longer overwrites user settings.
- Time conversion is now DST-aware for the entered time (handles ambiguous/invalid local times).
- Clippy cleanups and map background rendering aligned with ratatui APIs.

## 0.1.5
- Weather grid wind arrows now match each period’s actual direction; map panel background aligns with theme.
- Cloudy/night cloud icons use the emoji variant for consistent sizing.
- Version bump to 0.1.5.

## 0.1.4
- Dependency bumps: crossterm 0.29, dirs 6, thiserror 2, toml 0.9, unicode-width 0.2.2.
- Binary default run remains `nzi`; minor UI/help polishing.

## 0.1.3
- Weather grid centred with emoji-safe alignment; “Pt cldy” fits cleanly.
- Currency stays at 0 until a live rate arrives; offline shows a message instead of fallback values; refresh is requested on swap/cycle/missing rate.
- Keybindings: space cycles cities/pairs, s swaps or toggles weather view, e enters edit for time/currency (Esc exits); titles/help updated.
- Binary is now named `nzi` (default run target) for quicker launch.
- Version bump to 0.1.3.

## 0.1.2
- Default to expanded weather grid on Wellington with New York/USD comparisons.
- Weather, time, and currency layout now scales to terminal size and keeps weather visible in splits.
- Version bump and UI header updated.
- Keybindings: space cycles cities/pairs, s swaps or toggles weather view, e enters edit for time/currency.

## 0.1.1
- Initial release of the NZI CLI with weather, map, time, and currency panels. 
