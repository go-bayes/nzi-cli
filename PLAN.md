## Planning
- Prioritise responsive layout for the weather grid: compute widths from available space, clamp or wrap cells, and prefer compact view when the terminal is small.
- Define a config schema for map data sources, curated lists, and per-panel defaults, with safe fallbacks/ user overrides.
- Implement a searchable selector for currencies, countries, and cities with favourites and recent selections.
- Allow export/import of user presets and a reset-to-defaults flow.

## Draft config schema
```toml
[display]
show_seconds = true
use_24_hour = true
show_animations = true
animation_speed_ms = 100
editor = "nvim"

[locations]
current_city = { name = "Wellington", code = "WLG", country = "New Zealand", timezone = "Pacific/Auckland", currency = "NZD" }
home_city = { name = "New York", code = "NYC", country = "USA", timezone = "America/New_York", currency = "USD" }
tracked_cities = [
  { name = "London", code = "LDN", country = "UK", timezone = "Europe/London", currency = "GBP" },
  { name = "RIO", code = "RIO", country = "Brazil", timezone = "America/Sao_Paulo", currency = "BRL" },
]

[maps]
weather_map = { mode = "nz", highlight = "weather_city" }
time_map = { mode = "world", from = "time_from", to = "time_to" }
currency_map = { mode = "world", from = "currency_from", to = "currency_to" }

[menu]
favourites = { cities = ["WLG", "NYC", "RIO"], countries = ["NZ", "AUS", "USA", "FRA", "UK", "JPN"], currencies = ["NZD", "USD", "AUD", "EUR", "GBP", "JPY", "BRL"] }
recent_limit = 8

[tables]
countries = { source = "builtin", path = "" }
currencies = { source = "builtin", path = "" }
cities = { source = "builtin", path = "" }

[defaults]
currency_pair = { from = "NZD", to = "USD" }
time_pair = { from = "WLG", to = "NYC" }
```
