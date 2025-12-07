use crate::events::types::AppEvent;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use std::io::stdout;
use std::{sync::mpsc, thread::JoinHandle};

pub fn spawn(event_tx: mpsc::Sender<AppEvent>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let _ = execute!(stdout(), EnableMouseCapture);

        loop {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.kind == KeyEventKind::Press {
                        if event_tx.send(AppEvent::Input(key_event)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Event::Mouse(mouse_event)) => {
                    if event_tx.send(AppEvent::Mouse(mouse_event)).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(width, height)) => {
                    if event_tx.send(AppEvent::Resize(width, height)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let _ = execute!(stdout(), DisableMouseCapture);
    })
}
