# Product Roadmap

## Current direction
1. The app should revolve around one simple mental model:
   - one anchor city
   - one ordered list of target cities
2. The `Time` and `Currency` panels should stay aligned to that same selection model.
3. Weather remains anchored to New Zealand and does not generalise with the rest of the app.
4. The map is optional, not foundational. It may be disabled entirely, even for NZ-focused use.
5. The current config editor, draft workflow, and snapshot restore behaviour remain useful and should be preserved while the data model is simplified.
6. Reference data should move out of hand-written Rust arrays and into checked-in source files with generated Rust output.

## Product decisions
1. Anchor city is the primary user choice.
2. Target cities are the primary comparison list.
3. Time behaviour derives from `anchor_city -> target_cities`.
4. Currency behaviour derives from the countries and currencies of those same target cities.
5. If the user adds a country or currency that does not yet have a matching target city, the app may default to that country’s capital city as a convenience.
6. That capital-city fallback is a helper, not the core model. The primary model remains city-based.
7. Weather stays NZ-only for now.
8. The map should be user-toggleable and may be removed from non-NZ workflows if that keeps the product cleaner.
9. `/currency` should no longer behave like a separate FX-configuration command. It should resolve currency to country, then country to representative city, and finally add that city to the target-city list.
10. `/country` and `/currency` should converge on the same internal place-selection path.

## Why this is better
1. There is one mental model instead of parallel models for time, currency, and map state.
2. The `Time` and `Currency` windows become two views over the same user selection.
3. The editor becomes easier to understand because the user is choosing places, not managing multiple technical lists.
4. This reduces drift between panels and avoids surprising state mismatches.
5. It also gives the product a clearer identity: NZ weather plus place-to-place comparison.

## Current state
1. Config draft, apply, discard, reset, reload, and restore already exist.
2. Snapshot save and restore already exist.
3. A visual `/config` editor already exists with `Places` and `Actions`.
4. Search-backed pickers already exist for anchor city, target city, country, currency, and map visibility.
5. The current codebase still carries a few compatibility paths from the older model:
   - `time.city_codes`
   - `currency.country_codes`
6. Reference data now comes from checked-in source files in `data/` and is generated at build time through `build.rs`.
7. The editor now has a `Places` tab with anchor-city selection, target-city add or remove or reorder, country and currency helper flows, and clearer on-panel guidance.
8. `/currency` now follows the place model by resolving `currency -> country -> representative city -> target city`.
9. Time and currency interactions are re-coupled and now follow the same active target city.
10. The map now defaults to off, can be toggled from `/map` or `Actions`, disappears from the main layout when disabled, and is fixed to country-level rendering.
11. Country coverage is now effectively complete, with one representative city per supported country.
12. The next major constraint is no longer country coverage. It is whether the app should expose a broader city catalogue beyond one representative city per country.

## Resume here
1. Decide whether target-city search should stay at one representative city per country or expand into a broader curated city catalogue.
2. Review shared-currency policy country by country, especially where one currency maps to several states.
3. Decide whether to expose a small visible indicator for the currently active target city across the time and currency panels.
4. Keep `Esc` in the config editor as “close editor only”; do not silently discard the draft.
5. Keep direct commands such as `/map` immediate, even when a draft exists, unless there is a strong reason to route them through the draft.

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
10. Currency commands should add places, not create an independent list of FX preferences.
11. Canonical reference facts and curated product defaults should live in separate source files.

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
5. Treat currency-specific additions as convenience input that resolves back to a city through country.
6. If a country is chosen without a city, default to the capital city for that country.
7. Keep snapshot metadata and restore support under the config directory.
8. Legacy `currency` config may continue to load, but the primary runtime path should derive from anchor and target cities.

## Reference data strategy
1. Keep runtime fully offline and deterministic.
2. Avoid introducing a country or city crate as the primary source of truth.
3. Store canonical country and currency facts in `data/countries.csv`.
4. Store curated representative-city defaults in `data/representative_cities.json`.
5. Generate Rust tables from those files in `build.rs`.
6. Keep generated Rust out of hand-edited source modules where practical.
7. Treat representative-city choice as a product decision, not raw reference fact.
8. Allow multi-timezone countries to start with one default representative city, with optional later expansion.
9. Shared-currency cases such as `EUR` need an explicit canonical focal-country policy for the `/currency` shortcut.

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
   - `Actions`
4. `Places` should own:
   - anchor city
   - target city list
   - optional helper actions for adding a country or currency by resolving to a city
5. `Actions` should keep:
   - apply
   - discard
   - reset
   - reload
   - restore
   - map visibility

## Tab responsibilities
### Places
1. Show the current anchor city explicitly.
2. Show the ordered target-city list explicitly.
3. Allow add, remove, and reorder for target cities.
4. Allow changing the anchor city without implicitly rewriting the target list.
5. Offer a helper flow for “add country” or “add currency”, which resolves to a city, usually the capital.
6. Keep country and currency helpers semantically aligned by resolving both through country to representative city.

### Actions
1. Keep draft lifecycle controls in one place.
2. Keep snapshot restore visible.
3. Keep map visibility here rather than in a separate config tab.
4. Add snapshot browsing later if the single latest snapshot becomes limiting.

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
3. Reduce map configuration to a simple visibility toggle with country-level rendering.

### Phase 4 — Hardening
1. Add migration tests.
2. Add editor-flow tests.
3. Update README and usage text to match the simplified model.
4. Remove stale references to `/currency` as a separate FX configuration surface.

### Phase 5 — Generated reference data
1. Add `data/countries.csv` for canonical country and currency metadata.
2. Add `data/representative_cities.json` for curated default city metadata.
3. Add `build.rs` to validate and generate Rust reference tables.
4. Swap `reference.rs` to consume generated data without changing its public lookup API.
5. Add tests covering country count, representative-city coverage, and shared-currency policy.

## Acceptance criteria
1. The user can choose one anchor city and one ordered target-city list in `/config`.
2. The `Time` and `Currency` panels derive from that same city selection.
3. The user does not need to manage separate time and currency lists in normal use.
4. Weather remains stable and NZ-scoped.
5. The user can disable the map.
6. Existing configs still load without breakage.
7. Draft apply, discard, reload, reset, and restore continue to work.
8. `/currency` behaves as a place-selection shortcut rather than a separate configuration path.
9. Country and currency search cover the full supported registry.
10. Each supported country resolves to one default representative city.

## Immediate next coding step
1. Decide whether to keep one representative city per country or introduce a broader curated city list for target search.
2. Add more coverage tests around shared-currency choices and command behaviour that should remain immediate.
3. Keep tightening editor wording where the behaviour is correct but the panel is easy to misread.
