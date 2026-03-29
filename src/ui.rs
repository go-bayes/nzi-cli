//! ui rendering for nzi-cli
//! handles layout and drawing all widgets
//! inspired by nzme-cli's high-density, information-rich design

use chrono::NaiveDate;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use unicode_width::UnicodeWidthStr;

use crate::app::{App, ConfigTab, Focus, InputMode};
use crate::config::City;
use crate::map::{NZ_CITIES, NzMapCanvas, Sparkles, WorldMapCanvas, WorldMarker};
use crate::reference::{country_by_code, focal_country_code_for_currency};
use crate::theme::{Theme, catppuccin};
use crate::timezone::CityTime;
use crate::weather::{city_coords_by_code, city_coords_by_name};

/// main ui rendering function
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // fill background with base colour
    let bg_block = Block::default().style(Style::default().bg(catppuccin::BASE));
    frame.render_widget(bg_block, area);

    // main layout: header (3), content (flexible), footer (3)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header with rainbow animation
            Constraint::Min(12),   // content
            Constraint::Length(3), // footer with city codes + help hint
        ])
        .split(area);

    draw_header(frame, main_chunks[0], app);
    draw_content(frame, main_chunks[1], app);
    draw_footer(frame, main_chunks[2], app);

    if app.config_editor_state().is_some() {
        draw_config_editor_overlay(frame, area, app);
    }

    if app.picker.is_some() {
        draw_picker_overlay(frame, area, app);
    } else if app.show_help && app.config_editor_state().is_none() {
        draw_help_overlay(frame, area);
    }
}

fn draw_config_editor_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let Some(editor) = app.config_editor_state() else {
        return;
    };
    let Some(config) = app.config_editor_config() else {
        return;
    };

    let popup_width = 80.min(area.width.saturating_sub(4));
    let popup_height = 24.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default().style(Style::default().bg(catppuccin::BASE)),
        popup_area,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(catppuccin::GREEN))
        .title(Span::styled(
            " Config Editor [Esc] ",
            Style::default()
                .fg(catppuccin::GREEN)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let tab_line = Line::from(
        [ConfigTab::Places, ConfigTab::Actions]
            .into_iter()
            .flat_map(|tab| {
                let is_active = tab == editor.tab;
                [
                    Span::styled(
                        format!(" {} ", tab.label()),
                        Style::default()
                            .fg(if is_active {
                                catppuccin::BASE
                            } else {
                                catppuccin::OVERLAY1
                            })
                            .bg(if is_active {
                                catppuccin::GREEN
                            } else {
                                catppuccin::SURFACE1
                            })
                            .add_modifier(if is_active {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                    Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>(),
    );

    let body_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new(tab_line), body_area[0]);

    let lines = match editor.tab {
        ConfigTab::Places => config_editor_places_lines(app, config, editor.selected),
        ConfigTab::Actions => {
            config_editor_action_lines(editor.selected, config.effective_map_settings().enabled)
        }
    };
    let viewport_lines = body_area[1].height as usize;
    let selected_line = config_editor_selected_line_index(app, config, editor);
    let scroll_offset = centered_scroll_offset(lines.len(), viewport_lines, selected_line);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset as u16, 0)),
        body_area[1],
    );

    let footer = match editor.tab {
        ConfigTab::Places => Line::from(vec![
            Span::styled("[Tab]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" tabs ", Theme::text_muted()),
            Span::styled("[j/k]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" move ", Theme::text_muted()),
            Span::styled("[J/K]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" reorder ", Theme::text_muted()),
            Span::styled("[Enter]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" select ", Theme::text_muted()),
            Span::styled("[a]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" add ", Theme::text_muted()),
            Span::styled("[x]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" remove ", Theme::text_muted()),
            Span::styled("[Esc]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" close", Theme::text_muted()),
        ]),
        ConfigTab::Actions => Line::from(vec![
            Span::styled("[Tab]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" tabs ", Theme::text_muted()),
            Span::styled("[j/k]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" move ", Theme::text_muted()),
            Span::styled("[Enter]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" run action ", Theme::text_muted()),
            Span::styled("[Esc]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" close", Theme::text_muted()),
        ]),
    };
    frame.render_widget(Paragraph::new(footer), body_area[2]);
}

fn config_editor_places_lines(
    app: &App,
    config: &crate::config::Config,
    selected: usize,
) -> Vec<Line<'static>> {
    let anchor_code = config.effective_anchor_city_code();
    let anchor_city = config
        .all_cities()
        .into_iter()
        .find(|city| city.code.eq_ignore_ascii_case(&anchor_code));
    let anchor_label = anchor_city
        .map(|city| format!("{} ({})", city.name, city.code))
        .unwrap_or(anchor_code.clone());

    let mut lines = vec![
        Line::from(vec![Span::styled(
            "Places",
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("Choose one anchor city and one ordered list of target cities."),
        Line::from("Press Enter to change the anchor city or open add target city."),
        Line::from("Use a to add, x to remove the selected target city, and J/K to reorder."),
        Line::from("Search matches city names, country names, and currency names or codes."),
        Line::from(""),
        config_editor_row(selected == 0, "Anchor city", &anchor_label),
    ];

    let codes = config.effective_target_city_codes();
    for (index, code) in codes.iter().enumerate() {
        let city = config
            .all_cities()
            .into_iter()
            .find(|city| city.code.eq_ignore_ascii_case(code));
        let label = city
            .map(|city| format!("{} ({})", city.name, city.code))
            .unwrap_or_else(|| code.clone());
        let detail = city
            .map(|city| format!("{} · {}", city.country, city.currency))
            .unwrap_or_else(|| "Unknown".to_string());
        lines.push(config_editor_row(selected == index + 1, &label, &detail));
    }

    lines.push(config_editor_row(
        selected == codes.len() + 1,
        "[+] Add target city",
        "Search by city, country, or currency",
    ));

    if app.has_config_draft() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Draft edits stay local until you apply them.",
            Style::default().fg(catppuccin::OVERLAY0),
        )]));
    }

    lines
}

fn config_editor_action_lines(selected: usize, map_enabled: bool) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Draft actions",
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("Apply writes to config.toml and snapshots the current live config."),
        Line::from("Press Enter to run the highlighted action."),
        Line::from(""),
        config_editor_row(selected == 0, "Apply draft", "Save and close"),
        config_editor_row(selected == 1, "Discard draft", "Drop unsaved changes"),
        config_editor_row(selected == 2, "Reset draft", "Replace with package defaults"),
        config_editor_row(
            selected == 3,
            "Reload from disk",
            "Refresh draft from config.toml",
        ),
        config_editor_row(
            selected == 4,
            "Restore snapshot",
            "Load latest saved preferences",
        ),
        config_editor_row(
            selected == 5,
            "Toggle map",
            if map_enabled { "On" } else { "Off" },
        ),
    ]
}

fn config_editor_selected_line_index(
    app: &App,
    config: &crate::config::Config,
    editor: &crate::app::ConfigEditorState,
) -> usize {
    match editor.tab {
        ConfigTab::Places => {
            let base_line = 4;
            let target_count = config.effective_target_city_codes().len();
            let row_count = 2 + target_count;
            let selected = editor.selected.min(row_count.saturating_sub(1));
            let mut line_index = base_line + selected;
            if app.has_config_draft() {
                line_index += 2;
            }
            line_index
        }
        ConfigTab::Actions => {
            let base_line = 4;
            let row_count = 6usize;
            let selected = editor.selected.min(row_count.saturating_sub(1));
            base_line + selected
        }
    }
}

fn centered_scroll_offset(total_lines: usize, viewport_lines: usize, selected_line: usize) -> usize {
    if viewport_lines == 0 || total_lines <= viewport_lines {
        return 0;
    }

    let half = viewport_lines / 2;
    let mut offset = selected_line.saturating_sub(half);
    let max_offset = total_lines.saturating_sub(viewport_lines);
    if offset > max_offset {
        offset = max_offset;
    }
    offset
}

fn config_editor_row(selected: bool, label: &str, detail: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            if selected { "▸ " } else { "  " },
            Style::default().fg(if selected {
                catppuccin::GREEN
            } else {
                catppuccin::SURFACE2
            }),
        ),
        Span::styled(
            format!("{:<28}", label),
            Style::default().fg(if selected {
                catppuccin::TEXT
            } else {
                catppuccin::SUBTEXT1
            }),
        ),
        Span::styled(
            detail.to_string(),
            Style::default().fg(if selected {
                catppuccin::SAPPHIRE
            } else {
                catppuccin::OVERLAY0
            }),
        ),
    ])
}

fn draw_picker_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let title = app.picker_title().unwrap_or_else(|| "Picker".to_string());
    let prompt = app.picker_prompt().unwrap_or("");
    let Some(picker) = app.picker.as_ref() else {
        return;
    };
    let options = app.picker_options();

    let popup_width = 64.min(area.width.saturating_sub(4));
    let popup_height = 16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default().style(Style::default().bg(catppuccin::BASE)),
        popup_area,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(catppuccin::YELLOW))
        .title(Span::styled(
            format!(" {} [Esc] ", title),
            Style::default()
                .fg(catppuccin::YELLOW)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Search: ", Style::default().fg(catppuccin::PEACH)),
            Span::styled(
                format!("{}█", picker.query),
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![Span::styled(
            prompt,
            Style::default().fg(catppuccin::OVERLAY0),
        )]),
        Line::from(""),
    ];

    if options.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No matches",
            Style::default().fg(catppuccin::RED),
        )]));
    } else {
        let selected = picker.selected.min(options.len().saturating_sub(1));
        let mut start = selected.saturating_sub(3);
        let end = (start + 8).min(options.len());
        if end - start < 8 && end == options.len() {
            start = end.saturating_sub(8);
        }

        lines.push(Line::from(vec![
            Span::styled(
                format!("Result {} of {}", selected + 1, options.len()),
                Style::default().fg(catppuccin::OVERLAY0),
            ),
            Span::raw(" "),
            Span::styled(
                if start > 0 { "↑ more" } else { "" },
                Style::default().fg(catppuccin::SUBTEXT0),
            ),
            Span::raw(" "),
            Span::styled(
                if end < options.len() { "↓ more" } else { "" },
                Style::default().fg(catppuccin::SUBTEXT0),
            ),
        ]));

        for (index, option) in options[start..end].iter().enumerate() {
            let absolute_index = start + index;
            let is_selected = absolute_index == selected;
            lines.push(Line::from(vec![
                Span::styled(
                    if is_selected { "▸ " } else { "  " },
                    Style::default().fg(if is_selected {
                        catppuccin::GREEN
                    } else {
                        catppuccin::SURFACE2
                    }),
                ),
                Span::styled(
                    format!("{:<26}", option.label),
                    Style::default().fg(if is_selected {
                        catppuccin::TEXT
                    } else {
                        catppuccin::SUBTEXT1
                    }),
                ),
                Span::styled(
                    option.detail.clone(),
                    Style::default().fg(if is_selected {
                        catppuccin::SAPPHIRE
                    } else {
                        catppuccin::OVERLAY0
                    }),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[j/k]", Style::default().fg(catppuccin::OVERLAY1)),
        Span::styled(" move ", Theme::text_muted()),
        Span::styled("[Enter]", Style::default().fg(catppuccin::OVERLAY1)),
        Span::styled(" select ", Theme::text_muted()),
        Span::styled("[Esc]", Style::default().fg(catppuccin::OVERLAY1)),
        Span::styled(" cancel", Theme::text_muted()),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// draw help overlay popup
fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    // centre the help box
    let help_width = 50.min(area.width.saturating_sub(4));
    let help_height = 28.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;
    let help_area = Rect::new(x, y, help_width, help_height);

    // clear the area behind
    frame.render_widget(Clear, help_area);
    let clear = Block::default().style(Style::default().bg(catppuccin::BASE));
    frame.render_widget(clear, help_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(catppuccin::MAUVE))
        .title(Span::styled(
            " Help [Esc] to close ",
            Style::default()
                .fg(catppuccin::MAUVE)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Tab/↑↓←→  ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Cycle between panels",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  h/j/k/l   ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Cycle between panels (vim)",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Close help / cancel / exit edit",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  q         ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled("Quit application", Style::default().fg(catppuccin::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Panels",
                Style::default()
                    .fg(catppuccin::PEACH)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (when focused)", Style::default().fg(catppuccin::SUBTEXT0)),
        ]),
        Line::from(vec![
            Span::styled("  Space     ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Cycle weather city / current target",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  s         ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Swap current comparison / toggle weather view",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  e         ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Edit time panel input or FX amount",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  0-9       ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Direct entry (time in normal mode, amount in currency)",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled("Leave edit", Style::default().fg(catppuccin::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Hint      ", Style::default().fg(catppuccin::OVERLAY0)),
            Span::styled(
                "Title bars show keys (space, s, e)",
                Style::default().fg(catppuccin::SUBTEXT0),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Slash Commands",
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  /help     ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled("Show this help", Style::default().fg(catppuccin::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  /edit     ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Edit config in $EDITOR",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /config   ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Open the staged Places editor",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /quit     ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled("Quit application", Style::default().fg(catppuccin::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  /reload   ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Reload config from disk",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /apply    ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Save the current config draft",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /discard  ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Drop the current config draft",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /reset    ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Reset draft to defaults",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /restore  ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Load latest saved preferences into draft",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /country  ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Set focal city through country",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /currency ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Add a place by currency",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  /map      ", Style::default().fg(catppuccin::SAPPHIRE)),
            Span::styled(
                "Open picker or set on|off|cities|countries|both",
                Style::default().fg(catppuccin::TEXT),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Config Editor",
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Places: anchor city + ordered target cities"),
        Line::from("  j/k move  J/K reorder  Enter select  a add  x remove"),
        Line::from("  Add-target search matches city, country, and currency terms"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Examples",
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  /config"),
        Line::from("  /country united kingdom"),
        Line::from("  /currency yen"),
        Line::from("  /map off"),
    ];

    let para = Paragraph::new(help_text);
    frame.render_widget(para, inner);
}

/// draw the header with animated rainbow sparkles
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(catppuccin::SURFACE1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // render sparkle background
    if app.config.display.show_animations {
        frame.render_widget(Sparkles::new(app.animation_frame).density(12), inner);
    }

    // render rainbow animated title
    let title = "NZ AROUND THE WORLD";
    let subtitle: Option<&str> = None;
    let rainbow = Theme::rainbow_colors();
    // slow down rainbow animation for more relaxing effect
    let slow_frame = app.animation_frame / 8;

    let mut title_spans: Vec<Span> = vec![Span::raw("  ✦ ")];
    for (i, ch) in title.chars().enumerate() {
        let color = rainbow[(i + slow_frame) % rainbow.len()];
        title_spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(subtitle) = subtitle {
        title_spans.push(Span::styled(
            format!(" ✦  {}", subtitle),
            Style::default().fg(catppuccin::SUBTEXT0),
        ));
    } else {
        title_spans.push(Span::styled(
            " ✦",
            Style::default().fg(catppuccin::SUBTEXT0),
        ));
    }

    // version on the right
    let version = format!("v{} ", env!("CARGO_PKG_VERSION"));
    let version_span = Span::styled(version, Style::default().fg(catppuccin::OVERLAY0));

    // center the title
    let title_line = Line::from(title_spans);
    let para = Paragraph::new(title_line).alignment(Alignment::Center);
    frame.render_widget(para, inner);

    // render version in top right
    if inner.width > 10 {
        let version_area = Rect::new(inner.x + inner.width.saturating_sub(8), inner.y, 8, 1);
        frame.render_widget(
            Paragraph::new(version_span).alignment(Alignment::Right),
            version_area,
        );
    }
}

/// draw the main content area with dynamic layout based on weather expansion
fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    if !app.map_enabled() {
        draw_content_without_map(frame, area, app);
        return;
    }

    // decide whether expanded grid can fit; otherwise fall back to compact
    let mut use_expanded = app.weather_expanded;
    if use_expanded {
        // allow expanded view on smaller terminals; fall back only when truly too small
        let rhs_est_width = area.width.saturating_mul(62) / 100; // matches expanded split
        let min_grid_w = 40;
        let min_grid_h = 10;
        if rhs_est_width < min_grid_w || area.height < min_grid_h {
            use_expanded = false;
        }
    }

    if use_expanded {
        // expanded view: weather on the right, capped height to avoid empty space
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(38), // map
                Constraint::Percentage(62), // info panels
            ])
            .split(area);

        // scale weather height with terminal height; reserve a small strip for time/currency
        let rhs_height = body[1].height;
        let min_bottom = 7;
        let min_weather = 14;

        let mut weather_height = rhs_height.saturating_sub(min_bottom);
        if weather_height < min_weather {
            // when very tight, still give weather the majority
            weather_height = rhs_height.saturating_sub(min_bottom / 2);
        }

        // ensure bottom has at least a minimal height when possible
        let mut bottom_height = rhs_height.saturating_sub(weather_height);
        if bottom_height < min_bottom && rhs_height > min_bottom {
            bottom_height = min_bottom.min(rhs_height);
            weather_height = rhs_height.saturating_sub(bottom_height);
        }

        let right_side = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(weather_height), // weather chunk scales to fit
                Constraint::Length(bottom_height),  // leave room for utilities
            ])
            .split(body[1]);

        let bottom_right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(right_side[1]);

        draw_map_panel(frame, body[0], app);
        draw_weather_panel_expanded(frame, right_side[0], app);
        draw_time_panel(frame, bottom_right[0], app);
        draw_currency_panel(frame, bottom_right[1], app);
    } else {
        // compact view: map on left, weather + utilities on right
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // map
                Constraint::Percentage(60), // info panels
            ])
            .split(area);

        let right_side = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),    // compact weather panel
                Constraint::Length(11), // world clocks + fx
            ])
            .split(body[1]);

        let bottom_right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(right_side[1]);

        draw_map_panel(frame, body[0], app);
        draw_weather_panel(frame, right_side[0], app);
        draw_time_panel(frame, bottom_right[0], app);
        draw_currency_panel(frame, bottom_right[1], app);
    }
}

fn draw_content_without_map(frame: &mut Frame, area: Rect, app: &App) {
    let mut use_expanded = app.weather_expanded;
    if use_expanded && area.height < 14 {
        use_expanded = false;
    }

    if use_expanded {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(14), Constraint::Length(7)])
            .split(area);

        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(body[1]);

        draw_weather_panel_expanded(frame, body[0], app);
        draw_time_panel(frame, bottom[0], app);
        draw_currency_panel(frame, bottom[1], app);
    } else {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(11)])
            .split(area);

        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(body[1]);

        draw_weather_panel(frame, body[0], app);
        draw_time_panel(frame, bottom[0], app);
        draw_currency_panel(frame, bottom[1], app);
    }
}

/// create a styled block with focus indication
fn styled_block(title: &str, focused: bool) -> Block<'static> {
    let (border_type, border_color) = if focused {
        (BorderType::Double, catppuccin::YELLOW)
    } else {
        (BorderType::Rounded, catppuccin::SURFACE1)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", title),
            if focused {
                Style::default()
                    .fg(catppuccin::YELLOW)
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::block_title()
            },
        ))
}

/// draw the new zealand map panel with canvas/braille rendering
fn draw_map_panel(frame: &mut Frame, area: Rect, app: &App) {
    let map_settings = app.config.effective_map_settings();
    if !map_settings.enabled {
        return;
    }

    let context = app.active_map_focus();

    match context {
        Focus::Weather => {
            let highlight = Some(app.get_weather_city_code().to_string());
            frame.render_widget(
                NzMapCanvas::new()
                    .highlight_city(highlight)
                    .tick(app.animation_frame as u64)
                    .focused(app.focus == Focus::Map),
                area,
            );
        }
        Focus::TimeConvert | Focus::Currency | Focus::Map => {
            let (primary, secondary, label) = world_map_markers(app, context);
            let title = if context == Focus::Map {
                format!("World map ({})", configured_map_summary(app))
            } else {
                format!("World map ({})", label)
            };
            frame.render_widget(
                WorldMapCanvas::new()
                    .primary(primary)
                    .secondary(secondary)
                    .title(title)
                    .tick(app.animation_frame as u64)
                    .focused(app.focus == Focus::Map),
                area,
            );
        }
    }
}

fn world_marker_for_city(city: &City) -> Option<WorldMarker> {
    let (lat, lon) = city_coords_by_code(&city.code).or_else(|| city_coords_by_name(&city.name))?;
    Some(WorldMarker {
        label: city.code.clone(),
        lat,
        lon,
    })
}

fn world_marker_for_country_code(code: &str) -> Option<WorldMarker> {
    country_by_code(code).map(|country| WorldMarker {
        label: country.code.to_string(),
        lat: country.lat,
        lon: country.lon,
    })
}

fn world_marker_for_currency(currency: &str) -> Option<WorldMarker> {
    let country_code = focal_country_code_for_currency(currency)?;
    world_marker_for_country_code(country_code)
}

fn configured_world_map_markers(
    app: &App,
) -> (Option<WorldMarker>, Option<WorldMarker>, &'static str) {
    let map = app.config.effective_map_settings();
    let first_target_city = app
        .config
        .effective_target_city_codes()
        .first()
        .and_then(|code| app.city_by_code(code))
        .and_then(world_marker_for_city);
    let focal_country = map
        .focal_country_code
        .as_deref()
        .and_then(world_marker_for_country_code);
    let extra_country = map
        .focus_country_codes
        .first()
        .map(String::as_str)
        .and_then(world_marker_for_country_code);

    (focal_country, extra_country.or(first_target_city), "Countries")
}

fn configured_map_summary(app: &App) -> String {
    let map = app.config.effective_map_settings();
    if !map.enabled {
        return "disabled".to_string();
    }

    let focal_country = map
        .focal_country_code
        .as_deref()
        .map(|code| {
            country_by_code(code)
                .map(|country| country.code)
                .unwrap_or(code)
                .to_string()
        })
        .unwrap_or_else(|| "?".to_string());

    format!("countries {}", focal_country)
}

fn world_map_markers(
    app: &App,
    context: Focus,
) -> (Option<WorldMarker>, Option<WorldMarker>, &'static str) {
    let (mut primary, mut secondary, label) = match context {
        Focus::Currency => (
            world_marker_for_currency(&app.currency_converter.from_currency),
            world_marker_for_currency(&app.currency_converter.to_currency),
            "Currency",
        ),
        Focus::TimeConvert => (
            app.city_by_code(&app.time_converter.from_city_code)
                .and_then(world_marker_for_city),
            app.city_by_code(&app.time_converter.to_city_code)
                .and_then(world_marker_for_city),
            "Time",
        ),
        Focus::Map => configured_world_map_markers(app),
        Focus::Weather => (
            app.city_by_code(&app.time_converter.from_city_code)
                .and_then(world_marker_for_city),
            app.city_by_code(&app.time_converter.to_city_code)
                .and_then(world_marker_for_city),
            "Time",
        ),
    };

    if primary.is_none() {
        primary = world_marker_for_city(&app.config.current_city);
    }
    if secondary.is_none() && app.config.effective_map_settings().enabled {
        secondary = world_marker_for_city(&app.config.home_city);
    }

    (primary, secondary, label)
}

/// draw weather panel with current conditions and forecast-style layout (compact view)
fn draw_weather_panel(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Weather;
    let block = styled_block("Weather [s:view] [space:city]", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    draw_weather_detail(frame, inner, app);
}

/// draw detailed weather information (wttr-style with high density)
fn draw_weather_detail(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 4 || area.width < 20 {
        return;
    }

    // use the selected weather city, not config city
    let city_name = app.get_weather_city_name();
    let city_code = app.get_weather_city_code();
    let city_count = NZ_CITIES.len();
    let city_index = app.weather_city_index + 1;

    match &app.current_weather {
        Some(w) => {
            let mut lines = vec![];

            // row 1: city selector with navigation hint
            let day_night = if w.is_day { "☀" } else { "☾" };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", city_code),
                    Style::default().fg(catppuccin::SAPPHIRE),
                ),
                Span::styled(
                    city_name,
                    Style::default()
                        .fg(catppuccin::PEACH)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}", day_night),
                    Style::default().fg(if w.is_day {
                        catppuccin::YELLOW
                    } else {
                        catppuccin::LAVENDER
                    }),
                ),
                Span::styled(
                    format!(" [{}/{}]", city_index, city_count),
                    Style::default().fg(catppuccin::OVERLAY0),
                ),
            ]));

            // row 2: big temperature with prominent emoji
            let icon = w.icon.icon(w.is_day);
            let icon_color = match w.icon {
                crate::weather::WeatherIcon::Sunny => catppuccin::YELLOW,
                crate::weather::WeatherIcon::PartlyCloudy => catppuccin::PEACH,
                crate::weather::WeatherIcon::Cloudy => catppuccin::OVERLAY1,
                crate::weather::WeatherIcon::Rain | crate::weather::WeatherIcon::HeavyRain => {
                    catppuccin::BLUE
                }
                crate::weather::WeatherIcon::Drizzle => catppuccin::SAPPHIRE,
                crate::weather::WeatherIcon::Snow => catppuccin::TEXT,
                crate::weather::WeatherIcon::Thunderstorm => catppuccin::MAUVE,
                crate::weather::WeatherIcon::Fog => catppuccin::OVERLAY0,
                crate::weather::WeatherIcon::Unknown => catppuccin::SUBTEXT0,
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", icon),
                    Style::default().fg(icon_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}  ", w.temp_string()),
                    Style::default()
                        .fg(catppuccin::GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("feels {}", w.feels_like_string()),
                    Theme::text_dim(),
                ),
            ]));

            // row 3: condition description with emoji
            let condition_emoji = match w.icon {
                crate::weather::WeatherIcon::Sunny => {
                    if w.is_day {
                        "☀️"
                    } else {
                        "🌙"
                    }
                }
                crate::weather::WeatherIcon::PartlyCloudy => "⛅",
                crate::weather::WeatherIcon::Cloudy => "☁️",
                crate::weather::WeatherIcon::Rain | crate::weather::WeatherIcon::HeavyRain => "🌧️",
                crate::weather::WeatherIcon::Drizzle => "🌦️",
                crate::weather::WeatherIcon::Snow => "❄️",
                crate::weather::WeatherIcon::Thunderstorm => "⛈️",
                crate::weather::WeatherIcon::Fog => "🌫️",
                crate::weather::WeatherIcon::Unknown => "❓",
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", condition_emoji), Style::default()),
                Span::styled(&w.description, Style::default().fg(catppuccin::SUBTEXT1)),
            ]));

            // row 4: wind - crucial for NZ!
            let wind_arrow = match w.wind_dir.as_str() {
                "N" => "↓",
                "NE" => "↙",
                "E" => "←",
                "SE" => "↖",
                "S" => "↑",
                "SW" => "↗",
                "W" => "→",
                "NW" => "↘",
                _ => "○",
            };
            let wind_strength = if w.wind_kmph >= 50 {
                ("💨", catppuccin::RED, " STRONG")
            } else if w.wind_kmph >= 30 {
                ("💨", catppuccin::PEACH, " gusty")
            } else if w.wind_kmph >= 15 {
                ("🌬️", catppuccin::SAPPHIRE, "")
            } else {
                ("🍃", catppuccin::GREEN, " calm")
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", wind_strength.0),
                    Style::default().fg(wind_strength.1),
                ),
                Span::styled(
                    format!("{} km/h", w.wind_kmph),
                    Style::default()
                        .fg(wind_strength.1)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {} {}", wind_arrow, w.wind_dir),
                    Style::default().fg(catppuccin::SUBTEXT1),
                ),
                Span::styled(wind_strength.2, Style::default().fg(wind_strength.1)),
            ]));

            // row 5: humidity
            lines.push(Line::from(vec![
                Span::styled("  💧 ", Style::default().fg(catppuccin::SAPPHIRE)),
                Span::styled(format!("{}% humidity", w.humidity), Theme::text()),
            ]));

            // 3-day forecast with wind
            if !w.forecast.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "  ─── 3-Day Forecast ───",
                    Style::default().fg(catppuccin::SURFACE2),
                )]));
                for day in w.forecast.iter().take(3) {
                    let day_icon = day.icon.icon(true);
                    let wind_indicator = if day.wind_max >= 40 {
                        "💨"
                    } else if day.wind_max >= 20 {
                        "🌬️"
                    } else {
                        "🍃"
                    };
                    // format date as short (e.g., "Dec 10")
                    let short_date = if day.date.len() >= 10 {
                        format!("{}-{}", &day.date[5..7], &day.date[8..10])
                    } else {
                        day.date.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {} ", day_icon),
                            Style::default().fg(catppuccin::YELLOW),
                        ),
                        Span::styled(
                            format!("{} ", short_date),
                            Style::default().fg(catppuccin::SUBTEXT0),
                        ),
                        Span::styled(
                            format!("{:>2}/{:<2}°C ", day.temp_max, day.temp_min),
                            Style::default().fg(catppuccin::GREEN),
                        ),
                        Span::styled(
                            format!("{}{:>2}km/h", wind_indicator, day.wind_max),
                            Style::default().fg(catppuccin::SAPPHIRE),
                        ),
                    ]));
                }
            }

            // navigation hint and source on same line to save space
            let is_stale_or_offline = w.is_stale() || app.weather_error.is_some();
            let source_tag = if is_stale_or_offline {
                " [stale/offline]"
            } else {
                " [live]"
            };
            let source_tag_style = if is_stale_or_offline {
                Style::default().fg(catppuccin::YELLOW)
            } else {
                Style::default().fg(catppuccin::GREEN)
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("Open-Meteo", Style::default().fg(catppuccin::SAPPHIRE)),
                Span::styled(source_tag, source_tag_style),
            ]));

            let para = Paragraph::new(lines).wrap(Wrap { trim: false });
            frame.render_widget(para, area);
        }
        None => {
            // check if we have an error (offline) or just loading
            let mut lines = vec![];

            // city header
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", city_code),
                    Style::default().fg(catppuccin::SAPPHIRE),
                ),
                Span::styled(city_name, Theme::text_highlight()),
                Span::styled(
                    format!(" [{}/{}]", city_index, city_count),
                    Style::default().fg(catppuccin::OVERLAY0),
                ),
            ]));

            lines.push(Line::from(""));

            if let Some(error) = &app.weather_error {
                // offline / error state
                lines.push(Line::from(vec![
                    Span::styled("  ⚠ ", Style::default().fg(catppuccin::YELLOW)),
                    Span::styled(
                        "OFFLINE",
                        Style::default()
                            .fg(catppuccin::RED)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "  No weather data available",
                    Theme::text_muted(),
                )]));
                lines.push(Line::from(vec![Span::styled(
                    format!("  Error: {}", error.chars().take(60).collect::<String>()),
                    Theme::text_dim(),
                )]));
            } else {
                // loading state
                lines.push(Line::from(vec![
                    Span::styled("    ⟳ ", Style::default().fg(catppuccin::SAPPHIRE)),
                    Span::styled("Loading weather...", Theme::text_muted()),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("Source: ", Theme::text_muted()),
                Span::styled("Open-Meteo.com", Style::default().fg(catppuccin::SAPPHIRE)),
            ]));

            let para = Paragraph::new(lines);
            frame.render_widget(para, area);
        }
    }
}

/// get ASCII art for weather condition (wttr-style, 5 lines)
#[allow(dead_code)]
fn weather_ascii_art(icon: crate::weather::WeatherIcon, is_day: bool) -> [&'static str; 5] {
    match (icon, is_day) {
        (crate::weather::WeatherIcon::Sunny, true) => [
            r"   \  /  ",
            r"    .-.  ",
            r" - (   ) -",
            r"    `-'  ",
            r"   /  \  ",
        ],
        (crate::weather::WeatherIcon::Sunny, false)
        | (crate::weather::WeatherIcon::PartlyCloudy, false) => [
            "         ",
            "  .--.   ",
            " (    )  ",
            "  `--'   ",
            "   *     ",
        ],
        (crate::weather::WeatherIcon::PartlyCloudy, true) => [
            r"   \  /  ",
            "  _ .--.  ",
            " _ (    ) ",
            "   (___)) ",
            "         ",
        ],
        (crate::weather::WeatherIcon::Cloudy, _) => [
            "         ",
            "   .--.  ",
            "  (    ) ",
            " (____(__)",
            "         ",
        ],
        (crate::weather::WeatherIcon::Rain, _) | (crate::weather::WeatherIcon::HeavyRain, _) => [
            "   .--.  ",
            "  (    ) ",
            " (____(__)",
            "  ' ' ' ' ",
            " ' ' ' '  ",
        ],
        (crate::weather::WeatherIcon::Drizzle, _) => [
            "   .--.  ",
            "  (    ) ",
            " (____(__)",
            "   ' '   ",
            "  ' '    ",
        ],
        (crate::weather::WeatherIcon::Snow, _) => [
            "   .--.  ",
            "  (    ) ",
            " (____(__)",
            "  * * * * ",
            " * * * *  ",
        ],
        (crate::weather::WeatherIcon::Thunderstorm, _) => [
            "   .--.  ",
            "  (    ) ",
            " (____(__)",
            "  ⚡' '⚡ ",
            " ' ' ' '  ",
        ],
        (crate::weather::WeatherIcon::Fog, _) => [
            "         ",
            " _ _ _ _ ",
            "  _ _ _  ",
            " _ _ _ _ ",
            "         ",
        ],
        (crate::weather::WeatherIcon::Unknown, _) => [
            "         ",
            "   .-.   ",
            "    ?    ",
            "   .-.   ",
            "         ",
        ],
    }
}

/// get wind direction arrow
fn wind_arrow(dir: &str) -> &'static str {
    match dir {
        "N" => "↓",
        "NNE" | "NE" => "↙",
        "ENE" | "E" => "←",
        "ESE" | "SE" => "↖",
        "SSE" | "S" => "↑",
        "SSW" | "SW" => "↗",
        "WSW" | "W" => "→",
        "WNW" | "NW" | "NNW" => "↘",
        _ => "○",
    }
}

/// format description for wttr cell (truncated)
fn wttr_desc(icon: crate::weather::WeatherIcon) -> &'static str {
    match icon {
        crate::weather::WeatherIcon::Sunny => "Sunny",
        crate::weather::WeatherIcon::PartlyCloudy => "Pt cldy",
        crate::weather::WeatherIcon::Cloudy => "Cloudy",
        crate::weather::WeatherIcon::Fog => "Fog",
        crate::weather::WeatherIcon::Drizzle => "Drizzle",
        crate::weather::WeatherIcon::Rain => "Rain",
        crate::weather::WeatherIcon::HeavyRain => "Heavy rain",
        crate::weather::WeatherIcon::Snow => "Snow",
        crate::weather::WeatherIcon::Thunderstorm => "Thunder",
        crate::weather::WeatherIcon::Unknown => "Unknown",
    }
}

/// pad icon to a target display width (handles wide emoji)
fn pad_icon(icon: &str, target: usize) -> String {
    let width = UnicodeWidthStr::width(icon);
    if width >= target {
        icon.to_string()
    } else {
        let padding = " ".repeat(target.saturating_sub(width));
        format!("{}{}", icon, padding)
    }
}

/// centre text using display width for emoji-safe alignment
fn center_pad(content: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(content);
    if w >= width {
        content.to_string()
    } else {
        let total = width - w;
        let left = total / 2;
        let right = total - left;
        format!("{}{}{}", " ".repeat(left), content, " ".repeat(right))
    }
}

fn push_grid_line(lines: &mut Vec<Line<'static>>, padding: usize, spans: Vec<Span<'static>>) {
    let mut padded = Vec::with_capacity(spans.len() + 1);
    if padding > 0 {
        padded.push(Span::raw(" ".repeat(padding)));
    }
    padded.extend(spans);
    lines.push(Line::from(padded));
}

/// draw weather panel with wttr-style 3-day grid
fn draw_weather_panel_expanded(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Weather;
    let block = styled_block("Weather [s:view] [space:city]", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 || inner.width < 40 {
        return;
    }

    let city_name = app.get_weather_city_name();
    let city_code = app.get_weather_city_code();
    let city_count = NZ_CITIES.len();
    let city_index = app.weather_city_index + 1;

    match &app.current_weather {
        Some(w) => {
            let mut lines: Vec<Line> = vec![];
            let border = Style::default().fg(catppuccin::SURFACE2);
            let grid_width: u16 = 57;
            let is_stale_or_offline = w.is_stale() || app.weather_error.is_some();
            let grid_padding = if inner.width > grid_width {
                ((inner.width - grid_width) / 2) as usize
            } else {
                0
            };

            // current conditions header with ASCII art (wttr style)
            let current_art = weather_ascii_art(w.icon, w.is_day);
            let arrow = wind_arrow(&w.wind_dir);

            // row 0: description + city
            lines.push(Line::from(vec![
                Span::styled(current_art[0], Style::default().fg(catppuccin::YELLOW)),
                Span::styled(
                    format!("  {} ", wttr_desc(w.icon)),
                    Style::default().fg(catppuccin::TEXT),
                ),
                Span::styled(
                    format!("[{}/{}]", city_index, city_count),
                    Style::default().fg(catppuccin::OVERLAY0),
                ),
            ]));

            // row 1: art + temp + city
            lines.push(Line::from(vec![
                Span::styled(current_art[1], Style::default().fg(catppuccin::YELLOW)),
                Span::styled(
                    format!("  {} ", w.temp_string()),
                    Style::default()
                        .fg(catppuccin::GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} {}", city_code, city_name),
                    Style::default().fg(catppuccin::PEACH),
                ),
            ]));

            // row 2: art + wind
            let wind_color = if w.wind_kmph >= 40 {
                catppuccin::RED
            } else if w.wind_kmph >= 25 {
                catppuccin::YELLOW
            } else {
                catppuccin::GREEN
            };
            lines.push(Line::from(vec![
                Span::styled(current_art[2], Style::default().fg(catppuccin::YELLOW)),
                Span::styled(
                    format!("  {} {} km/h", arrow, w.wind_kmph),
                    Style::default().fg(wind_color),
                ),
            ]));

            // row 3: art + visibility
            lines.push(Line::from(vec![
                Span::styled(current_art[3], Style::default().fg(catppuccin::YELLOW)),
                Span::styled("  10 km", Style::default().fg(catppuccin::SUBTEXT0)),
            ]));

            // row 4: art + humidity
            lines.push(Line::from(vec![
                Span::styled(current_art[4], Style::default().fg(catppuccin::YELLOW)),
                Span::styled(
                    format!("  {}% humidity", w.humidity),
                    Style::default().fg(catppuccin::SUBTEXT0),
                ),
            ]));

            // blank line before grid
            lines.push(Line::from(""));

            // wttr-style grid with day headers
            use crate::weather::TimeOfDay;
            let period_order = [
                TimeOfDay::Morning,
                TimeOfDay::Noon,
                TimeOfDay::Evening,
                TimeOfDay::Night,
            ];

            for day in w.forecast.iter().take(3) {
                // format day header (centred)
                let day_header = if day.date.len() >= 10 {
                    let month = &day.date[5..7];
                    let dom = &day.date[8..10];
                    let day_name = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d")
                        .map(|date| date.format("%a").to_string())
                        .unwrap_or_else(|_| "???".to_string());
                    format!("{} {} {}", day_name, dom, month_name(month))
                } else {
                    day.date.clone()
                };

                // day header row (centred above columns)
                // keep widths consistent with 4×13-char columns + 5 pipes (57 total)
                push_grid_line(
                    &mut lines,
                    grid_padding,
                    vec![
                        Span::styled("┌", border),
                        Span::styled(
                            format!("{:─^55}", format!(" {} ", day_header)),
                            Style::default().fg(catppuccin::TEXT),
                        ),
                        Span::styled("┐", border),
                    ],
                );

                // column headers
                push_grid_line(
                    &mut lines,
                    grid_padding,
                    vec![
                        Span::styled("│", border),
                        Span::styled(
                            "   Morning   ",
                            Style::default()
                                .fg(catppuccin::PEACH)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("│", border),
                        Span::styled(
                            "    Noon     ",
                            Style::default()
                                .fg(catppuccin::YELLOW)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("│", border),
                        Span::styled(
                            "   Evening   ",
                            Style::default()
                                .fg(catppuccin::MAUVE)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("│", border),
                        Span::styled(
                            "    Night    ",
                            Style::default()
                                .fg(catppuccin::LAVENDER)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("│", border),
                    ],
                );

                // separator
                push_grid_line(
                    &mut lines,
                    grid_padding,
                    vec![Span::styled(
                        "├─────────────┼─────────────┼─────────────┼─────────────┤",
                        border,
                    )],
                );

                // content row: icon + description
                let mut desc_spans = vec![Span::styled("│", border)];
                for target in &period_order {
                    if let Some(p) = day.periods.iter().find(|p| {
                        std::mem::discriminant(&p.period) == std::mem::discriminant(target)
                    }) {
                        let is_day = matches!(target, TimeOfDay::Morning | TimeOfDay::Noon);
                        let icon = p.icon.icon(is_day);
                        let desc = wttr_desc(p.icon);
                        // pad icon to align with text columns
                        let icon_padded = pad_icon(icon, 2);
                        let cell = format!("{} {}", icon_padded, &desc[..desc.len().min(9)]);
                        desc_spans.push(Span::styled(
                            center_pad(&cell, 13),
                            Style::default().fg(catppuccin::TEXT),
                        ));
                    } else {
                        desc_spans.push(Span::styled(center_pad("--", 13), Theme::text_muted()));
                    }
                    desc_spans.push(Span::styled("│", border));
                }
                push_grid_line(&mut lines, grid_padding, desc_spans);

                // content row: temp
                let mut temp_spans = vec![Span::styled("│", border)];
                for target in &period_order {
                    if let Some(p) = day.periods.iter().find(|p| {
                        std::mem::discriminant(&p.period) == std::mem::discriminant(target)
                    }) {
                        let temp_color = if p.temp >= 25 {
                            catppuccin::RED
                        } else if p.temp >= 18 {
                            catppuccin::YELLOW
                        } else if p.temp >= 10 {
                            catppuccin::GREEN
                        } else {
                            catppuccin::SAPPHIRE
                        };
                        temp_spans.push(Span::styled(
                            center_pad(&format!("{} °C", p.temp), 13),
                            Style::default().fg(temp_color),
                        ));
                    } else {
                        temp_spans.push(Span::styled(center_pad("--", 13), Theme::text_muted()));
                    }
                    temp_spans.push(Span::styled("│", border));
                }
                push_grid_line(&mut lines, grid_padding, temp_spans);

                // content row: wind
                let mut wind_spans = vec![Span::styled("│", border)];
                for target in &period_order {
                    if let Some(p) = day.periods.iter().find(|p| {
                        std::mem::discriminant(&p.period) == std::mem::discriminant(target)
                    }) {
                        let wind_color = if p.wind >= 40 {
                            catppuccin::RED
                        } else if p.wind >= 25 {
                            catppuccin::YELLOW
                        } else {
                            catppuccin::GREEN
                        };
                        let wind_arrow = wind_arrow(&p.wind_dir);
                        wind_spans.push(Span::styled(
                            center_pad(&format!("{} {} km/h", wind_arrow, p.wind), 13),
                            Style::default().fg(wind_color),
                        ));
                    } else {
                        wind_spans.push(Span::styled(center_pad("--", 13), Theme::text_muted()));
                    }
                    wind_spans.push(Span::styled("│", border));
                }
                push_grid_line(&mut lines, grid_padding, wind_spans);

                // bottom of day section
                let bottom = Span::styled(
                    "└─────────────┴─────────────┴─────────────┴─────────────┘",
                    border,
                );
                push_grid_line(&mut lines, grid_padding, vec![bottom]);
            }

            // source
            let source_tag = if is_stale_or_offline {
                " [stale/offline]"
            } else {
                " [live]"
            };
            let source_tag_style = if is_stale_or_offline {
                Style::default().fg(catppuccin::YELLOW)
            } else {
                Style::default().fg(catppuccin::GREEN)
            };
            lines.push(Line::from(vec![
                Span::styled("Open-Meteo.com", Style::default().fg(catppuccin::SAPPHIRE)),
                Span::styled(source_tag, source_tag_style),
            ]));

            let para = Paragraph::new(lines);
            frame.render_widget(para, inner);
        }
        None => {
            // show loading or error state
            let mut lines = vec![];
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} {} ", city_code, city_name),
                    Style::default().fg(catppuccin::SAPPHIRE),
                ),
                Span::styled(
                    format!("[{}/{}]", city_index, city_count),
                    Style::default().fg(catppuccin::OVERLAY0),
                ),
            ]));
            lines.push(Line::from(""));

            if let Some(error) = &app.weather_error {
                lines.push(Line::from(vec![
                    Span::styled("  ⚠ OFFLINE - ", Style::default().fg(catppuccin::RED)),
                    Span::styled(
                        error.chars().take(40).collect::<String>(),
                        Theme::text_muted(),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![Span::styled(
                    "  ⟳ Loading weather data...",
                    Theme::text_muted(),
                )]));
            }

            let para = Paragraph::new(lines);
            frame.render_widget(para, inner);
        }
    }
}

/// convert month number to short name
fn month_name(month: &str) -> &'static str {
    match month {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => "???",
    }
}

/// draw time panel - simplified NZ → overseas city
fn draw_time_panel(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::TimeConvert;
    let block = styled_block("Time [space:city] [s:swap] [e:edit/Esc]", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    let mut lines = vec![];

    // NZ city (anchor) - always Wellington
    if let Some(ct) = &app.current_city_time {
        let time_str = ct.time_string(true, false);
        let day = if ct.is_daytime() { "☀" } else { "☾" };
        let day_color = if ct.is_daytime() {
            catppuccin::YELLOW
        } else {
            catppuccin::LAVENDER
        };

        lines.push(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(catppuccin::GREEN)),
            Span::styled(
                format!("{:<3}", ct.city_code),
                Style::default().fg(catppuccin::SAPPHIRE),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(
                format!("{:<12}", ct.city_name),
                Style::default()
                    .fg(catppuccin::PEACH)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", time_str),
                Style::default()
                    .fg(catppuccin::GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(day, Style::default().fg(day_color)),
        ]));
    }

    // overseas city (cycles with spacebar - uses converter's to_city)
    let to_city_code = &app.time_converter.to_city_code;
    let overseas_time = app
        .world_city_times
        .iter()
        .find(|ct| &ct.city_code == to_city_code)
        .or(app.home_city_time.as_ref());

    if let Some(ht) = overseas_time {
        let time_str = ht.time_string(true, false);
        let day = if ht.is_daytime() { "☀" } else { "☾" };
        let day_color = if ht.is_daytime() {
            catppuccin::YELLOW
        } else {
            catppuccin::LAVENDER
        };

        let delta = if let Some(ct) = &app.current_city_time {
            format_time_delta(ct, ht)
        } else {
            String::new()
        };

        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{:<3}", ht.city_code),
                Style::default().fg(catppuccin::OVERLAY1),
            ),
            Span::styled(" ", Style::default()),
            Span::styled(
                format!("{:<12}", ht.city_name),
                Style::default().fg(catppuccin::SUBTEXT0),
            ),
            Span::styled(
                format!(" {} ", time_str),
                Style::default().fg(catppuccin::TEXT),
            ),
            Span::styled(day, Style::default().fg(day_color)),
            Span::styled(
                format!(" {}", delta),
                Style::default().fg(catppuccin::OVERLAY1),
            ),
        ]));
    }

    // blank line
    lines.push(Line::from(""));

    // time converter section
    let converter = &app.time_converter;
    let from_name = app.get_time_convert_from_name();
    let to_name = app.get_time_convert_to_name();

    let input_display = if converter.is_typing() {
        converter.format_input_display()
    } else {
        converter.format_input_time()
    };
    let result_style = if converter.invalid_input {
        Style::default()
            .fg(catppuccin::RED)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(catppuccin::GREEN)
            .add_modifier(Modifier::BOLD)
    };

    lines.push(Line::from(vec![Span::styled(
        " ─ Convert ─",
        Style::default().fg(catppuccin::SURFACE2),
    )]));

    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", input_display),
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} → ", from_name.chars().take(6).collect::<String>()),
            Style::default().fg(catppuccin::SUBTEXT1),
        ),
        Span::styled(format!("{} ", converter.format_result_time()), result_style),
        Span::styled(
            to_name.chars().take(6).collect::<String>(),
            Style::default().fg(catppuccin::SUBTEXT1),
        ),
    ]));

    // hint for controls
    if focused {
        lines.push(Line::from(vec![Span::styled(
            " [0-9]:time [Esc]:exit",
            Style::default().fg(catppuccin::OVERLAY0),
        )]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);

    if app.input_mode == InputMode::EditingTime {
        draw_editing_indicator(frame, area);
    }
}

/// format a city time line with optional marker
#[allow(dead_code)]
fn format_city_time_line(
    ct: &CityTime,
    marker: &str,
    highlight: bool,
    _tick: usize,
) -> Line<'static> {
    let time_str = ct.time_string(true, false);
    let day_indicator = if ct.is_daytime() { "☀" } else { "☾" };
    let day_color = if ct.is_daytime() {
        catppuccin::YELLOW
    } else {
        catppuccin::LAVENDER
    };

    let name_style = if highlight {
        Style::default()
            .fg(catppuccin::PEACH)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(catppuccin::SUBTEXT1)
    };

    Line::from(vec![
        Span::styled(
            format!("{} ", marker),
            Style::default().fg(catppuccin::GREEN),
        ),
        Span::styled(
            format!("{:<3}", ct.city_code),
            Style::default().fg(catppuccin::SAPPHIRE),
        ),
        Span::styled(" │ ", Style::default().fg(catppuccin::SURFACE2)),
        Span::styled(
            format!("{:<12}", ct.city_name.chars().take(12).collect::<String>()),
            name_style,
        ),
        Span::styled(
            format!(" {} ", time_str),
            Style::default()
                .fg(catppuccin::GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(day_indicator, Style::default().fg(day_color)),
    ])
}

/// format a city time line with time delta
#[allow(dead_code)]
fn format_city_time_line_with_delta(
    ct: &CityTime,
    marker: &str,
    _highlight: bool,
    delta: &str,
) -> Line<'static> {
    let time_str = ct.time_string(true, false);
    let day_indicator = if ct.is_daytime() { "☀" } else { "☾" };
    let day_color = if ct.is_daytime() {
        catppuccin::YELLOW
    } else {
        catppuccin::LAVENDER
    };

    Line::from(vec![
        Span::styled(
            format!("{} ", marker),
            Style::default().fg(catppuccin::OVERLAY0),
        ),
        Span::styled(
            format!("{:<3}", ct.city_code),
            Style::default().fg(catppuccin::OVERLAY1),
        ),
        Span::styled(" │ ", Style::default().fg(catppuccin::SURFACE2)),
        Span::styled(
            format!("{:<12}", ct.city_name.chars().take(12).collect::<String>()),
            Style::default().fg(catppuccin::SUBTEXT0),
        ),
        Span::styled(
            format!(" {} ", time_str),
            Style::default().fg(catppuccin::TEXT),
        ),
        Span::styled(day_indicator, Style::default().fg(day_color)),
        Span::styled(
            format!(" {}", delta),
            Style::default().fg(catppuccin::OVERLAY1),
        ),
    ])
}

/// format time delta between two cities
fn format_time_delta(from: &CityTime, to: &CityTime) -> String {
    let diff_hours = to.offset_hours - from.offset_hours;
    let hours = diff_hours.abs() as i32;
    let mins = ((diff_hours.abs() - hours as f32) * 60.0) as i32;

    let direction = if diff_hours > 0.0 { "ahead" } else { "behind" };

    if mins == 0 {
        format!("{:+}h {}", diff_hours as i32, direction)
    } else {
        format!(
            "{:+}h {:02}m {}",
            hours * diff_hours.signum() as i32,
            mins,
            direction
        )
    }
}

/// draw compact time converter
#[allow(dead_code)]
fn draw_time_converter_compact(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 {
        return;
    }

    let converter = &app.time_converter;
    let from_name = app.get_time_convert_from_name();
    let to_name = app.get_time_convert_to_name();

    // show typing indicator if actively entering time
    let input_display = if converter.is_typing() {
        converter.format_input_display()
    } else {
        converter.format_input_time()
    };
    let result_style = if converter.invalid_input {
        Style::default()
            .fg(catppuccin::RED)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(catppuccin::GREEN)
            .add_modifier(Modifier::BOLD)
    };

    let mut lines = vec![];

    // separator line
    lines.push(Line::from(vec![Span::styled(
        "  ─── Convert ───",
        Style::default().fg(catppuccin::SURFACE2),
    )]));

    // conversion line
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} ", input_display),
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<8}", from_name.chars().take(8).collect::<String>()),
            Style::default().fg(catppuccin::SUBTEXT1),
        ),
        Span::styled(" → ", Style::default().fg(catppuccin::OVERLAY1)),
        Span::styled(format!("{} ", converter.format_result_time()), result_style),
        Span::styled(
            to_name.chars().take(8).collect::<String>(),
            Style::default().fg(catppuccin::SUBTEXT1),
        ),
    ]));

    // help text
    if area.height > 3 {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[0-9]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" type ", Theme::text_muted()),
            Span::styled("[jk]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" hr ", Theme::text_muted()),
            Span::styled("[hl]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" min ", Theme::text_muted()),
            Span::styled("[s]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" swap", Theme::text_muted()),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);

    // editing indicator
    if app.input_mode == InputMode::EditingTime {
        draw_editing_indicator(frame, area);
    }
}

/// draw currency panel with bidirectional conversion
fn draw_currency_panel(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Currency;
    let block = styled_block("Currency [space:cycle] [s:swap] [e:edit/Esc]", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    draw_currency_detail(frame, inner, app);
}

/// draw simplified currency conversion - linked to time cities
fn draw_currency_detail(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 15 {
        return;
    }

    let converter = &app.currency_converter;
    let mut lines = vec![];

    // from amount and currency
    lines.push(Line::from(vec![
        Span::styled(
            format!("{:>8.2} ", converter.from_amount),
            Style::default()
                .fg(catppuccin::PEACH)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &converter.from_currency,
            Style::default().fg(catppuccin::SAPPHIRE),
        ),
    ]));

    // rate info
    let is_live = app.is_online && converter.rate.is_some();
    let rate_display = if let Some(r) = converter.rate {
        format!(
            "1 {} = {:.4} {}",
            converter.from_currency, r, converter.to_currency
        )
    } else if app.is_online {
        "loading...".to_string()
    } else {
        "rate unavailable (offline, no cache)".to_string()
    };

    lines.push(Line::from(vec![
        Span::styled("    ↓ ", Style::default().fg(catppuccin::OVERLAY1)),
        Span::styled(rate_display, Style::default().fg(catppuccin::OVERLAY0)),
    ]));

    // to amount and currency
    lines.push(Line::from(vec![
        Span::styled(
            format!("{:>8.2} ", converter.to_amount),
            Style::default()
                .fg(catppuccin::GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &converter.to_currency,
            Style::default().fg(catppuccin::SAPPHIRE),
        ),
    ]));

    // reverse rate
    if let Some(rate) = converter.rate
        && rate > 0.0
    {
        lines.push(Line::from(vec![Span::styled(
            format!(
                "1 {} ≈ {:.2} {}",
                converter.to_currency,
                1.0 / rate,
                converter.from_currency
            ),
            Theme::text_muted(),
        )]));
    }

    // source with live indicator
    lines.push(Line::from(vec![
        Span::styled(
            "exchangerate-api",
            Style::default().fg(catppuccin::SAPPHIRE),
        ),
        if is_live {
            Span::styled(" [live]", Style::default().fg(catppuccin::GREEN))
        } else if converter.rate.is_some() {
            Span::styled(" [cache]", Style::default().fg(catppuccin::YELLOW))
        } else {
            Span::styled("", Style::default())
        },
    ]));

    // controls hint when focused
    if app.focus == Focus::Currency {
        lines.push(Line::from(vec![Span::styled(
            "[0-9]:amt [Esc]:exit",
            Style::default().fg(catppuccin::OVERLAY0),
        )]));
    }

    // help text
    if area.height > 10 && app.focus == Focus::Currency {
        lines.push(Line::from(vec![
            Span::styled(" [0-9]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" amt ", Theme::text_muted()),
            Span::styled("[s]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" swap ", Theme::text_muted()),
            Span::styled("[c]", Style::default().fg(catppuccin::OVERLAY1)),
            Span::styled(" pair", Theme::text_muted()),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);

    // editing indicator
    if app.input_mode == InputMode::EditingCurrency {
        draw_editing_indicator(frame, area);
    }
}

/// draw footer with city codes and help hint
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(catppuccin::SURFACE1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // if typing a command, show command buffer
    if !app.command_buffer.is_empty() {
        let cmd_line = Line::from(vec![
            Span::styled(&app.command_buffer, Style::default().fg(catppuccin::YELLOW)),
            Span::styled("█", Style::default().fg(catppuccin::TEXT)),
        ]);
        frame.render_widget(Paragraph::new(cmd_line), inner);
        return;
    }

    // single row: contextual info (or status) on left, help hint on right
    let left_content = match app.focus {
        Focus::Currency => {
            let converter = &app.currency_converter;
            let rate_line = if let Some(rate) = converter.rate {
                format!(
                    "1 {} = {:.4} {}",
                    converter.from_currency, rate, converter.to_currency
                )
            } else {
                format!(
                    "{} → {} (rate pending)",
                    converter.from_currency, converter.to_currency
                )
            };
            Line::from(vec![
                Span::styled(" FX: ", Style::default().fg(catppuccin::PEACH)),
                Span::styled(rate_line, Style::default().fg(catppuccin::OVERLAY1)),
            ])
        }
        Focus::TimeConvert => {
            let converter = &app.time_converter;
            let from = &converter.from_city_code;
            let to = &converter.to_city_code;
            let input = converter.format_input_time();
            let result = converter.format_result_time();
            Line::from(vec![
                Span::styled(" Time: ", Style::default().fg(catppuccin::GREEN)),
                Span::styled(
                    format!("{} {} → {} {}", from, input, to, result),
                    Style::default().fg(catppuccin::OVERLAY1),
                ),
            ])
        }
        Focus::Map => Line::from(vec![
            Span::styled(" Map: ", Style::default().fg(catppuccin::PEACH)),
            Span::styled(
                configured_map_summary(app),
                Style::default().fg(catppuccin::OVERLAY1),
            ),
        ]),
        _ => {
            if let Some((message, _)) = &app.status_message {
                Line::from(vec![
                    Span::styled(" ℹ ", Style::default().fg(catppuccin::SAPPHIRE)),
                    Span::styled(message, Theme::text_dim()),
                ])
            } else if app.has_config_draft() {
                Line::from(vec![
                    Span::styled(" Draft: ", Style::default().fg(catppuccin::PEACH)),
                    Span::styled(
                        "/apply /discard /reset /restore",
                        Style::default().fg(catppuccin::OVERLAY1),
                    ),
                ])
            } else {
                // show NZ city codes
                let codes: String = NZ_CITIES
                    .iter()
                    .map(|c| c.code)
                    .collect::<Vec<_>>()
                    .join(" · ");
                Line::from(vec![
                    Span::styled(" NZ: ", Style::default().fg(catppuccin::GREEN)),
                    Span::styled(codes, Style::default().fg(catppuccin::OVERLAY1)),
                ])
            }
        }
    };

    // help hint for right side (margo style)
    let help_hint = Line::from(vec![Span::styled(
        "/help ",
        Style::default().fg(catppuccin::OVERLAY0),
    )]);

    // split horizontally
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(10)])
        .split(inner);

    frame.render_widget(Paragraph::new(left_content), cols[0]);
    frame.render_widget(
        Paragraph::new(help_hint).alignment(Alignment::Right),
        cols[1],
    );
}

/// draw editing indicator overlay
fn draw_editing_indicator(frame: &mut Frame, area: Rect) {
    if area.height < 1 || area.width < 10 {
        return;
    }

    let indicator = Paragraph::new(Line::from(vec![
        Span::styled("▸ ", Style::default().fg(catppuccin::GREEN)),
        Span::styled(
            "editing",
            Style::default()
                .fg(catppuccin::GREEN)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Right);

    let indicator_area = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width.saturating_sub(1),
        1,
    );

    frame.render_widget(indicator, indicator_area);
}
