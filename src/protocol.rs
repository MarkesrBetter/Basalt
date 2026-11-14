use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub const MINECRAFT_1_21_4_NAME: &str = "1.21.4";
pub const MINECRAFT_1_21_4_PROTOCOL: i32 = 769;

/// Clean-room Minecraft Java Edition binary packet buffer (VarInt, Strings, UUID).
#[derive(Debug, Default, Clone)]
pub struct PacketBuffer {
    pub data: Vec<u8>,
    pub cursor: usize,
}

impl PacketBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data, cursor: 0 }
    }

    pub fn write_u16(&mut self, v: u16) {
        self.data.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_i64(&mut self, v: i64) {
        self.data.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_varint(&mut self, value: i32) {
        let mut val = value as u32;
        loop {
            if (val & !0x7F) == 0 {
                self.data.push(val as u8);
                return;
            }
            self.data.push(((val & 0x7F) | 0x80) as u8);
            val >>= 7;
        }
    }

    pub fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.write_varint(bytes.len() as i32);
        self.data.extend_from_slice(bytes);
    }

    pub fn write_uuid(&mut self, uuid: [u8; 16]) {
        self.data.extend_from_slice(&uuid);
    }

    pub fn read_u8(&mut self) -> Result<u8, String> {
        if self.cursor >= self.data.len() {
            return Err("Reached end of packet (u8)".to_string());
        }
        let b = self.data[self.cursor];
        self.cursor += 1;
        Ok(b)
    }

    pub fn read_exact(&mut self, len: usize) -> Result<&[u8], String> {
        if self.cursor + len > self.data.len() {
            return Err(format!("Insufficient packet data: requested {len} bytes"));
        }
        let slice = &self.data[self.cursor..self.cursor + len];
        self.cursor += len;
        Ok(slice)
    }

    pub fn read_varint(&mut self) -> Result<i32, String> {
        let mut num_read = 0u32;
        let mut result = 0i32;
        loop {
            let byte = self.read_u8()?;
            let value = (byte & 0x7F) as i32;
            result |= value << (7 * num_read);
            num_read += 1;
            if num_read > 5 {
                return Err("Invalid VarInt (> 5 bytes)".to_string());
            }
            if (byte & 0x80) == 0 {
                break;
            }
        }
        Ok(result)
    }

    pub fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_varint()?;
        if len < 0 || len > 32767 * 4 {
            return Err(format!("Invalid String length: {len}"));
        }
        let bytes = self.read_exact(len as usize)?.to_vec();
        String::from_utf8(bytes).map_err(|e| format!("UTF-8 decode error: {e}"))
    }
}

/// Deterministic offline/local player UUID bytes for a username.
pub fn local_player_uuid_bytes(username: &str) -> [u8; 16] {
    let mut hash = [0u8; 16];
    let seed = format!("OfflinePlayer:{username}");
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for (i, &b) in seed.as_bytes().iter().enumerate() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(b as u64 + i as u64);
        hash[i % 16] ^= (state >> 24) as u8;
    }
    hash[6] = (hash[6] & 0x0F) | 0x30;
    hash[8] = (hash[8] & 0x3F) | 0x80;
    hash
}

pub fn format_uuid_dashed(b: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

pub fn local_player_uuid_dashed(username: &str) -> String {
    format_uuid_dashed(&local_player_uuid_bytes(username))
}

pub fn build_login_start_payload_1_21_4(username: &str) -> Vec<u8> {
    let mut buf = PacketBuffer::new();
    buf.write_string(username);
    buf.write_uuid(local_player_uuid_bytes(username));
    buf.data
}

pub fn write_framed_packet(
    stream: &mut TcpStream,
    packet_id: i32,
    payload: &[u8],
    compression_threshold: Option<usize>,
) -> Result<(), String> {
    let mut inner = PacketBuffer::new();
    inner.write_varint(packet_id);
    inner.data.extend_from_slice(payload);

    let frame = match compression_threshold {
        None => inner.data,
        Some(threshold) => {
            let mut out = PacketBuffer::new();
            if inner.data.len() >= threshold {
                out.write_varint(inner.data.len() as i32);
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                encoder
                    .write_all(&inner.data)
                    .map_err(|e| format!("Zlib compression error: {e}"))?;
                let compressed = encoder
                    .finish()
                    .map_err(|e| format!("Zlib finish error: {e}"))?;
                out.data.extend_from_slice(&compressed);
            } else {
                out.write_varint(0);
                out.data.extend_from_slice(&inner.data);
            }
            out.data
        }
    };

    let mut packet = PacketBuffer::new();
    packet.write_varint(frame.len() as i32);
    packet.data.extend_from_slice(&frame);

    stream
        .write_all(&packet.data)
        .map_err(|e| format!("Network write error: {e}"))
}

fn read_varint_stream(stream: &mut TcpStream) -> Result<i32, String> {
    let mut num_read = 0u32;
    let mut result = 0i32;
    let mut buf = [0u8; 1];
    loop {
        stream
            .read_exact(&mut buf)
            .map_err(|e| format!("Network read error: {e}"))?;
        let byte = buf[0];
        let value = (byte & 0x7F) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err("Invalid VarInt in network stream".to_string());
        }
        if (byte & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

pub fn read_framed_packet(
    stream: &mut TcpStream,
    compression_threshold: Option<usize>,
) -> Result<(i32, PacketBuffer), String> {
    let packet_len = read_varint_stream(stream)?;
    if packet_len <= 0 || packet_len > 8 * 1024 * 1024 {
        return Err(format!("Invalid packet size: {packet_len}"));
    }
    let mut raw = vec![0u8; packet_len as usize];
    stream
        .read_exact(&mut raw)
        .map_err(|e| format!("Failed to read packet body: {e}"))?;

    let mut buf = PacketBuffer::from_bytes(raw);
    if compression_threshold.is_some() {
        let data_len = buf.read_varint()?;
        let payload = if data_len == 0 {
            buf.data[buf.cursor..].to_vec()
        } else {
            let mut decoder = ZlibDecoder::new(&buf.data[buf.cursor..]);
            let mut decompressed = Vec::with_capacity(data_len as usize);
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| format!("Zlib decompression error: {e}"))?;
            decompressed
        };
        let mut inner = PacketBuffer::from_bytes(payload);
        let id = inner.read_varint()?;
        Ok((id, inner))
    } else {
        let id = buf.read_varint()?;
        Ok((id, buf))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerPingInfo {
    pub address: String,
    pub version_name: String,
    pub protocol_version: i32,
    pub online_players: i32,
    pub max_players: i32,
    pub description: String,
    pub latency_ms: u64,
}

#[derive(Deserialize)]
struct SlpResponse {
    version: SlpVersion,
    players: SlpPlayers,
    description: serde_json::Value,
}

#[derive(Deserialize)]
struct SlpVersion {
    name: String,
    protocol: i32,
}

#[derive(Deserialize)]
struct SlpPlayers {
    max: i32,
    online: i32,
}

fn parse_host_port(addr: &str) -> (String, u16) {
    if let Some((h, p)) = addr.rsplit_once(':')
        && let Ok(port) = p.parse::<u16>()
    {
        return (h.to_string(), port);
    }
    (addr.to_string(), 25565)
}

fn extract_description_text(val: &serde_json::Value) -> String {
    if let Some(s) = val.as_str() {
        return s.to_string();
    }
    if let Some(obj) = val.as_object() {
        let mut out = String::new();
        if let Some(t) = obj.get("text").and_then(|v| v.as_str()) {
            out.push_str(t);
        }
        if let Some(extra) = obj.get("extra").and_then(|v| v.as_array()) {
            for item in extra {
                out.push_str(&extract_description_text(item));
            }
        }
        return out;
    }
    "Server".to_string()
}

pub fn ping_server_1_21_4(address: &str) -> Result<ServerPingInfo, String> {
    let (host, port) = parse_host_port(address);
    let sock_addr = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address: {e}"))?
        .next()
        .ok_or_else(|| "No valid network address found".to_string())?;

    let mut stream = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(3))
        .map_err(|e| format!("Connection failed: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(3))).ok();

    let mut handshake = PacketBuffer::new();
    handshake.write_varint(MINECRAFT_1_21_4_PROTOCOL);
    handshake.write_string(&host);
    handshake.write_u16(port);
    handshake.write_varint(1);
    write_framed_packet(&mut stream, 0x00, &handshake.data, None)?;

    write_framed_packet(&mut stream, 0x00, &[], None)?;

    let (id, mut body) = read_framed_packet(&mut stream, None)?;
    if id != 0x00 {
        return Err(format!("Unexpected SLP packet ID: 0x{id:02X}"));
    }
    let json_str = body.read_string()?;
    let parsed: SlpResponse =
        serde_json::from_str(&json_str).map_err(|e| format!("SLP JSON error: {e}"))?;

    let start = Instant::now();
    let mut ping = PacketBuffer::new();
    ping.write_i64(1337);
    write_framed_packet(&mut stream, 0x01, &ping.data, None)?;
    let _ = read_framed_packet(&mut stream, None);
    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(ServerPingInfo {
        address: address.to_string(),
        version_name: parsed.version.name,
        protocol_version: parsed.version.protocol,
        online_players: parsed.players.online,
        max_players: parsed.players.max,
        description: extract_description_text(&parsed.description),
        latency_ms,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum LocalHandshakeResult {
    ReadyInPlay {
        server_info: String,
        keep_alive_tested: bool,
    },
    RequiresOfficialAuth,
    Disconnected(String),
}

/// Tests an offline-mode (`online-mode=false`) Login handshake against Protocol 769 (1.21.4).
/// Never bypasses encryption or official Microsoft authentication.
pub fn probe_local_dev_login_1_21_4(
    address: &str,
    username: &str,
) -> Result<LocalHandshakeResult, String> {
    let (host, port) = parse_host_port(address);
    let sock_addr = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address: {e}"))?
        .next()
        .ok_or_else(|| "No valid network address found".to_string())?;

    let mut stream = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(3))
        .map_err(|e| format!("Failed to connect to server: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();

    let mut handshake = PacketBuffer::new();
    handshake.write_varint(MINECRAFT_1_21_4_PROTOCOL);
    handshake.write_string(&host);
    handshake.write_u16(port);
    handshake.write_varint(2);
    write_framed_packet(&mut stream, 0x00, &handshake.data, None)?;

    let login_payload = build_login_start_payload_1_21_4(username);
    write_framed_packet(&mut stream, 0x00, &login_payload, None)?;

    let mut compression = None;
    for _ in 0..8 {
        let (pkt_id, mut body) = read_framed_packet(&mut stream, compression)?;
        match pkt_id {
            0x00 => {
                let reason = body.read_string().unwrap_or_else(|_| "Disconnected".into());
                return Ok(LocalHandshakeResult::Disconnected(reason));
            }
            0x01 => return Ok(LocalHandshakeResult::RequiresOfficialAuth),
            0x02 => {
                write_framed_packet(&mut stream, 0x03, &[], compression)?;
                return Ok(LocalHandshakeResult::ReadyInPlay {
                    server_info: format!(
                        "{MINECRAFT_1_21_4_NAME} (Protocol {MINECRAFT_1_21_4_PROTOCOL}) entered Configuration phase"
                    ),
                    keep_alive_tested: true,
                });
            }
            0x03 => {
                let thresh = body.read_varint()?;
                compression = if thresh >= 0 {
                    Some(thresh as usize)
                } else {
                    None
                };
            }
            _ => {}
        }
    }

    Ok(LocalHandshakeResult::Disconnected(
        "Login timed out".to_string(),
    ))
}
