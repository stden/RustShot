//! `decide_app_mode` routing tests: which flags select which application mode.

use super::*;

// --- decide_app_mode additional tests ---

#[test]
fn decide_app_mode_output_some_gives_cli() {
    let args = Args {
        output: Some(PathBuf::from("x.png")),
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_select_true_gives_selection() {
    let args = Args {
        select: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

#[test]
fn decide_app_mode_cli_true_gives_cli() {
    let args = Args {
        cli: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_list_true_gives_cli() {
    let args = Args {
        list: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_all_false_gives_tray() {
    let args = base_args();
    assert!(matches!(decide_app_mode(&args), AppMode::Tray));
}

#[test]
fn decide_app_mode_select_wins_over_cli() {
    // select=true takes priority even when cli=true
    let args = Args {
        cli: true,
        select: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

#[test]
fn decide_app_mode_select_overrides_output() {
    // select=true takes priority even when output is also set
    let args = Args {
        select: true,
        output: Some(PathBuf::from("x.png")),
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

// --- select_record flag tests ---

#[test]
fn decide_app_mode_select_record_true_gives_select_record() {
    let args = Args {
        select_record: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_wins_over_cli() {
    let args = Args {
        cli: true,
        select_record: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_wins_over_list() {
    let args = Args {
        list: true,
        select_record: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_and_select_both_true_gives_select_record() {
    let args = Args {
        select: true,
        select_record: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_record_with_output_gives_select_record() {
    // select_record takes priority even when output path is set
    let args = Args {
        output: Some(PathBuf::from("out.png")),
        select_record: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_window_gives_cli() {
    let args = Args {
        window: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

// --- decide_app_mode: window vs select_record precedence ---

#[test]
fn decide_app_mode_select_record_wins_over_window() {
    let args = Args {
        select_record: true,
        window: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::SelectRecord));
}

#[test]
fn decide_app_mode_select_wins_over_window() {
    let args = Args {
        select: true,
        window: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

// --- decide_app_mode: CLI-only window operations ---
// (--mcp is routed in `app::run` before decide_app_mode is ever consulted,
// so it is covered by parse-level tests in cli_parsing.rs, not here.)

#[test]
fn decide_app_mode_list_windows_gives_cli() {
    let args = Args {
        list_windows: true,
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}

#[test]
fn decide_app_mode_window_id_gives_cli() {
    let args = Args {
        window_id: Some(42),
        ..base_args()
    };
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
}
