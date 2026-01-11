use std::io::{self, Read, Write};
use serde_json::Value;

/// Read a message from standard input to comply with the
/// native messenger protocol.
pub fn read_message() -> Option<Value> {
    let mut len_buf = [0u8; 4];
    io::stdin().read_exact(&mut len_buf).ok()?;
    let msg_len = u32::from_le_bytes(len_buf) as usize;

    let mut msg_buf = vec![0u8; msg_len];
    io::stdin().read_exact(&mut msg_buf).ok()?;
    serde_json::from_slice(&msg_buf).ok()
}

/// Write a message to standard out to comply with the 
/// native messenger protocol.
pub fn write_message(message: &Value) -> io::Result<()> {
    let json_bytes = serde_json::to_vec(message)?;
    let len = (json_bytes.len() as u32).to_le_bytes();
    io::stdout().write_all(&len)?;
    io::stdout().write_all(&json_bytes)?;
    io::stdout().flush()
}
