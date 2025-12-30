# session-dialog

[![CI](https://github.com/Osso/session-dialog/actions/workflows/ci.yml/badge.svg)](https://github.com/Osso/session-dialog/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Wayland session-lock confirmation dialogs using iced.

## Features

- Session-lock based dialogs (locks screen during confirmation)
- Privilege escalation confirmation prompts
- Timeout support
- Async dialog API

## Usage

```rust
use session_dialog::{DialogConfig, DialogKind, DialogResult};

let config = DialogConfig {
    kind: DialogKind::PrivilegeEscalation {
        command: "pacman -Syu".to_string()
    },
    timeout_secs: Some(30),
};

let handle = session_dialog::show_dialog_async(config, env_vars);
match handle.join() {
    Ok(DialogResult::Confirmed) => println!("User confirmed"),
    Ok(DialogResult::Denied) => println!("User denied"),
    _ => println!("Error or timeout"),
}
```

## License

MIT
