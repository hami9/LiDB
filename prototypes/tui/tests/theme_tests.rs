use lidb_tui_prototype::theme::{Theme, ThemeMode};

#[test]
fn test_theme_cycle_all() {
    let mode = ThemeMode::Dark;
    let next1 = mode.next();
    assert_eq!(next1, ThemeMode::Light);

    let next2 = next1.next();
    assert_eq!(next2, ThemeMode::HighContrast);

    let next3 = next2.next();
    assert_eq!(next3, ThemeMode::Monochrome);

    let next4 = next3.next();
    assert_eq!(next4, ThemeMode::Dark);
}

#[test]
fn test_theme_styles_render() {
    for mode in [
        ThemeMode::Dark,
        ThemeMode::Light,
        ThemeMode::HighContrast,
        ThemeMode::Monochrome,
    ] {
        let theme = Theme::new(mode);
        assert!(!theme.mode.name().is_empty());
        let _ = theme.title_style();
        let _ = theme.block_border_style(true);
        let _ = theme.block_border_style(false);
        let _ = theme.fixture_badge_style();
        let _ = theme.selected_row_style();
    }
}
