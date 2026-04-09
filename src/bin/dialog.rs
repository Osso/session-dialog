//! session-dialog binary
//!
//! This binary is spawned by daemons (authd, fwd) to show session-lock dialogs.
//! It receives configuration via --config (base64-encoded msgpack).

use session_dialog::{base64_decode, run_dialog, DialogConfig};
use std::env;

fn main() {
    // Force Wayland backend, skip X11 fallback
    // SAFETY: Called before any threads are spawned
    unsafe { std::env::set_var("WINIT_UNIX_BACKEND", "wayland") };

    let config = parse_args();
    run_dialog(config);
}

fn parse_args() -> DialogConfig {
    let args: Vec<String> = env::args().collect();

    let Some(pos) = args.iter().position(|a| a == "--config") else {
        return legacy_config(&args);
    };

    let Some(config_b64) = args.get(pos + 1) else {
        eprintln!("session-dialog: --config requires an argument");
        std::process::exit(3);
    };

    let bytes = base64_decode(config_b64).unwrap_or_else(|e| {
        eprintln!("session-dialog: failed to decode config: {}", e);
        std::process::exit(3);
    });

    DialogConfig::from_bytes(&bytes).unwrap_or_else(|e| {
        eprintln!("session-dialog: failed to parse config: {}", e);
        std::process::exit(3);
    })
}

fn legacy_config(args: &[String]) -> DialogConfig {
    let command = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    if command.is_empty() {
        eprintln!("usage: session-dialog --config <base64> | <command>");
        std::process::exit(3);
    }
    DialogConfig {
        kind: session_dialog::DialogKind::PrivilegeEscalation { command },
        timeout_secs: None,
    }
}
