# Product Roadmap

## Current direction
1. The app should revolve around one simple mental model:
   - one anchor city
   - one ordered list of target cities
2. The `Time` and `Currency` panels should stay aligned to that same selection model.
3. Weather remains anchored to New Zealand and does not generalise with the rest of the app.
4. The map is optional, not foundational. It may be disabled entirely, even for NZ-focused use.
5. The current config editor, draft workflow, and snapshot restore behaviour remain useful and should be preserved while the data model is simplified.

## Product decisions
1. Anchor city is the primary user choice.
2. Target cities are the primary comparison list.
3. Time behaviour derives from `anchor_city -> target_cities`.
4. Currency behaviour derives from the countries and currencies of those same target cities.
5. If the user adds a country or currency that does not yet have a matching target city, the app may default to that country’s capital city as a convenience.
6. That capital-city fallback is a helper, not the core model. The primary model remains city-based.
7. Weather stays NZ-only for now.
8. The map should be user-toggleable and may be removed from non-NZ workflows if that keeps the product cleaner.

## Why this is better
1. There is one mental model instead of parallel models for time, currency, and map state.
2. The `Time` and `Currency` windows become two views over the same user selection.
3. The editor becomes easier to understand because the user is choosing places, not managing multiple technical lists.
4. This reduces drift between panels and avoids surprising state mismatches.
5. It also gives the product a clearer identity: NZ weather plus place-to-place comparison.

## Current state
1. Config draft, apply, discard, reset, reload, and restore already exist.
2. Snapshot save and restore already exist.
3. A visual `/config` editor already exists with `Time`, `Currency`, `Map`, and `Actions` tabs.
4. Search-backed pickers already exist for country, currency, and map selections.
5. The current codebase still reflects an older, more general model:
   - `time.city_codes`
   - `currency.country_codes`
   - map country focus settings
6. That model works, but it is no longer the desired long-term shape.

## Resume here
1. Refactor the data model around `anchor_city` and `target_cities`.
2. Keep legacy config compatibility while deriving the new model from old fields where possible.
3. Simplify the config editor so it edits anchor city, target cities, and map visibility rather than parallel time and currency sources.
4. Decide whether the map remains a panel that can be hidden, or becomes an optional feature entirely outside the default layout.
5. Keep `Esc` in the config editor as “close editor only”; do not silently discard the draft.

## Design principles
1. One selection model should drive both time and currency.
2. City is the core comparison unit.
3. Country and currency data should fall out of city metadata by default.
4. Weather remains NZ-scoped.
5. Map is optional.
6. User-authored preferences stay separate from derived runtime state.
7. Restore means return to a previously saved preference snapshot.
8. Reload means re-read the current config file from disk.
9. Reset means replace draft state with built-in defaults, not old user preferences.

## Target data model
1. Keep the TOML schema backward compatible where practical.
2. Move toward a primary structure like:
   - `anchor_city_code`
   - `target_city_codes`
   - `map.enabled`
3. Keep the city catalogue:
   - `current_city`
   - `home_city`
   - `tracked_cities`
4. Derive runtime behaviour from those fields:
   - time: anchor city to each target city
   - currency: anchor city currency to each target city currency
   - map: anchor city to each target city when enabled
5. Treat currency-specific additions as convenience input that resolves back to a city where possible.
6. If a country is chosen without a city, default to the capital city for that country.
7. Keep snapshot metadata and restore support under the config directory.

## Migration strategy
1. Existing configs must continue to load.
2. Existing `time.city_codes` should map into the new target-city model.
3. Existing `currency.country_codes` should be treated as legacy override data during migration.
4. Existing map settings should be preserved where possible, but the end state should prefer `map.enabled` plus anchor-to-target rendering.
5. NYC to BOS migration behaviour must still be preserved.

## Validation rules
1. City codes remain unique case-insensitively.
2. Timezones must parse successfully.
3. Every target city code must refer to a known catalogue city.
4. Anchor city code must refer to a known catalogue city.
5. Any capital-city fallback must resolve to a known city entry before save.
6. Legacy fields may still load, but the editor should guide the user toward the simplified model.

## Preference workflow
1. `/config` edits should modify a draft, not live config.
2. Action meanings remain:
   - `apply`: persist the current draft and make it live
   - `discard`: drop the draft and return to the last loaded config
   - `reset`: replace the current draft with built-in defaults
   - `reload`: re-read the latest config file from disk
   - `restore`: load a previously saved user snapshot
3. Snapshot restore restores user preferences, not recomputed defaults.
4. Reload should never silently replace preferences with defaults.

## `/config` structure
1. Keep `/config` as the entry point for the overlay editor.
2. Rework the editor around the simplified model.
3. Preferred tabs:
   - `Places`
   - `Map`
   - `Actions`
4. `Places` should own:
   - anchor city
   - target city list
   - optional helper actions for adding a country or currency by resolving to a city
5. `Map` should own:
   - map enabled or disabled
   - any remaining NZ-specific map preferences
6. `Actions` should keep:
   - apply
   - discard
   - reset
   - reload
   - restore

## Tab responsibilities
### Places
1. Show the current anchor city explicitly.
2. Show the ordered target-city list explicitly.
3. Allow add, remove, and reorder for target cities.
4. Allow changing the anchor city without implicitly rewriting the target list.
5. Offer a helper flow for “add country” or “add currency”, which resolves to a city, usually the capital.

### Map
1. Let the user opt out of the map entirely.
2. If enabled, render maps as anchor city to target city routes.
3. Do not expand into a general world-map focus model unless there is a strong later need.

### Actions
1. Keep draft lifecycle controls in one place.
2. Keep snapshot restore visible.
3. Add snapshot browsing later if the single latest snapshot becomes limiting.

## Implementation phases
### Phase 0 — Already done
1. Draft config workflow and snapshots.
2. Initial `/config` editor shell.
3. Search-backed pickers.

### Phase 1 — Simplify the model
1. Introduce explicit anchor city and target-city list settings.
2. Derive time and currency behaviour from that shared list.
3. Keep compatibility shims for old config fields.

### Phase 2 — Rework the editor
1. Replace the current split `Time` and `Currency` editing model with a unified `Places` tab.
2. Add target-city reordering.
3. Add anchor-city selection.
4. Add helper flows for country or currency to city resolution.

### Phase 3 — Map simplification
1. Add `map.enabled`.
2. Let the user hide the map panel.
3. Reduce map configuration to anchor-to-target behaviour.

### Phase 4 — Hardening
1. Add migration tests.
2. Add editor-flow tests.
3. Update README and usage text to match the simplified model.

## Acceptance criteria
1. The user can choose one anchor city and one ordered target-city list in `/config`.
2. The `Time` and `Currency` panels derive from that same city selection.
3. The user does not need to manage separate time and currency lists in normal use.
4. Weather remains stable and NZ-scoped.
5. The user can disable the map.
6. Existing configs still load without breakage.
7. Draft apply, discard, reload, reset, and restore continue to work.

## Immediate next coding step
1. Rewrite the config model around explicit anchor city and target cities.
2. Add migration logic from `time.city_codes` and `currency.country_codes`.
3. Replace the current `Time` and `Currency` tabs with a unified `Places` tab.
4. Add map enable or disable control.
5. Keep the existing draft and snapshot workflow intact while the editor is reworked.
