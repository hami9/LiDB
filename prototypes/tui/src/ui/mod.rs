pub mod common;
pub mod modals;
pub mod tabs;

use crate::app::{App, Modal, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Responsive guard for very small terminals
    if size.width < 50 || size.height < 12 {
        render_compact_fallback(f, size, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top header bar
            Constraint::Length(2), // Tab bar
            Constraint::Min(8),    // Active tab contents
            Constraint::Length(2), // Footer keybindings
        ])
        .split(size);

    common::render_header(f, chunks[0], app);
    common::render_tabs_bar(f, chunks[1], app);

    match app.current_tab {
        Tab::System => tabs::system::render(f, chunks[2], app),
        Tab::CpuMemory => tabs::cpu_mem::render(f, chunks[2], app),
        Tab::Networking => tabs::network::render(f, chunks[2], app),
        Tab::GpuAi => tabs::gpu_ai::render(f, chunks[2], app),
        Tab::Processes => tabs::processes::render(f, chunks[2], app),
        Tab::Diagnostics => tabs::diagnostics::render(f, chunks[2], app),
        Tab::SettingsHelp => tabs::settings_help::render(f, chunks[2], app),
    }

    common::render_footer(f, chunks[3], app);

    // Render active modal on top
    match &app.active_modal {
        Modal::Help => {
            modals::help_modal::render(f, size, &app.theme);
        }
        Modal::ProcessDetail(proc) => {
            modals::detail_modal::render_process_detail(f, size, proc, &app.theme);
        }
        Modal::DiagnosticDetail(check) => {
            modals::detail_modal::render_diagnostic_detail(f, size, check, &app.theme);
        }
        Modal::None => {}
    }
}

fn render_compact_fallback(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let lines = vec![
        Line::from(vec![Span::styled(
            " LiDB - Terminal Window Small ",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Current: ", Style::default().fg(theme.fg_muted)),
            Span::styled(
                format!("{}x{}", area.width, area.height),
                Style::default().fg(theme.accent),
            ),
            Span::styled(" (Minimum: 50x12)", Style::default().fg(theme.fg_muted)),
        ]),
        Line::from(" Please enlarge your terminal window."),
        Line::from(" Press 'q' to quit."),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.block_border_style(false));
    f.render_widget(Paragraph::new(lines).block(block), area);
}
