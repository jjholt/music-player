mod config;
mod mpd;
mod tui;
mod ui;
mod vim;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::config::Config;
use crate::mpd::MpdClient;
use crate::tui::TuiGuard;
use crate::ui::{ActiveView, App, Mode};

enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Tick,
}

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut terminal = TuiGuard::init()?;
    let mut app = App::new(config)
        .connect()
        .map_err(|e| anyhow::anyhow!("Failed to connect to MPD: {}", e))?;

    app.load_library().load_playlist();

    let (tx, rx) = mpsc::channel::<AppEvent>();
    let tick_tx = tx.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(250));
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    let input_tx = tx.clone();
    thread::spawn(move || {
        loop {
            if let Ok(Event::Key(key)) = event::read() {
                if input_tx.send(AppEvent::Key(key)).is_err() {
                    break;
                }
            }
        }
    });

    loop {
        terminal.draw(|f| {
            app.draw(f);
        })?;
        match rx.recv()? {
            AppEvent::Tick => {
                app.tick();
            }
            AppEvent::Key(key) => {
                if handle_global_key(&mut app, key) {
                    break;
                }
            }
        }
    }

    TuiGuard::restore()?;
    Ok(())
}

fn handle_global_key(app: &mut App<MpdClient>, key: crossterm::event::KeyEvent) -> bool {
    match app.mode {
        Mode::Input => {
            app.handle_key(key);
        }
        Mode::Normal => match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('q')) => return true,
            (KeyModifiers::CONTROL, KeyCode::Enter) => {
                app.trigger_database_update();
            }
            (KeyModifiers::NONE, KeyCode::Char('p')) => {
                app.toggle_pause();
            }
            (KeyModifiers::NONE, KeyCode::Char('>')) => {
                app.next();
            }
            (KeyModifiers::NONE, KeyCode::Char('<')) => {
                app.prev();
            }
            (KeyModifiers::NONE, KeyCode::Char('1')) => {
                app.set_view(ActiveView::Playlist);
            }
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                app.set_view(ActiveView::Browse);
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                app.set_view(ActiveView::Library);
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                app.seek_forward();
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                app.seek_backward();
            }
            _ => {
                app.handle_key(key);
            }
        },
    }
    false
}
