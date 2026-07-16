//! `decide_app_mode` routing tests: which flags select which application mode.

use super::*;

// --- decide_app_mode additional tests ---

#[test]
fn decide_app_mode_output_some_gives_cli() {
    let args = Args {
        cli: false,
        select: false,
        output: Some(PathBuf::from("x.png")),
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_select_true_gives_selection() {
    let args = Args {
        cli: false,
        select: true,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

#[test]
fn decide_app_mode_cli_true_gives_cli() {
    let args = Args {
        cli: true,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_list_true_gives_cli() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: true,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_all_false_gives_tray() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Tray));
}

#[test]
fn decide_app_mode_select_wins_over_cli() {
    // select=true takes priority even when cli=true
    let args = Args {
        cli: true,
        select: true,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

#[test]
fn decide_app_mode_select_overrides_output() {
    // select=true takes priority even when output is also set
    let args = Args {
        cli: false,
        select: true,
        output: Some(PathBuf::from("x.png")),
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

// --- select_record flag tests ---

#[test]
fn decide_app_mode_select_record_true_gives_select_record() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_wins_over_cli() {
    let args = Args {
        cli: true,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_wins_over_list() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: true,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_and_select_both_true_gives_select_record() {
    let args = Args {
        cli: false,
        select: true,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_with_output_gives_select_record() {
    // select_record takes priority even when output path is set
    let args = Args {
        cli: false,
        select: false,
        output: Some(PathBuf::from("out.png")),
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_window_gives_cli() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

// --- decide_app_mode: select_record ---

#[test]
fn decide_app_mode_select_record_gives_select_record() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_window_flag_gives_cli() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

// --- decide_app_mode: window vs select_record precedence ---

#[test]
fn decide_app_mode_select_record_wins_over_window() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: true,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_wins_over_window() {
    let args = Args {
        cli: false,
        select: true,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

// -------------------------------------------------------------------------
// decide_app_mode — select_record
// -------------------------------------------------------------------------

#[test]
fn decide_app_mode_select_record_is_select_record() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: true,
        window: false,
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}
