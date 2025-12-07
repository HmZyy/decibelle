use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};
use ratatui_image::StatefulImage;

use crate::{
    api::models::{Chapter, LibraryItem},
    app::state::{App, Focus},
    player::commands::PlayerState,
    ui::{
        cover::ImageCache, format_duration, format_duration_long, format_size,
        notifications::Notification, theme::get_theme,
    },
};

const ROUNDED_BORDER: border::Set = border::ROUNDED;
const NOTIFICATION_WIDTH: u16 = 40;
const NOTIFICATION_HEIGHT: u16 = 3;

fn block_with_title(title: &'_ str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED_BORDER)
        .title(title)
}

pub fn render(f: &mut Frame, app: &mut App, image_cache: &mut ImageCache) {
    let theme = get_theme();
    let area = f.area();

    app.notifications.tick();

    let background = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(background, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(f, chunks[0]);
    draw_main_content(f, chunks[1], app, image_cache);
    draw_playback_controls(f, chunks[2], app);
    draw_footer(f, chunks[3], app);

    app.layout_regions.controls = Some(chunks[2]);

    draw_notifications(f, area, app.notifications.active_notifications());
}

fn draw_notifications(f: &mut Frame, area: Rect, notifications: &[Notification]) {
    let theme = get_theme();

    for (i, notif) in notifications.iter().rev().take(5).enumerate() {
        let y_offset = (i as u16) * (NOTIFICATION_HEIGHT + 1);

        if y_offset + NOTIFICATION_HEIGHT > area.height {
            break;
        }

        let notif_area = Rect {
            x: area.width.saturating_sub(NOTIFICATION_WIDTH + 2),
            y: area.y + 1 + y_offset,
            width: NOTIFICATION_WIDTH,
            height: NOTIFICATION_HEIGHT,
        };

        let color = theme.notification_color(notif.level);
        let prefix = notif.level.prefix();

        f.render_widget(Clear, notif_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(ROUNDED_BORDER)
            .border_style(Style::new().fg(color))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(notif_area);
        f.render_widget(block, notif_area);

        let max_text_len = (inner.width as usize).saturating_sub(prefix.len() + 3);
        let text = if notif.text.len() > max_text_len {
            format!("{}...", &notif.text[..max_text_len.saturating_sub(3)])
        } else {
            notif.text.clone()
        };

        let content = Line::from(vec![
            Span::styled(
                format!("{} - ", prefix),
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(text, Style::new().fg(theme.fg)),
        ]);

        f.render_widget(Paragraph::new(content), inner);
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let theme = get_theme();
    let header = Paragraph::new("Decibelle")
        .style(theme.header_style())
        .block(block_with_title(" 🎧 ").border_style(theme.border_style(false)))
        .centered();
    f.render_widget(header, area);
}

fn draw_main_content(f: &mut Frame, area: Rect, app: &mut App, image_cache: &mut ImageCache) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(main_chunks[0]);

    app.layout_regions.library_list = Some(top_chunks[0]);
    app.layout_regions.chapters = Some(top_chunks[1]);

    draw_library_list(f, top_chunks[0], app);
    draw_chapters(f, top_chunks[1], app);
    draw_now_playing(f, main_chunks[1], app, image_cache);
}

fn draw_library_list(f: &mut Frame, area: Rect, app: &App) {
    let theme = get_theme();
    let is_focused = app.focus == Focus::Libraries;
    let border_style = theme.border_style(is_focused);

    if app.libraries.len() > 0 {
        let selected_library = app.libraries[app.selected_library_index].clone();
        let title = format!(" ● {} ", selected_library.name);

        let items: Vec<ListItem> = app
            .library_items
            .clone()
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = i == app.selected_library_item_index;
                let prefix = if is_selected { "> " } else { "  " };
                let title = item
                    .media
                    .as_ref()
                    .and_then(|m| m.metadata.title.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("N/A");
                let text = format!("{}{}", prefix, title);
                let style = if is_focused && is_selected {
                    theme.selection_style()
                } else {
                    theme.value_style()
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(block_with_title(&title).border_style(border_style))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

        f.render_widget(list, area);
    } else {
        let block = block_with_title(" ● Libraries ").border_style(border_style);
        f.render_widget(block, area);

        let inner = Rect {
            x: area.x + 2,
            y: area.y + area.height / 2,
            width: area.width.saturating_sub(4),
            height: 1,
        };
        let no_library = Paragraph::new("No library loaded")
            .alignment(Alignment::Center)
            .style(theme.label_style());
        f.render_widget(no_library, inner);
    }
}

fn draw_now_playing(f: &mut Frame, area: Rect, app: &mut App, image_cache: &mut ImageCache) {
    let theme = get_theme();
    let is_focused = app.focus == Focus::InfoPanel;
    let block = block_with_title(" ● Now Playing ").border_style(theme.border_style(is_focused));
    let inner = block.inner(area);
    f.render_widget(block, area);

    app.layout_regions.info_panel = Some(area);

    match (&app.current_library_item, &app.current_chapter) {
        (Some(item), Some(chapter)) => {
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
                .split(inner);
            draw_info_panel(
                f,
                panels[0],
                item,
                Some(chapter),
                app.current_position.as_secs_f64(),
                &mut app.info_scroll,
                is_focused,
            );
            draw_thumbnail(f, panels[1], item, image_cache);
        }
        (Some(item), None) => {
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
                .split(inner);
            draw_info_panel(
                f,
                panels[0],
                item,
                None,
                0.0,
                &mut app.info_scroll,
                is_focused,
            );
            draw_thumbnail(f, panels[1], item, image_cache);
        }
        _ => {
            image_cache.clear();
            let center_y = inner.y + inner.height / 2;
            let text_area = Rect {
                x: inner.x,
                y: center_y,
                width: inner.width,
                height: 1,
            };
            let no_playback = Paragraph::new("No audiobook selected")
                .alignment(Alignment::Center)
                .style(theme.label_style());
            f.render_widget(no_playback, text_area);
        }
    }
}

fn draw_info_panel(
    f: &mut Frame,
    area: Rect,
    item: &LibraryItem,
    _chapter: Option<&Chapter>,
    _current_pos: f64,
    scroll: &mut u16,
    is_focused: bool,
) {
    let theme = get_theme();
    let media = match &item.media {
        Some(m) => m,
        None => {
            f.render_widget(
                Paragraph::new("No media information").style(theme.label_style()),
                area,
            );
            return;
        }
    };

    let metadata = &media.metadata;
    let label = theme.label_style();
    let value = theme.value_style();

    // Build all content lines
    let mut lines: Vec<Line> = Vec::new();

    // Title
    let title = metadata.title.as_deref().unwrap_or("N/A");
    lines.push(Line::from(vec![
        Span::styled("Title:     ", label),
        Span::styled(title, theme.title_style()),
    ]));

    // Subtitle
    let subtitle = metadata.subtitle.as_deref().unwrap_or("-");
    lines.push(Line::from(vec![
        Span::styled("Subtitle:  ", label),
        Span::styled(subtitle, Style::new().fg(theme.accent)),
    ]));

    // Author
    let author = metadata.author_name.as_deref().unwrap_or("Unknown");
    lines.push(Line::from(vec![
        Span::styled("Author:    ", label),
        Span::styled(author, value),
    ]));

    // Narrator
    let narrator = metadata.narrator_name.as_deref().unwrap_or("-");
    lines.push(Line::from(vec![
        Span::styled("Narrator:  ", label),
        Span::styled(narrator, value),
    ]));

    // Series
    let series = metadata.series_name.as_deref().unwrap_or("-");
    let series_display = if series.is_empty() { "-" } else { series };
    lines.push(Line::from(vec![
        Span::styled("Series:    ", label),
        Span::styled(series_display, Style::new().fg(theme.info)),
    ]));

    // Publisher
    let publisher = metadata.publisher.as_deref().unwrap_or("-");
    lines.push(Line::from(vec![
        Span::styled("Publisher: ", label),
        Span::styled(publisher, value),
    ]));

    // Year
    let year = metadata.published_year.as_deref().unwrap_or("-");
    lines.push(Line::from(vec![
        Span::styled("Year:      ", label),
        Span::styled(year, value),
    ]));

    // Spacer
    lines.push(Line::from(""));

    // Duration
    let duration = media.duration.unwrap_or(0.0);
    lines.push(Line::from(vec![
        Span::styled("Duration:  ", label),
        Span::styled(format_duration_long(duration), value),
    ]));

    // Size
    let size = media.size.unwrap_or(0);
    lines.push(Line::from(vec![
        Span::styled("Size:      ", label),
        Span::styled(format_size(size), value),
    ]));

    // Chapters / Tracks
    let num_chapters = media.num_chapters.unwrap_or(0);
    let num_tracks = media.num_tracks.unwrap_or(0);
    lines.push(Line::from(vec![
        Span::styled("Chapters:  ", label),
        Span::styled(format!("{}", num_chapters), value),
        Span::styled("  Tracks: ", label),
        Span::styled(format!("{}", num_tracks), value),
    ]));

    // Spacer
    lines.push(Line::from(""));

    if let Some(description) = &metadata.description {
        if !description.is_empty() {
            let plain_desc = description
                .replace("<br>", " ")
                .replace("<br/>", " ")
                .replace("<br />", " ")
                .replace("</p>", " ")
                .replace("<p>", "");
            let re_cleaned: String = plain_desc
                .chars()
                .fold((String::new(), false), |(mut acc, in_tag), c| {
                    if c == '<' {
                        (acc, true)
                    } else if c == '>' {
                        (acc, false)
                    } else if !in_tag {
                        acc.push(c);
                        (acc, false)
                    } else {
                        (acc, true)
                    }
                })
                .0;

            let desc_width = area.width.saturating_sub(4) as usize;
            let words: Vec<&str> = re_cleaned.trim().split_whitespace().collect();
            let mut current_line = String::new();

            for word in words {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_line.len() + 1 + word.len() <= desc_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    lines.push(Line::from(Span::styled(current_line.clone(), value)));
                    current_line = word.to_string();
                }
            }
            if !current_line.is_empty() {
                lines.push(Line::from(Span::styled(current_line, value)));
            }
        }
    }

    let total_lines = lines.len() as u16;
    let visible_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(visible_height);

    if *scroll > max_scroll {
        *scroll = max_scroll;
    }

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(3),
        height: visible_height,
    };
    let para = Paragraph::new(lines).scroll((*scroll, 0));
    f.render_widget(para, inner_area);

    if total_lines > visible_height {
        let scrollbar_area = Rect {
            x: area.x + area.width - 2,
            y: area.y + 1,
            width: 1,
            height: visible_height,
        };

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(*scroll as usize);

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

        if is_focused && max_scroll > 0 {
            let indicator = format!(" {}/{} ", *scroll + 1, max_scroll + 1);
            let indicator_area = Rect {
                x: area.x + area.width.saturating_sub(indicator.len() as u16 + 2),
                y: area.y,
                width: indicator.len() as u16,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(indicator).style(Style::new().fg(theme.accent)),
                indicator_area,
            );
        }
    }
}

fn draw_thumbnail(f: &mut Frame, area: Rect, item: &LibraryItem, image_cache: &mut ImageCache) {
    let theme = get_theme();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ROUNDED_BORDER)
        .border_style(Style::new().fg(theme.fg_dim))
        .style(Style::default().bg(theme.bg));

    let image_area = block.inner(area);
    f.render_widget(block, area);

    let bg_fill = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(bg_fill, image_area);

    if image_cache.current_item_id.as_deref() == Some(&item.id) {
        if let Some(ref mut protocol) = image_cache.current_image {
            let max_height = image_area.height;
            let max_width = image_area.width;

            let thumb_height = max_height.min(max_width / 2).max(1);
            let thumb_width = thumb_height * 2;

            let centered_area = Rect {
                x: image_area.x + (image_area.width.saturating_sub(thumb_width)) / 2,
                y: image_area.y + (image_area.height.saturating_sub(thumb_height)) / 2,
                width: thumb_width,
                height: thumb_height,
            };

            f.render_stateful_widget(StatefulImage::default(), centered_area, protocol);
            return;
        }
    }

    let text_area = Rect {
        x: image_area.x,
        y: image_area.y + image_area.height / 2,
        width: image_area.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new("Loading cover...")
            .alignment(Alignment::Center)
            .style(theme.label_style()),
        text_area,
    );
}

fn draw_chapters(f: &mut Frame, area: Rect, app: &App) {
    let theme = get_theme();
    let is_focused = app.focus == Focus::Chapters;
    let border_style = theme.border_style(is_focused);

    let is_current_item = app
        .current_item_id
        .as_ref()
        .zip(app.current_library_item.as_ref())
        .map(|(id, item)| id == &item.id)
        .unwrap_or(false);

    let items: Vec<ListItem> = app
        .chapters
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, chapter)| {
            let is_selected = i == app.selected_chapter_index;
            let is_current = is_current_item
                && app.current_position.as_secs_f64() >= chapter.start
                && app.current_position.as_secs_f64() < chapter.end;

            let prefix = if is_current {
                "▶ "
            } else if is_selected {
                "> "
            } else {
                "  "
            };

            let style = if is_current {
                theme.current_style()
            } else if is_focused && is_selected {
                theme.selection_style()
            } else {
                theme.value_style()
            };

            let duration_str = format_duration(chapter.end - chapter.start);
            let chapter_title = format!("{}{:02}. {}", prefix, i + 1, chapter.title);
            let padding = area
                .width
                .saturating_sub(duration_str.len() as u16 + chapter_title.len() as u16 + 4);

            ListItem::new(Line::from(vec![
                Span::styled(chapter_title, style),
                Span::styled(" ".repeat(padding as usize), style),
                Span::styled(duration_str, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block_with_title(" ● Chapters ").border_style(border_style))
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

    let mut list_state =
        ratatui::widgets::ListState::default().with_selected(Some(app.selected_chapter_index));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_playback_controls(f: &mut Frame, area: Rect, app: &App) {
    let theme = get_theme();
    let is_focused = app.focus == Focus::Controls;
    let border_style = theme.border_style(is_focused);

    let title = match &app.current_chapter {
        Some(ch) => format!(" ● Playing: {} ", ch.title),
        None => " ● Playback Controls ".to_string(),
    };

    let block = block_with_title(&title).border_style(border_style);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let (play_icon, play_label) = match app.player_state {
        PlayerState::Playing => ("󰏤", "Pause"),
        PlayerState::Paused => ("", "Resume"),
        _ => ("", "Play"),
    };

    let controls = Paragraph::new(format!(
        "󰒮 Prev   󰑟 -30s   {} {}   󰈑 +30s   󰒭 Next",
        play_icon, play_label
    ))
    .alignment(Alignment::Center)
    .style(theme.value_style());
    f.render_widget(controls, chunks[0]);

    let (chapter_start, chapter_duration) = match app.current_chapter.as_ref() {
        Some(ch) => (ch.start, ch.end - ch.start),
        None => (0.0, 0.0),
    };
    let chapter_position = (app.current_position.as_secs_f64() - chapter_start).max(0.0);
    let chapter_progress = if chapter_duration > 0.0 {
        (chapter_position / chapter_duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    draw_progress_bar(
        f,
        chunks[2],
        "Chapter:",
        chapter_position,
        chapter_duration,
        chapter_progress,
        app,
    );

    let book_duration = app
        .current_library_item
        .as_ref()
        .and_then(|item| item.media.as_ref())
        .and_then(|media| media.duration)
        .unwrap_or(0.0);
    let book_position = app.current_position.as_secs_f64();
    let book_progress = if book_duration > 0.0 {
        (book_position / book_duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    draw_progress_bar(
        f,
        chunks[3],
        "Book:",
        book_position,
        book_duration,
        book_progress,
        app,
    );
}

fn draw_progress_bar(
    f: &mut Frame,
    area: Rect,
    label: &str,
    current: f64,
    total: f64,
    progress: f64,
    app: &App,
) {
    let theme = get_theme();
    let progress_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(8),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(label).style(theme.label_style()),
        progress_chunks[0],
    );
    f.render_widget(
        Paragraph::new(format_duration(current))
            .alignment(Alignment::Right)
            .style(theme.value_style()),
        progress_chunks[1],
    );

    let slider_width = progress_chunks[3].width as usize;
    let filled = ((progress * slider_width as f64) as usize).min(slider_width);
    let is_playing = matches!(app.player_state, PlayerState::Playing);
    let slider_color = theme.slider_color(is_playing);

    let mut slider = String::new();
    for i in 0..slider_width {
        if i < filled.saturating_sub(1) {
            slider.push_str("━");
        } else if i == filled.saturating_sub(1) || (filled == 0 && i == 0) {
            slider.push_str("●");
        } else {
            slider.push_str("─");
        }
    }

    let slider_spans = vec![
        Span::styled(
            slider.chars().take(filled).collect::<String>(),
            Style::new().fg(slider_color),
        ),
        Span::styled(
            slider.chars().skip(filled).collect::<String>(),
            Style::new().fg(theme.fg_dim),
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(slider_spans)), progress_chunks[3]);

    f.render_widget(
        Paragraph::new(format_duration(total))
            .alignment(Alignment::Left)
            .style(theme.value_style()),
        progress_chunks[5],
    );
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let theme = get_theme();
    let keybinds = match app.focus {
        Focus::Libraries => {
            "↑↓/jk: Navigate | →/l/Enter: Select | L/H: Switch Library | Tab: Focus | Space: Pause | q: Quit"
        }
        Focus::Chapters => {
            "↑↓/jk: Navigate | ←/h: Back | Enter: Play Chapter | Tab: Focus | Space: Pause | q: Quit"
        }
        Focus::Controls => {
            "←→/hl: ±5s | ←→(global): ±30s | Space: Play/Pause | Tab: Focus | q: Quit"
        }
        Focus::InfoPanel => "↑↓/jk: Scroll | Tab: Focus | Space: Pause | q: Quit",
    };

    f.render_widget(
        Paragraph::new(keybinds)
            .style(theme.label_style())
            .block(block_with_title("").border_style(theme.border_style(false))),
        area,
    );
}
