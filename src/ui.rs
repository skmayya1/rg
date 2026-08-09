//! Ratatui rendering for the finder.

use std::{path::Path, sync::LazyLock};

use kamon::icon_and_color;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Style as SyntaxStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

use crate::app::App;

const MUTED_GRAY: Color = Color::DarkGray;
const FOCUSED_GRAY: Color = Color::Rgb(38, 38, 38);
const SYNTAX_THEME: &str = "base16-ocean.dark";

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_nonewlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

pub(crate) fn draw(frame: &mut ratatui::Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let prompt = Paragraph::new(Line::from(Span::raw(&app.query)))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(prompt, areas[0]);
    let query_width = u16::try_from(app.query.chars().count()).unwrap_or(u16::MAX);
    let cursor_x = areas[0]
        .x
        .saturating_add(1)
        .saturating_add(query_width)
        .min(areas[0].right().saturating_sub(2));
    frame.set_cursor_position((cursor_x, areas[0].y.saturating_add(1)));

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(areas[1]);

    let items = app.matches.iter().enumerate().map(|(index, result)| {
        let marker = if app.selected == Some(index) {
            "❯ "
        } else {
            "  "
        };
        let (icon, icon_color) = icon_and_color(&result.path);
        ListItem::new(Line::from(vec![
            Span::styled(marker, Style::default().fg(MUTED_GRAY)),
            Span::styled(
                format!("{icon} "),
                Style::default().fg(color_from_hex(icon_color).unwrap_or(MUTED_GRAY)),
            ),
            Span::styled(
                display_name(&result.path),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]))
    });
    let results = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .title(" files "),
        )
        .highlight_style(
            Style::default()
                .bg(FOCUSED_GRAY)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    let mut results_state = ListState::default();
    results_state.select(app.selected);
    frame.render_stateful_widget(results, columns[0], &mut results_state);

    let preview_lines = if let Some(message) = &app.preview.message {
        vec![Line::from(Span::styled(
            message,
            Style::default().fg(MUTED_GRAY),
        ))]
    } else {
        let mut highlighter = preview_highlighter(&app.preview.path);
        app.preview
            .lines
            .iter()
            .zip(app.preview.first_line..)
            .map(|(source, line_number)| {
                let is_focused = line_number == app.preview.focused_line;
                let line_number_style = if is_focused {
                    Style::default().bg(FOCUSED_GRAY).fg(Color::White)
                } else {
                    Style::default().fg(MUTED_GRAY)
                };
                let mut spans = vec![Span::styled(
                    format!("{line_number:>5}  "),
                    line_number_style,
                )];
                spans.extend(highlighted_spans(source, highlighter.as_mut(), is_focused));
                Line::from(spans)
            })
            .collect()
    };
    let preview_title = if app.preview.path.is_empty() {
        " preview ".to_owned()
    } else {
        format!(" preview · {} ", app.preview.path)
    };
    let preview = Paragraph::new(preview_lines).block(
        Block::default()
            .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
            .title(preview_title),
    );
    frame.render_widget(preview, columns[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(MUTED_GRAY)),
        Span::raw("navigate  "),
        Span::styled("Enter ", Style::default().fg(MUTED_GRAY)),
        Span::raw("copy file  "),
        Span::styled("y ", Style::default().fg(MUTED_GRAY)),
        Span::raw("copy location  "),
        Span::styled("Esc ", Style::default().fg(MUTED_GRAY)),
        Span::raw(format!("quit   ·   {}", app.status)),
    ]))
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, areas[2]);
}

fn display_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
}

fn preview_highlighter(path: &str) -> Option<HighlightLines<'static>> {
    let theme = THEME_SET
        .themes
        .get(SYNTAX_THEME)
        .or_else(|| THEME_SET.themes.values().next())?;
    Some(HighlightLines::new(syntax_for_path(Path::new(path)), theme))
}

fn syntax_for_path(path: &Path) -> &'static SyntaxReference {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| SYNTAX_SET.find_syntax_by_extension(extension))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}

fn highlighted_spans(
    source: &str,
    highlighter: Option<&mut HighlightLines<'static>>,
    is_focused: bool,
) -> Vec<Span<'static>> {
    let source = source.replace('\t', "  ");
    let Some(highlighter) = highlighter else {
        return vec![plain_source_span(source, is_focused)];
    };
    let Ok(ranges) = highlighter.highlight_line(&source, &SYNTAX_SET) else {
        return vec![plain_source_span(source, is_focused)];
    };

    ranges
        .into_iter()
        .map(|(style, text)| Span::styled(text.to_owned(), ratatui_style(style, is_focused)))
        .collect()
}

fn plain_source_span(source: String, is_focused: bool) -> Span<'static> {
    let style = if is_focused {
        Style::default().bg(FOCUSED_GRAY).fg(Color::White)
    } else {
        Style::default()
    };
    Span::styled(source, style)
}

fn ratatui_style(style: SyntaxStyle, is_focused: bool) -> Style {
    let mut ratatui_style = Style::default().fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ));
    if is_focused {
        ratatui_style = ratatui_style.bg(FOCUSED_GRAY);
    }
    if style.font_style.contains(FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }
    ratatui_style
}

fn color_from_hex(value: &str) -> Option<Color> {
    let value = value.strip_prefix('#')?;
    Some(Color::Rgb(
        u8::from_str_radix(value.get(0..2)?, 16).ok()?,
        u8::from_str_radix(value.get(2..4)?, 16).ok()?,
        u8::from_str_radix(value.get(4..6)?, 16).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_only_the_file_name() {
        assert_eq!(
            display_name("src/components/SearchPanel.tsx"),
            "SearchPanel.tsx"
        );
    }

    #[test]
    fn parses_file_icon_hex_colors() {
        assert_eq!(color_from_hex("#dea584"), Some(Color::Rgb(222, 165, 132)));
        assert_eq!(color_from_hex("not-a-color"), None);
    }

    #[test]
    fn selects_rust_syntax_from_a_file_extension() {
        assert_eq!(syntax_for_path(Path::new("main.rs")).name, "Rust");
    }
}
