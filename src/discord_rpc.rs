use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// Public Discord Rich Presence application client ID for Basalt Launcher.
pub const BASALT_DISCORD_CLIENT_ID: &str = "1306984192837468190";
pub const BASALT_GITHUB_URL: &str = "https://github.com/Markesrrr/Basalt";
pub const BASALT_RELEASES_URL: &str = "https://github.com/Markesrrr/Basalt/releases";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRpcStatus {
    pub enabled: bool,
    pub connected: bool,
    pub pipe_name: String,
    pub details: String,
    pub state_label: String,
    pub started_epoch_secs: u64,
    pub github_url: String,
}

impl Default for DiscordRpcStatus {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            enabled: true,
            connected: false,
            pipe_name: "discord-ipc-0".to_string(),
            details: "Minecraft 1.21.4 (Vanilla)".to_string(),
            state_label: "Basalt Launcher · Ready".to_string(),
            started_epoch_secs: now,
            github_url: BASALT_GITHUB_URL.to_string(),
        }
    }
}

/// Encodes a Discord IPC frame: `[opcode: u32 LE][length: u32 LE][json payload]`.
pub fn encode_ipc_frame(opcode: u32, json_payload: &str) -> Vec<u8> {
    let bytes = json_payload.as_bytes();
    let len = bytes.len() as u32;
    let mut frame = Vec::with_capacity(8 + bytes.len());
    frame.extend_from_slice(&opcode.to_le_bytes());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(bytes);
    frame
}

/// Builds the `SET_ACTIVITY` JSON payload for Discord IPC Opcode 1.
pub fn build_activity_payload(
    pid: u32,
    details: &str,
    state_label: &str,
    started_epoch_secs: u64,
    mc_version: &str,
) -> String {
    let payload = serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": {
                "details": details,
                "state": state_label,
                "timestamps": {
                    "start": started_epoch_secs
                },
                "assets": {
                    "large_image": "basalt_logo",
                    "large_text": format!("Basalt Launcher · Vanilla {mc_version}"),
                    "small_image": "java21",
                    "small_text": "Java 21 · IDM Turbo · Low-Ping"
                },
                "buttons": [
                    {
                        "label": "Basalt GitHub",
                        "url": BASALT_GITHUB_URL
                    },
                    {
                        "label": "Download Release",
                        "url": BASALT_RELEASES_URL
                    }
                ]
            }
        },
        "nonce": format!("basalt-{started_epoch_secs}-{pid}")
    });
    payload.to_string()
}

/// Builds the `SET_ACTIVITY` clear payload when Rich Presence is disabled.
pub fn build_clear_activity_payload(pid: u32) -> String {
    serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": null
        },
        "nonce": format!("basalt-clear-{pid}")
    })
    .to_string()
}

/// Attempts a single non-blocking handshake + activity update over Windows named pipes
/// `\\.\pipe\discord-ipc-0` .. `\\.\pipe\discord-ipc-9`.
pub fn publish_discord_activity(
    enabled: bool,
    mc_version: &str,
    details: &str,
    state_label: &str,
    started_epoch_secs: u64,
) -> DiscordRpcStatus {
    let mut status = DiscordRpcStatus {
        enabled,
        connected: false,
        pipe_name: "Standby (WebRTC/IPC)".to_string(),
        details: details.to_string(),
        state_label: state_label.to_string(),
        started_epoch_secs,
        github_url: BASALT_GITHUB_URL.to_string(),
    };

    let pid = std::process::id();
    let handshake_json = serde_json::json!({
        "v": 1,
        "client_id": BASALT_DISCORD_CLIENT_ID
    })
    .to_string();
    let handshake_frame = encode_ipc_frame(0, &handshake_json);

    let activity_json = if enabled {
        build_activity_payload(pid, details, state_label, started_epoch_secs, mc_version)
    } else {
        build_clear_activity_payload(pid)
    };
    let activity_frame = encode_ipc_frame(1, &activity_json);

    for idx in 0..10 {
        let pipe_label = format!("discord-ipc-{idx}");
        let pipe_path = format!(r"\\.\pipe\{pipe_label}");
        let Ok(mut pipe) = OpenOptions::new().read(true).write(true).open(&pipe_path) else {
            continue;
        };

        if pipe.write_all(&handshake_frame).is_err() || pipe.flush().is_err() {
            continue;
        }

        let mut hdr = [0u8; 8];
        if pipe.read_exact(&mut hdr).is_err() {
            continue;
        }
        let resp_len = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
        if resp_len > 0 && resp_len <= 65536 {
            let mut body = vec![0u8; resp_len];
            let _ = pipe.read_exact(&mut body);
        }

        if pipe.write_all(&activity_frame).is_ok() && pipe.flush().is_ok() {
            status.connected = enabled;
            status.pipe_name = pipe_label;
            break;
        }
    }

    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_discord_ipc_frame_header_and_activity_json() {
        let frame = encode_ipc_frame(1, r#"{"v":1}"#);
        assert_eq!(&frame[0..4], &1u32.to_le_bytes());
        assert_eq!(&frame[4..8], &7u32.to_le_bytes());
        assert_eq!(&frame[8..], br#"{"v":1}"#);

        let payload = build_activity_payload(
            4242,
            "Minecraft 1.21.4 (Vanilla)",
            "Singleplayer · Survival",
            1700000000,
            "1.21.4",
        );
        assert!(payload.contains("SET_ACTIVITY"));
        assert!(payload.contains("https://github.com/Markesrrr/Basalt"));
        assert!(payload.contains("Singleplayer · Survival"));
    }
}
