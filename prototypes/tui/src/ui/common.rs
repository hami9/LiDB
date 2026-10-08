use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(28), // Title & Agent
            Constraint::Min(20),    // Center status banner
            Constraint::Length(26), // Badge & pause status
        ])
        .split(area);

    // Left: Brand & agent badge
    let brand_text = vec![Line::from(vec![
        Span::styled(
            " LiDB ",
            Style::default()
                .bg(theme.accent)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "LiDashBoard",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" v{}", app.system.lidb_version),
            Style::default().fg(theme.fg_muted),
        ),
    ])];
    f.render_widget(Paragraph::new(brand_text), chunks[0]);

    // Center: Environment notice (PRoot notice or active node)
    let center_msg = if app.system.proot_detected {
        Span::styled(
            "⚙ PRoot/Termux Container [Rule R05/R10 Sandbox]",
            Style::default().fg(theme.info),
        )
    } else {
        Span::styled("⚙ Baremetal/Host Node", Style::default().fg(theme.success))
    };
    let center_p = Paragraph::new(Line::from(vec![center_msg])).alignment(Alignment::Center);
    f.render_widget(center_p, chunks[1]);

    // Right: Status badge & pause state
    let mut right_spans = Vec::new();
    if app.settings.paused {
        right_spans.push(Span::styled(
            " [PAUSED] ",
            Style::default()
                .bg(theme.warning)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::raw(" "));
    }
    right_spans.push(Span::styled(
        " FIXTURE:SIMULATED ",
        theme.fixture_badge_style(),
    ));

    let right_p = Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right);
    f.render_widget(right_p, chunks[2]);
}

pub fn render_tabs_bar(f: &mut Frame, area: Rect, app: &App) {
    use crate::app::Tab;
    let theme = &app.theme;

    let mut tab_spans = Vec::new();
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let is_selected = *tab == app.current_tab;
        let title = tab.title();

        if is_selected {
            tab_spans.push(Span::styled(
                format!(" [{}] ", title),
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            tab_spans.push(Span::styled(
                format!("  {}  ", title),
                Style::default().fg(theme.fg_muted),
            ));
        }

        if i + 1 < Tab::ALL.len() {
            tab_spans.push(Span::styled("│", Style::default().fg(theme.border)));
        }
    }

    let p = Paragraph::new(Line::from(tab_spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);
}

pub fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let shortcuts = Line::from(vec![
        Span::styled(
            "1-7",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Tabs ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "Tab/←/→",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Nav ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "↑/↓/j/k",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Scroll ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Detail ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "Space",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Pause ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "g",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":GPU Mode ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "t",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(":Theme({}) ", app.settings.theme_mode.name()),
            Style::default().fg(theme.fg_muted),
        ),
        Span::styled(
            "?",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Help ", Style::default().fg(theme.fg_muted)),
        Span::styled(
            "q",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Quit", Style::default().fg(theme.fg_muted)),
    ]);

    let p = Paragraph::new(shortcuts).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);
}
