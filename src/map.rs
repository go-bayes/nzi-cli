//! new zealand map display using ratatui canvas with braille markers
//! includes city markers, animations, waves, and birds

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    symbols::Marker,
    text::Span,
    widgets::{
        Block, BorderType, Borders, Widget,
        canvas::{Canvas, Map, MapResolution, Points},
    },
};

use crate::theme::{Theme, catppuccin};

// nz bounding box for canvas map (from nzme-cli)
pub const NZ_LAT_MIN: f64 = -47.5;
pub const NZ_LAT_MAX: f64 = -34.0;
pub const NZ_LON_MIN: f64 = 166.0;
pub const NZ_LON_MAX: f64 = 179.0;

/// city locations (lon, lat) for map markers - NZ cities only
#[derive(Debug, Clone)]
pub struct CityMarker {
    pub code: &'static str,
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
}

impl CityMarker {
    pub const fn new(code: &'static str, name: &'static str, lat: f64, lon: f64) -> Self {
        Self {
            code,
            name,
            lat,
            lon,
        }
    }
}

/// major NZ cities with coordinates (reduced to 4 main centres)
pub const NZ_CITIES: &[CityMarker] = &[
    CityMarker::new("AKL", "Auckland", -36.8485, 174.7633),
    CityMarker::new("WLG", "Wellington", -41.2865, 174.7762),
    CityMarker::new("CHC", "Christchurch", -43.5321, 172.6362),
    CityMarker::new("DUD", "Dunedin", -45.8788, 170.5028),
];

/// canvas-based nz map widget with braille rendering
#[derive(Default)]
pub struct NzMapCanvas {
    tick: u64,
    highlight_city: Option<String>,
    focused: bool,
}

impl NzMapCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    pub fn highlight_city(mut self, code: Option<String>) -> Self {
        self.highlight_city = code;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for NzMapCanvas {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rainbow = Theme::rainbow_colors();
        let tick = self.tick as usize;

        // ensure map background matches theme rather than terminal default
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(catppuccin::BASE);
                    // clear symbol so background shows through consistently
                    cell.set_symbol(" ");
                }
            }
        }

        // rainbow colour cycling for the map coastline (like nzme-cli)
        let map_color = rainbow[(tick / 3) % rainbow.len()];

        // wave animation along the bottom of the map
        let wave_points: Vec<(f64, f64)> = (0..70)
            .map(|i| {
                let t = self.tick as f64 / 6.0;
                let x = NZ_LON_MIN + (NZ_LON_MAX - NZ_LON_MIN) * (i as f64 / 70.0);
                let y = -47.0 + (t + i as f64 / 5.0).sin() * 0.12;
                (x, y)
            })
            .collect();

        // flying birds animation - multiple flocks across NZ
        let bird_span = NZ_LON_MAX - NZ_LON_MIN;
        let tick_f = self.tick as f64;

        // north island flock (near Auckland)
        let north_offset = (tick_f / 10.0) % bird_span;
        let north_y = -36.5 + ((tick_f / 20.0).sin() * 0.4);

        // south island flock (near Christchurch) - moves opposite direction
        let south_offset = bird_span - ((tick_f / 12.0) % bird_span);
        let south_y = -43.5 + ((tick_f / 25.0).cos() * 0.3);

        // deep south flock (near Queenstown)
        let deep_south_offset = (tick_f / 15.0) % bird_span;
        let deep_south_y = -45.0 + ((tick_f / 30.0).sin() * 0.2);

        // kiwi birds (slower, ground level) - these don't fly but waddle!
        let kiwi_offset = (tick_f / 25.0) % (bird_span * 0.3);

        let birds = vec![
            // north island flock
            (NZ_LON_MIN + north_offset, north_y),
            (NZ_LON_MIN + north_offset - 0.8, north_y + 0.15),
            (NZ_LON_MIN + north_offset - 1.6, north_y - 0.1),
            // south island flock
            (NZ_LON_MIN + south_offset, south_y),
            (NZ_LON_MIN + south_offset + 0.7, south_y + 0.2),
            // deep south
            (NZ_LON_MIN + deep_south_offset + 2.0, deep_south_y),
            (NZ_LON_MIN + deep_south_offset + 2.8, deep_south_y + 0.1),
            // kiwi near wellington (ground level, slower)
            (174.5 + kiwi_offset, -41.3),
        ];

        let highlight_city = self.highlight_city.clone();

        let (border_type, border_color) = if self.focused {
            (BorderType::Double, catppuccin::YELLOW)
        } else {
            (BorderType::Rounded, catppuccin::SURFACE1)
        };

        let title_style = if self.focused {
            Style::default()
                .fg(catppuccin::YELLOW)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::block_title()
        };

        let canvas = Canvas::default()
            .block(
                Block::default()
                    .style(Style::default().bg(catppuccin::BASE))
                    .borders(Borders::ALL)
                    .border_type(border_type)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(" 🥝 Aotearoa ", title_style)),
            )
            .background_color(catppuccin::BASE)
            .marker(Marker::Braille)
            .x_bounds([NZ_LON_MIN, NZ_LON_MAX])
            .y_bounds([NZ_LAT_MIN, NZ_LAT_MAX])
            .paint(move |ctx| {
                // draw NZ using the built-in high-resolution world map
                ctx.draw(&Map {
                    color: map_color,
                    resolution: MapResolution::High,
                });

                // draw wave animation
                ctx.draw(&Points {
                    coords: &wave_points,
                    color: catppuccin::GREEN,
                });

                // draw flying birds
                ctx.draw(&Points {
                    coords: &birds,
                    color: catppuccin::YELLOW,
                });

                // draw city markers
                for city in NZ_CITIES {
                    let is_highlighted = highlight_city
                        .as_ref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(city.code));

                    let dot_color = if is_highlighted {
                        catppuccin::YELLOW
                    } else {
                        catppuccin::SAPPHIRE
                    };

                    // city dot
                    ctx.draw(&Points {
                        coords: &[(city.lon, city.lat)],
                        color: dot_color,
                    });

                    // city label
                    let label = if is_highlighted {
                        format!("{}*", city.code)
                    } else {
                        city.code.to_string()
                    };
                    ctx.print(city.lon + 0.25, city.lat + 0.15, label);
                }
            });

        canvas.render(area, buf);
    }
}

/// sparkle decoration widget with constellation-like patterns
pub struct Sparkles {
    frame: usize,
    density: usize,
}

impl Sparkles {
    pub fn new(frame: usize) -> Self {
        Self { frame, density: 8 }
    }

    pub fn density(mut self, density: usize) -> Self {
        self.density = density;
        self
    }
}

impl Widget for Sparkles {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // varied sparkle characters - stars and celestial symbols
        let sparkle_chars = ['✦', '✧', '⋆', '·', '✵', '✶', '˚', '°'];
        // very slow animation - peaceful, stargazing feel
        let slow_frame = self.frame / 12;

        // phase for twinkling effect (some stars brighter than others)
        let twinkle_phase = (self.frame % 60) as f64 / 60.0 * std::f64::consts::PI * 2.0;

        for y in 0..area.height {
            for x in 0..area.width {
                // use prime numbers for more natural distribution
                let hash = (x as usize * 37 + y as usize * 23 + slow_frame * 7) % self.density;
                if hash == 0 {
                    // different sparkle types based on position
                    let char_idx = (x as usize * 13 + y as usize * 11) % sparkle_chars.len();
                    let ch = sparkle_chars[char_idx];

                    // twinkling: some stars pulse in brightness based on position
                    let star_phase = (x as f64 * 0.3 + y as f64 * 0.7 + twinkle_phase).sin();
                    let should_show = star_phase > -0.3; // stars appear ~65% of time, creating twinkle

                    if should_show {
                        // colour cycling with offset based on position for wave effect
                        let color_offset = (x as usize / 8 + slow_frame) % 7;
                        let color = Theme::rainbow(color_offset);
                        if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                            cell.set_char(ch).set_style(Style::default().fg(color));
                        }
                    }
                }
            }
        }
    }
}
