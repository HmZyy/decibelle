use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, FocusedPane};

pub fn render_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(f.size());

    render_left_panel(f, app, chunks[0]);
    render_right_panel(f, app, chunks[1]);
}

fn render_left_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(area);

    render_book_list(f, app, chunks[0]);
    render_chapter_list(f, app, chunks[1]);
}

fn render_book_list(f: &mut Frame, app: &App, area: Rect) {
    let books: Vec<ListItem> = app
        .books
        .iter()
        .enumerate()
        .map(|(i, book)| {
            let style = if i == app.selected_book_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(book.title.clone(), style),
                Span::styled(
                    format!(" - {}", book.author),
                    Style::default().fg(Color::Gray),
                ),
            ]))
        })
        .collect();

    let border_color = if app.focused_pane == FocusedPane::BookList {
        Color::Magenta
    } else {
        Color::Blue
    };

    let books_list = List::new(books)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📚 Audiobooks")
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(books_list, area, &mut {
        let mut state = ListState::default();
        state.select(Some(app.selected_book_index));
        state
    });
}

fn render_chapter_list(f: &mut Frame, app: &App, area: Rect) {
    let chapters: Vec<ListItem> = if let Some(book) = app.get_current_book() {
        book.chapters
            .iter()
            .enumerate()
            .map(|(i, chapter)| {
                let style = if i == app.selected_chapter_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(chapter.clone(), style)))
            })
            .collect()
    } else {
        vec![]
    };

    let border_color = if app.focused_pane == FocusedPane::ChapterList {
        Color::Magenta
    } else {
        Color::Blue
    };

    let chapters_list = List::new(chapters)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📖 Chapters")
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(chapters_list, area, &mut {
        let mut state = ListState::default();
        state.select(Some(app.selected_chapter_index));
        state
    });
}

fn render_right_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(area);

    render_book_info(f, app, chunks[0]);
    render_audio_controls(f, app, chunks[1]);
}

fn render_book_info(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focused_pane == FocusedPane::BookInfo {
        Color::Magenta
    } else {
        Color::Blue
    };

    if let Some(book) = app.get_current_book() {
        let info_text = vec![
            Line::from(vec![
                Span::styled(
                    "Title: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(book.title.clone()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Author: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(book.author.clone()),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        let mut all_lines = info_text;

        // Add wrapped description lines
        let description_lines: Vec<Line> = book
            .description
            .split('\n')
            .map(|line| Line::from(line.to_string()))
            .collect();

        all_lines.extend(description_lines);

        let info_paragraph = Paragraph::new(all_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("ℹ️ Book Information")
                    .border_style(Style::default().fg(border_color)),
            )
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

        f.render_widget(info_paragraph, area);
    } else {
        let empty_info = Paragraph::new("No book selected")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("ℹ️ Book Information")
                    .border_style(Style::default().fg(border_color)),
            )
            .alignment(Alignment::Center);

        f.render_widget(empty_info, area);
    }
}

fn render_audio_controls(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focused_pane == FocusedPane::AudioControls {
        Color::Magenta
    } else {
        Color::Blue
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Controls
                Constraint::Length(3), // Progress bar
                Constraint::Min(1),    // Status
            ]
            .as_ref(),
        )
        .split(area);

    // Control buttons
    let play_pause_text = if app.is_playing {
        "⏸️  Pause"
    } else {
        "▶️  Play"
    };
    let current_chapter = app.get_current_chapter().unwrap_or("No chapter selected");

    let controls_text = vec![Line::from(vec![
        Span::styled("⏮️ Prev ", Style::default().fg(Color::White)),
        Span::styled(
            play_pause_text,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ⏭️ Next", Style::default().fg(Color::White)),
    ])];

    let controls = Paragraph::new(controls_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🎵 Audio Controls")
                .border_style(Style::default().fg(border_color)),
        )
        .alignment(Alignment::Center);

    f.render_widget(controls, chunks[0]);

    // Progress bar
    let progress_bar = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .percent((app.progress * 100.0) as u16)
        .label(format!("{} / {}", app.current_time, app.total_time));

    f.render_widget(progress_bar, chunks[1]);

    // Current chapter info
    let status_text = vec![Line::from(vec![
        Span::styled("Now Playing: ", Style::default().fg(Color::Cyan)),
        Span::raw(current_chapter.to_string()),
    ])];

    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(Alignment::Left);

    f.render_widget(status, chunks[2]);
}
