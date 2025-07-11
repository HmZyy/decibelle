use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, FocusedPane};

pub fn render_ui(f: &mut Frame, app: &App) {
    if app.is_loading {
        render_loading_screen(f);
        return;
    }

    if let Some(error) = &app.error_message {
        render_error_screen(f, error);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)].as_ref())
        .split(f.size());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(main_chunks[0]);

    render_left_panel(f, app, chunks[0]);
    render_right_panel(f, app, chunks[1]);
    render_console_pane(f, app, main_chunks[1]);
}

fn render_loading_screen(f: &mut Frame) {
    let loading_text = vec![
        Line::from(""),
        Line::from("🔍 Scanning audiobooks..."),
        Line::from(""),
        Line::from("This may take a moment while we:"),
        Line::from("• Scan ~/Audiobooks directory"),
        Line::from("• Extract metadata with ffprobe"),
        Line::from("• Discover chapters"),
        Line::from(""),
        Line::from("Press 'q' to quit"),
    ];

    let loading_paragraph = Paragraph::new(loading_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📚 Decibelle - Audiobook Player")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .alignment(Alignment::Center);

    f.render_widget(loading_paragraph, f.size());
}

fn render_error_screen(f: &mut Frame, error: &str) {
    let error_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "⚠️ Error: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(error),
        ]),
        Line::from(""),
        Line::from("Troubleshooting:"),
        Line::from("• Make sure ~/Audiobooks directory exists"),
        Line::from("• Check that ffprobe is installed (part of ffmpeg)"),
        Line::from("• Verify audio files are in supported formats"),
        Line::from(""),
        Line::from("Press 'r' to retry or 'q' to quit"),
    ];

    let error_paragraph = Paragraph::new(error_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📚 Decibelle - Error")
                .border_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center);

    f.render_widget(error_paragraph, f.size());
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
    let books: Vec<ListItem> = if app.books.is_empty() {
        vec![ListItem::new(Line::from("No audiobooks found"))]
    } else {
        app.books
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
            .collect()
    };

    let border_color = if app.focused_pane == FocusedPane::BookList {
        Color::Magenta
    } else {
        Color::Blue
    };

    let title = format!("📚 Audiobooks ({})", app.books.len());
    let books_list = List::new(books)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(books_list, area, &mut {
        let mut state = ListState::default();
        if !app.books.is_empty() {
            state.select(Some(app.selected_book_index));
        }
        state
    });
}

fn render_chapter_list(f: &mut Frame, app: &App, area: Rect) {
    let chapters: Vec<ListItem> = if let Some(book) = app.get_current_book() {
        if book.chapters.is_empty() {
            vec![ListItem::new(Line::from("No chapters found"))]
        } else {
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
        }
    } else {
        vec![ListItem::new(Line::from("Select a book first"))]
    };

    let border_color = if app.focused_pane == FocusedPane::ChapterList {
        Color::Magenta
    } else {
        Color::Blue
    };

    let chapter_count = app
        .get_current_book()
        .map(|book| book.chapters.len())
        .unwrap_or(0);
    let title = format!("📖 Chapters ({})", chapter_count);

    let chapters_list = List::new(chapters)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(chapters_list, area, &mut {
        let mut state = ListState::default();
        if app.get_current_book().is_some() && chapter_count > 0 {
            state.select(Some(app.selected_chapter_index));
        }
        state
    });
}

fn render_right_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(6)].as_ref())
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
            Line::from(vec![
                Span::styled(
                    "Chapters: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(book.chapters.len().to_string()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Audio Files: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(app.current_audio_files.len().to_string()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Path: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(book.path.clone()),
            ]),
            Line::from(""),
        ];

        let mut all_lines = info_text;

        // Add description if it exists
        if !book.description.is_empty() {
            all_lines.push(Line::from(vec![Span::styled(
                "Description:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            all_lines.push(Line::from(""));

            let description_lines: Vec<Line> = book
                .description
                .split('\n')
                .map(|line| Line::from(line.to_string()))
                .collect();

            all_lines.extend(description_lines);
        }

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
        let empty_info = Paragraph::new(vec![
            Line::from("No book selected"),
            Line::from(""),
            Line::from("Navigate to a book and press Enter"),
            Line::from(""),
            Line::from("Controls:"),
            Line::from("• h/l: Move between panes"),
            Line::from("• j/k: Move up/down"),
            Line::from("• Enter: Select book/chapter"),
            Line::from("• Space: Play/pause"),
            Line::from("• r: Refresh library"),
            Line::from("• c: Clear console"),
            Line::from("• q: Quit"),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("ℹ️ Book Information")
                .border_style(Style::default().fg(border_color)),
        )
        .alignment(Alignment::Left);

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
}

fn render_console_pane(f: &mut Frame, app: &App, area: Rect) {
    let console_lines: Vec<Line> = app
        .console_messages
        .iter()
        .map(|msg| {
            let level_color = match msg.level.as_str() {
                "ERROR" => Color::Red,
                "WARN" => Color::Yellow,
                "INFO" => Color::Green,
                "DEBUG" => Color::Blue,
                _ => Color::White,
            };

            Line::from(vec![
                Span::styled(
                    format!("[{}]", msg.timestamp),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!(" {}: ", msg.level),
                    Style::default()
                        .fg(level_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(msg.message.clone()),
            ])
        })
        .collect();

    let console_paragraph = Paragraph::new(console_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🖥️ Console (Press 'c' to clear)")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(console_paragraph, area);
}
