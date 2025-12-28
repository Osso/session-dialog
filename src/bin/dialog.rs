//! session-dialog binary
//!
//! This binary is spawned by daemons (authd, fwd) to show session-lock dialogs.
//! It receives configuration via --config (base64-encoded msgpack).

use session_dialog::{base64_decode, run_dialog, DialogConfig};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse --config argument
    let config = if let Some(pos) = args.iter().position(|a| a == "--config") {
        if let Some(config_b64) = args.get(pos + 1) {
            match base64_decode(config_b64) {
                Ok(bytes) => match DialogConfig::from_bytes(&bytes) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("session-dialog: failed to parse config: {}", e);
                        std::process::exit(3);
                    }
                },
                Err(e) => {
                    eprintln!("session-dialog: failed to decode config: {}", e);
                    std::process::exit(3);
                }
            }
        } else {
            eprintln!("session-dialog: --config requires an argument");
            std::process::exit(3);
        }
    } else {
        // Legacy mode: treat remaining args as command (for backward compat with authd)
        let command = args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
        if command.is_empty() {
            eprintln!("usage: session-dialog --config <base64> | <command>");
            std::process::exit(3);
        }
        DialogConfig {
            kind: session_dialog::DialogKind::PrivilegeEscalation { command },
            timeout_secs: None,
        }
    };

    // Run dialog (never returns)
    run_dialog(config);
}
