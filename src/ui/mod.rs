mod app;
mod preview;
mod render;

use std::io;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::clang::types::CFunction;
use crate::generator::TypeRegistry;
use crate::options::interface_choices::InterfaceChoices;

use app::App;

pub fn run(
    choices: InterfaceChoices,
    functions: Vec<CFunction>,
    registry: TypeRegistry,
) -> io::Result<InterfaceChoices> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(choices, functions, registry);

    loop {
        terminal.draw(|f| render::render(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.handle_key(key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(app.choices)
}
