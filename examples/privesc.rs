//! Show a PrivilegeEscalation dialog for visual testing/screenshots

use session_dialog::{show_dialog_inline, DialogConfig, DialogKind};
use std::collections::HashMap;

fn main() {
    let mut env = HashMap::new();
    for key in session_dialog::WAYLAND_ENV_VARS {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.to_string(), val);
        }
    }

    let config = DialogConfig {
        kind: DialogKind::PrivilegeEscalation {
            command: "/usr/bin/pacman -Syu --noconfirm".into(),
        },
        timeout_secs: Some(30),
    };

    let result = show_dialog_inline(config, &env);
    println!("Result: {:?}", result);
}
