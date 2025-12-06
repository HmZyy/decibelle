use crate::api::client::ApiClient;
use crate::api::models::{AudioTrack, find_track_for_position};
use crate::events::types::{AppEvent, TrackInfo};
use std::sync::mpsc;

pub enum ApiCommand {
    FetchLibraries,
    FetchLibraryItems(String),
    FetchItemChapters(String),
    DownloadForPlayback(String, f64),
    FetchContinueListening(String),
}

pub fn spawn(
    config: crate::config::Config,
    cmd_rx: mpsc::Receiver<ApiCommand>,
    event_tx: mpsc::Sender<AppEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let client = ApiClient::new(&config);

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                ApiCommand::FetchLibraries => match client.get_libraries() {
                    Ok(libs) => {
                        let _ = event_tx.send(AppEvent::LibrariesLoaded(libs));
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::ApiError(format!("{:?}", e)));
                    }
                },
                ApiCommand::FetchLibraryItems(library_id) => {
                    match client.get_library_items(&library_id) {
                        Ok(items) => {
                            let _ = event_tx.send(AppEvent::ItemsLoaded(items));
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::ApiError(format!("{:?}", e)));
                        }
                    }
                }
                ApiCommand::FetchItemChapters(item_id) => {
                    match client.get_item_chapters(&item_id) {
                        Ok(chapters) => {
                            let _ = event_tx.send(AppEvent::ChaptersLoaded(chapters));
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::ApiError(format!("{:?}", e)));
                        }
                    }
                }

                ApiCommand::DownloadForPlayback(item_id, position) => {
                    match client.get_library_item(&item_id) {
                        Ok(item) => {
                            let tracks = item.media.as_ref().and_then(|m| m.tracks.as_ref());

                            match tracks {
                                Some(tracks) if !tracks.is_empty() => {
                                    let track = find_track_for_position(tracks, position)
                                        .or_else(|| tracks.first());

                                    if let Some(track) = track {
                                        let track_local_position =
                                            (position - track.start_offset).max(0.0);

                                        match client.download_track(&item_id, track) {
                                            Ok(path) => {
                                                let _ = event_tx.send(AppEvent::DownloadFinished(
                                                    path,
                                                    track_local_position,
                                                    TrackInfo {
                                                        index: track.index,
                                                        start_offset: track.start_offset,
                                                        duration: track.duration,
                                                    },
                                                ));
                                            }
                                            Err(e) => {
                                                let _ = event_tx
                                                    .send(AppEvent::ApiError(format!("{:?}", e)));
                                            }
                                        }
                                    }
                                }
                                _ => match client.download_audio(&item_id) {
                                    Ok(path) => {
                                        let _ = event_tx.send(AppEvent::DownloadFinished(
                                            path,
                                            position,
                                            TrackInfo::single_file(),
                                        ));
                                    }
                                    Err(e) => {
                                        let _ =
                                            event_tx.send(AppEvent::ApiError(format!("{:?}", e)));
                                    }
                                },
                            }
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::ApiError(format!("{:?}", e)));
                        }
                    }
                }

                ApiCommand::FetchContinueListening(library_id) => {
                    match client.get_continue_listening(&library_id) {
                        Ok(Some((item, position))) => {
                            let _ =
                                event_tx.send(AppEvent::ContinueListeningLoaded(item, position));
                        }
                        Ok(None) => {}
                        Err(e) => {
                            eprintln!("Continue listening error: {:?}", e);
                        }
                    }
                }
            }
        }
    })
}
