use clap::Parser;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lidb_tui_prototype::{
    app::{App, Tab},
    events::{AppEvent, EventHandler},
    model::gpu_ai::{GpuAiTelemetry, GpuViewMode},
    theme::ThemeMode,
    ui,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, panic, time::Duration};

#[derive(Parser, Debug)]
#[command(
    name = "lidb-tui-prototype",
    about = "Modular terminal-native TUI prototype for LiDashBoard (LiDB)",
    version = "0.1.0"
)]
struct Args {
    /// Refresh interval in milliseconds
    #[arg(short, long, default_value_t = 500)]
    tick_rate: u64,

    /// Initial color theme (dark, light, high-contrast, monochrome)
    #[arg(long, default_value = "dark")]
    theme: String,

    /// GPU mode: 'simulated' (Grace Hopper / GB10) or 'host' (PRoot unsupported state)
    #[arg(long, default_value = "host")]
    gpu_mode: String,

    /// Run non-interactive smoke test and exit cleanly (for CI and container validation)
    #[arg(long)]
    smoke_test: bool,

    /// Run full headless render test of all tabs using Ratatui TestBackend and exit
    #[arg(long)]
    headless_test: bool,
}

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        original_hook(panic_info);
    }));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. Headless test mode
    if args.headless_test {
        println!("Running headless Ratatui TestBackend validation for all tabs...");
        let backend = ratatui::backend::TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new();

        for tab in Tab::ALL {
            app.current_tab = tab;
            terminal.draw(|f| ui::draw(f, &app))?;
        }
        println!("Headless render validation successful: all 7 tabs rendered without panic.");
        return Ok(());
    }

    // 2. Smoke test mode (for automated CI/container runs without TTY)
    if args.smoke_test {
        println!("Running smoke test (5 simulated telemetry ticks)...");
        let mut app = App::new();
        for _ in 0..5 {
            app.on_tick();
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("Smoke test passed: state updated successfully.");
        return Ok(());
    }

    // 3. Interactive TUI mode
    let theme_mode = match args.theme.to_lowercase().as_str() {
        "light" => ThemeMode::Light,
        "high-contrast" | "contrast" => ThemeMode::HighContrast,
        "monochrome" | "mono" | "no-color" => ThemeMode::Monochrome,
        _ => ThemeMode::Dark,
    };

    let mut app = App::new();
    app.settings.refresh_rate_ms = args.tick_rate;
    app.settings.theme_mode = theme_mode;
    app.theme = lidb_tui_prototype::theme::Theme::new(theme_mode);

    if args.gpu_mode.to_lowercase() == "simulated" {
        app.gpu_ai = GpuAiTelemetry::simulated_fixture();
    } else {
        app.gpu_ai = GpuAiTelemetry::default();
        app.gpu_ai.view_mode = GpuViewMode::HostReality;
    }

    setup_panic_hook();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut events = EventHandler::new(args.tick_rate);

    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, &app))?;

        match events.next_event()? {
            AppEvent::Key(key) => {
                app.handle_key(key);
            }
            AppEvent::Tick => {
                app.on_tick();
            }
            AppEvent::Resize(_, _) => {}
        }
    }

    // Graceful teardown
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()?;

    Ok(())
}
