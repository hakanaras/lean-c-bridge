use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use super::{
    app::{App, FormItem, FormItemKind, View},
    preview::preview_lean_function,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(f.area());

    match app.view {
        View::FunctionList => render_function_list(f, app, main_chunks[0]),
        View::FunctionForm => render_form(f, app, main_chunks[0]),
    }

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(main_chunks[1]);

    render_preview(f, app, right_chunks[0]);
    render_keybindings(f, app, right_chunks[1]);
}

fn render_function_list(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = if app.list_search_active {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(4)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    };

    let items: Vec<ListItem> = app
        .functions
        .iter()
        .map(|func| {
            let configured = if app.function_has_choices(&func.name) {
                " [configured]"
            } else {
                ""
            };
            ListItem::new(format!("{}{}", func.name, configured))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Functions"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    state.select(Some(app.list_selected));

    f.render_stateful_widget(list, chunks[0], &mut state);

    if app.list_search_active {
        render_function_search(f, app, chunks[1]);
    }
}

fn render_function_search(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Find Function");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let before: String = app.list_search_buffer[..app.list_search_cursor]
        .iter()
        .collect();
    let cursor_ch = app
        .list_search_buffer
        .get(app.list_search_cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after: String = if app.list_search_cursor < app.list_search_buffer.len() {
        app.list_search_buffer[app.list_search_cursor + 1..]
            .iter()
            .collect()
    } else {
        String::new()
    };

    let query_line = Line::from(vec![
        Span::raw(" Query: "),
        Span::raw(before),
        Span::styled(
            cursor_ch,
            Style::default().bg(Color::White).fg(Color::Black),
        ),
        Span::raw(after),
    ]);

    let status = app
        .list_search_status
        .as_deref()
        .unwrap_or("Enter confirms the current match");
    let status_line = Line::from(vec![Span::styled(
        status,
        Style::default().fg(Color::DarkGray),
    )]);

    let paragraph = Paragraph::new(vec![query_line, status_line]);
    f.render_widget(paragraph, inner);
}

fn render_form(f: &mut Frame, app: &mut App, area: Rect) {
    let func_name = &app.functions[app.form_function_index].name;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Edit: {}", func_name));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;

    // Adjust scroll to keep focus visible
    if app.form_focus < app.form_scroll {
        app.form_scroll = app.form_focus;
    }
    if app.form_focus >= app.form_scroll + visible_height {
        app.form_scroll = app.form_focus - visible_height + 1;
    }

    let mut y = 0u16;
    for (i, item) in app.form_items.iter().enumerate() {
        if i < app.form_scroll {
            continue;
        }
        if y >= inner.height {
            break;
        }

        let is_focused = i == app.form_focus;
        let indent = item.indent * 2;
        let item_area = Rect {
            x: inner.x + indent,
            y: inner.y + y,
            width: inner.width.saturating_sub(indent),
            height: 1,
        };

        let is_editing = app.editing_text && is_focused;
        render_form_item(
            f,
            item,
            is_focused,
            item_area,
            is_editing,
            &app.text_buffer,
            app.text_cursor,
        );

        y += 1;
    }
}

fn render_form_item(
    f: &mut Frame,
    item: &FormItem,
    focused: bool,
    area: Rect,
    editing: bool,
    text_buffer: &[char],
    text_cursor: usize,
) {
    let focus_style = if focused {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };
    let disabled_style = Style::default().fg(Color::DarkGray);

    match &item.kind {
        FormItemKind::Header => {
            let label = format!("── {} ", item.label);
            let remaining = area.width.saturating_sub(label.len() as u16) as usize;
            let line = Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("─".repeat(remaining), Style::default().fg(Color::Yellow)),
            ]);
            f.render_widget(Paragraph::new(line), area);
        }
        FormItemKind::Checkbox { checked, enabled } => {
            let check = if *checked { "x" } else { " " };
            let style = if !enabled {
                disabled_style
            } else {
                focus_style
            };
            let prefix = if focused { "▸ " } else { "  " };
            let text = format!("{}[{}] {}", prefix, check, item.label);
            f.render_widget(Paragraph::new(text).style(style), area);
        }
        FormItemKind::Selector {
            options,
            selected,
            enabled,
        } => {
            let value = options.get(*selected).map(|s| s.as_str()).unwrap_or("?");
            let style = if !enabled {
                disabled_style
            } else {
                focus_style
            };
            let prefix = if focused { "▸ " } else { "  " };
            let text = format!("{}{}: ◀ {} ▶", prefix, item.label, value);
            f.render_widget(Paragraph::new(text).style(style), area);
        }
        FormItemKind::TextInput { value, enabled } => {
            let style = if !enabled {
                disabled_style
            } else {
                focus_style
            };
            let prefix = if focused { "▸ " } else { "  " };

            if editing {
                let before: String = text_buffer[..text_cursor].iter().collect();
                let cursor_ch = text_buffer
                    .get(text_cursor)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| " ".to_string());
                let after: String = if text_cursor < text_buffer.len() {
                    text_buffer[text_cursor + 1..].iter().collect()
                } else {
                    String::new()
                };

                let line = Line::from(vec![
                    Span::styled(format!("{}{}: ", prefix, item.label), style),
                    Span::styled(before, style),
                    Span::styled(
                        cursor_ch,
                        Style::default().bg(Color::White).fg(Color::Black),
                    ),
                    Span::styled(after, style),
                ]);
                f.render_widget(Paragraph::new(line), area);
            } else {
                let display_val = if value.is_empty() {
                    "(empty)"
                } else {
                    value.as_str()
                };
                let text = format!("{}{}: {}", prefix, item.label, display_val);
                f.render_widget(Paragraph::new(text).style(style), area);
            }
        }
    }
}

fn render_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Preview");
    let preview_lines = block.inner(area).height as usize;
    let preview = app
        .preview_target()
        .map(|(function, choices)| {
            preview_lean_function(&app.registry, function, &choices, preview_lines)
        })
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| "No function selected".to_string());

    let paragraph = Paragraph::new(preview)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn render_keybindings(f: &mut Frame, app: &App, area: Rect) {
    let bindings: Vec<(&str, &str)> = match app.view {
        View::FunctionList => {
            if app.list_search_active {
                vec![
                    ("Type", "Search by name"),
                    ("←/→", "Move cursor"),
                    ("Backspace", "Delete character"),
                    ("Enter", "Jump to match"),
                    ("Esc", "Cancel search"),
                ]
            } else {
                vec![
                    ("↑/↓", "Navigate"),
                    ("PgUp/PgDn", "Page navigate"),
                    ("Home/End", "Jump to start/end"),
                    ("F", "Find function"),
                    ("Enter", "Edit function"),
                    ("q/Esc", "Save & Quit"),
                ]
            }
        }
        View::FunctionForm => {
            if app.editing_text {
                vec![
                    ("Type", "Enter text"),
                    ("←/→", "Move cursor"),
                    ("Backspace", "Delete character"),
                    ("Enter/Esc", "Finish editing"),
                ]
            } else {
                vec![
                    ("↑/↓", "Navigate"),
                    ("Space/Enter", "Toggle / Edit"),
                    ("←/→", "Cycle option"),
                    ("Esc", "Save & back"),
                ]
            }
        }
    };

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!(" {:<14}", key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(*desc),
            ])
        })
        .collect();

    let block = Block::default().borders(Borders::ALL).title("Keybindings");
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
