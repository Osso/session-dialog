//! Test show_dialog_inline

use session_dialog::{show_dialog_inline, DialogConfig, DialogKind};
use std::collections::HashMap;

fn main() {
    let mut env = HashMap::new();

    // Get Wayland env from current environment
    for key in session_dialog::WAYLAND_ENV_VARS {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.to_string(), val);
        }
    }

    let config = DialogConfig {
        kind: DialogKind::NetworkConnection {
            process: "firefox".into(),
            process_path: "/usr/lib/firefox/firefox".into(),
            destination: "api.anthropic.com".into(),
            port: 443,
            protocol: "TCP".into(),
        },
        timeout_secs: Some(10),
    };

    println!("Showing dialog...");
    let result = show_dialog_inline(config, &env);
    println!("Result: {:?}", result);
}
