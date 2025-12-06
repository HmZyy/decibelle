use std::{sync::mpsc, thread::JoinHandle};

use crossterm::event::{self, Event, KeyEventKind};

use crate::events::types::AppEvent;

pub fn spawn(event_tx: mpsc::Sender<AppEvent>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.kind == KeyEventKind::Press {
                        if event_tx.send(AppEvent::Input(key_event)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Event::Resize(width, height)) => {
                    if event_tx.send(AppEvent::Resize(width, height)).is_err() {
                        break;
                    }
                }
                Ok(_) => {
                    // Ignore mouse events, focus events, paste events, etc.
                }
                Err(_) => {
                    break;
                }
            }
        }
    })
}
