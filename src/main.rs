mod config;
mod mpd;
mod tui;
mod ui;
mod vim;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

use crate::config::Config;
use crate::tui::TuiGuard;
use crate::ui::{ActiveView, App};

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut terminal = TuiGuard::init()?;
    let mut app = App::new(config)
        .connect()
        .map_err(|e| anyhow::anyhow!("Failed to connect to MPD: {}", e))?;

    app.load_library();

    loop {
        terminal.draw(|f| app.draw(f))?;
        app.tick();

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => match (key.modifiers, key.code) {
                    (KeyModifiers::NONE, KeyCode::Char('q')) => break,
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
                _ => {}
            }
        }
    }

    TuiGuard::restore()?;
    Ok(())
}
