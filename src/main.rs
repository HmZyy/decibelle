use std::{io, sync::mpsc, time::Duration};

use crate::{
    api::thread::ApiCommand,
    app::state::App,
    events::types::AppEvent,
    player::commands::PlayerCommand,
    ui::cover::{CoverFetcher, CoverMessage, ImageCache},
};

mod api;
mod app;
mod config;
mod events;
mod input;
mod player;
mod ui;

fn main() -> io::Result<()> {
    let config = config::load_or_create_config();
    let config = match config {
        Ok(config) => config,
        Err(err) => {
            panic!("Failed to load config: {}", err);
        }
    };

    ui::theme::init_theme(config.theme);

    let mut terminal = ratatui::init();
    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();
    let (player_cmd_tx, player_cmd_rx) = mpsc::channel::<PlayerCommand>();
    let (api_cmd_tx, api_cmd_rx) = mpsc::channel::<ApiCommand>();

    let _input_handle = input::thread::spawn(event_tx.clone());
    let _player_handle = player::thread::spawn(player_cmd_rx, event_tx.clone());
    let _api_handle = api::thread::spawn(config.clone(), api_cmd_rx, event_tx.clone());

    let mut app = App::new(player_cmd_tx, api_cmd_tx);
    app.load_libraries();

    let mut image_cache = ImageCache::new();
    let cover_fetcher = CoverFetcher::new(config.clone());
    let mut last_item_id: Option<String> = None;

    loop {
        while let Ok(msg) = cover_fetcher.try_recv() {
            match msg {
                CoverMessage::Loaded { item_id, data } => {
                    if let Err(e) = image_cache.load_cover(&item_id, &data) {
                        eprintln!("Failed to load cover: {}", e);
                    }
                }
                CoverMessage::Error { item_id, error } => {
                    eprintln!("Cover fetch error for {}: {}", item_id, error);
                }
            }
        }

        if let Some(ref item) = app.current_library_item {
            let current_id = &item.id;
            if last_item_id.as_ref() != Some(current_id) {
                cover_fetcher.fetch(current_id.clone());
                last_item_id = Some(current_id.clone());
            }
        } else if last_item_id.is_some() {
            last_item_id = None;
            image_cache.clear();
        }

        terminal.draw(|f| ui::render::render(f, &app, &mut image_cache))?;

        match event_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => match event {
                AppEvent::Input(key_event) => app.handle_input(key_event),
                AppEvent::Resize(_width, _height) => {}
                AppEvent::PlayerStateChanged(state) => app.on_player_state_changed(state),
                AppEvent::PositionUpdate(pos) => app.on_position_update(pos),
                AppEvent::DurationChanged(dur) => app.on_duration_changed(dur),
                AppEvent::TrackEnded => app.on_track_ended(),
                AppEvent::PlayerError(e) => app.on_player_error(e),
                AppEvent::LibrariesLoaded(libraries) => app.on_libraries_loaded(libraries),
                AppEvent::ItemsLoaded(items) => app.on_items_loaded(items),
                AppEvent::ChaptersLoaded(chapters) => app.on_chapters_loaded(chapters),
                AppEvent::DownloadFinished(path, position, track_info) => {
                    app.on_download_finished(path, position, track_info)
                }
                AppEvent::ContinueListeningLoaded(item, position) => {
                    app.on_continue_listening_loaded(item, position)
                }
                AppEvent::ApiError(err) => app.on_api_error(err),
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("Event channel disconnected");
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
