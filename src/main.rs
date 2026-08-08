mod config;
mod vim;
mod mpd;
mod tui;
mod ui;

use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::config::Config;
use crate::tui::TuiGuard;
use crate::ui::{App, ActiveView};

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut terminal = TuiGuard::init()?;
    let mut app = App::new(config);

    app.connect_mpd();

    loop {
        terminal.draw(|f| app.draw(f))?;
        app.tick();

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Char('q')) => break,
                        (KeyModifiers::CONTROL, KeyCode::Enter) => {
                            app.trigger_database_update();
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
                    }
                }
                _ => {}
            }
        }
    }

    TuiGuard::restore()?;
    Ok(())
}
