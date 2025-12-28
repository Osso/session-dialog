//! Session-lock confirmation dialogs for Wayland
//!
//! This library provides secure full-screen confirmation dialogs using
//! the ext-session-lock Wayland protocol. Designed for privilege escalation
//! daemons (authd) and application firewalls.
//!
//! # Usage
//!
//! For daemons that need to show confirmation dialogs:
//!
//! ```no_run
//! use session_dialog::{DialogConfig, DialogKind, show_dialog};
//!
//! let config = DialogConfig {
//!     kind: DialogKind::PrivilegeEscalation {
//!         command: "/usr/bin/pacman -Syu".into(),
//!     },
//!     timeout_secs: None,
//! };
//!
//! let result = show_dialog(&config, 1000, 1000, &wayland_env);
//! ```

mod ui;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

/// Result of showing a confirmation dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogResult {
    /// User confirmed the action
    Confirmed,
    /// User denied the action
    Denied,
    /// Dialog timed out
    Timeout,
    /// Error showing dialog
    Error,
}

/// Type of confirmation dialog to show
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogKind {
    /// Privilege escalation (authd/sudo replacement)
    PrivilegeEscalation {
        /// Command requesting elevation
        command: String,
    },
    /// Network connection request (application firewall)
    NetworkConnection {
        /// Process name requesting connection
        process: String,
        /// Process path
        process_path: PathBuf,
        /// Destination address
        destination: String,
        /// Port number
        port: u16,
        /// Protocol (TCP/UDP)
        protocol: String,
    },
    /// Generic confirmation
    Generic {
        /// Title text
        title: String,
        /// Main message
        message: String,
        /// Detail/command text
        detail: String,
    },
}

/// Configuration for a dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogConfig {
    /// Type of dialog to show
    pub kind: DialogKind,
    /// Optional timeout in seconds (None = no timeout)
    pub timeout_secs: Option<u32>,
}

impl DialogConfig {
    /// Serialize config to msgpack bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("serialize config")
    }

    /// Deserialize config from msgpack bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Get the title for this dialog kind
    pub fn title(&self) -> &str {
        match &self.kind {
            DialogKind::PrivilegeEscalation { .. } => "Authorization Required",
            DialogKind::NetworkConnection { .. } => "Network Connection Request",
            DialogKind::Generic { title, .. } => title,
        }
    }

    /// Get the subtitle/description for this dialog kind
    pub fn subtitle(&self) -> &str {
        match &self.kind {
            DialogKind::PrivilegeEscalation { .. } => "An application wants to run as root:",
            DialogKind::NetworkConnection { .. } => "An application wants to connect to:",
            DialogKind::Generic { message, .. } => message,
        }
    }

    /// Get the detail text (command, connection info, etc.)
    pub fn detail(&self) -> String {
        match &self.kind {
            DialogKind::PrivilegeEscalation { command } => command.clone(),
            DialogKind::NetworkConnection {
                process,
                destination,
                port,
                protocol,
                ..
            } => format!("{} → {}:{} ({})", process, destination, port, protocol),
            DialogKind::Generic { detail, .. } => detail.clone(),
        }
    }
}

/// Wayland environment variables needed for dialog
pub const WAYLAND_ENV_VARS: &[&str] = &[
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// Show a confirmation dialog by spawning the session-dialog binary
///
/// This spawns the dialog binary with dropped privileges (caller's UID/GID)
/// and the necessary Wayland environment variables.
///
/// # Arguments
/// * `config` - Dialog configuration
/// * `uid` - User ID to run dialog as
/// * `gid` - Group ID to run dialog as
/// * `env` - Environment variables (must include WAYLAND_DISPLAY, XDG_RUNTIME_DIR)
///
/// # Returns
/// DialogResult indicating user's choice or error
pub fn show_dialog(
    config: &DialogConfig,
    uid: u32,
    gid: u32,
    env: &HashMap<String, String>,
) -> DialogResult {
    // Find session-dialog binary
    let dialog_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("session-dialog")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("/usr/bin/session-dialog"));

    // Encode config as base64 msgpack
    let config_bytes = config.to_bytes();
    let config_b64 = base64_encode(&config_bytes);

    // Spawn dialog with dropped privileges
    let result = Command::new(&dialog_bin)
        .arg("--config")
        .arg(&config_b64)
        .uid(uid)
        .gid(gid)
        .envs(
            WAYLAND_ENV_VARS
                .iter()
                .filter_map(|&key| env.get(key).map(|val| (key, val))),
        )
        .status();

    match result {
        Ok(status) => {
            match status.code() {
                Some(0) => DialogResult::Confirmed,
                Some(1) => DialogResult::Denied,
                Some(2) => DialogResult::Timeout,
                _ => DialogResult::Error,
            }
        }
        Err(_) => DialogResult::Error,
    }
}

/// Run the dialog UI (called by the binary, not by library users)
///
/// This function takes over the process and displays the session-lock dialog.
/// It exits with code 0 (confirmed), 1 (denied), 2 (timeout), or 3 (error).
pub fn run_dialog(config: DialogConfig) -> ! {
    let exit_code = ui::run(config);
    std::process::exit(exit_code);
}

/// Show the dialog inline without forking
///
/// Sets the necessary Wayland environment variables and runs the dialog
/// in the current process. This blocks until the user responds.
///
/// # Arguments
/// * `config` - Dialog configuration
/// * `env` - Environment variables (must include WAYLAND_DISPLAY, XDG_RUNTIME_DIR)
///
/// # Returns
/// DialogResult indicating user's choice
///
/// # Note
/// This works even when running as root, as long as the Wayland env vars are correct.
pub fn show_dialog_inline(config: DialogConfig, env: &std::collections::HashMap<String, String>) -> DialogResult {
    // Set Wayland env vars
    // SAFETY: We're single-threaded at this point or the caller ensures thread safety
    for key in WAYLAND_ENV_VARS {
        if let Some(val) = env.get(*key) {
            unsafe { std::env::set_var(key, val) };
        }
    }

    let exit_code = ui::run(config);
    match exit_code {
        0 => DialogResult::Confirmed,
        1 => DialogResult::Denied,
        2 => DialogResult::Timeout,
        _ => DialogResult::Error,
    }
}

/// Show the dialog in a separate thread
///
/// Spawns a new thread to run the dialog, allowing the caller to continue
/// other work. Returns a handle that can be joined to get the result.
///
/// # Arguments
/// * `config` - Dialog configuration
/// * `env` - Environment variables (must include WAYLAND_DISPLAY, XDG_RUNTIME_DIR)
///
/// # Returns
/// JoinHandle that resolves to DialogResult
pub fn show_dialog_async(
    config: DialogConfig,
    env: std::collections::HashMap<String, String>,
) -> std::thread::JoinHandle<DialogResult> {
    std::thread::spawn(move || show_dialog_inline(config, &env))
}

// Simple base64 encoding (no external dependency)
fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    let mut encoder = Base64Encoder::new(&mut buf);
    encoder.write_all(data).unwrap();
    drop(encoder);
    String::from_utf8(buf).unwrap()
}

/// Decode base64 string to bytes
pub fn base64_decode(s: &str) -> Result<Vec<u8>, &'static str> {
    fn decode_char(c: u8) -> Result<u8, &'static str> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err("invalid base64 character"),
        }
    }

    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return Err("invalid base64 length");
        }

        let a = decode_char(chunk[0])?;
        let b = decode_char(chunk[1])?;
        let c = decode_char(chunk[2])?;
        let d = decode_char(chunk[3])?;

        result.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            result.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            result.push((c << 6) | d);
        }
    }

    Ok(result)
}

struct Base64Encoder<'a> {
    buf: &'a mut Vec<u8>,
    pending: u32,
    pending_bits: u8,
}

impl<'a> Base64Encoder<'a> {
    fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { buf, pending: 0, pending_bits: 0 }
    }

    fn flush_pending(&mut self) {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        while self.pending_bits >= 6 {
            self.pending_bits -= 6;
            let idx = ((self.pending >> self.pending_bits) & 0x3F) as usize;
            self.buf.push(ALPHABET[idx]);
        }
    }
}

impl std::io::Write for Base64Encoder<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &byte in buf {
            self.pending = (self.pending << 8) | byte as u32;
            self.pending_bits += 8;
            self.flush_pending();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Base64Encoder<'_> {
    fn drop(&mut self) {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        if self.pending_bits > 0 {
            let shift = 6 - self.pending_bits;
            let idx = ((self.pending << shift) & 0x3F) as usize;
            self.buf.push(ALPHABET[idx]);
            // Padding
            let padding = (3 - (self.pending_bits / 8) % 3) % 3;
            for _ in 0..padding {
                self.buf.push(b'=');
            }
        }
    }
}
