//! V108-N26 Discord IPC frames: little-endian `u32` op, `u32` length, then JSON.

use std::io::{self, Read};

pub const HANDSHAKE: u32 = 0;
pub const FRAME: u32 = 1;
pub const CLOSE: u32 = 2;
pub const PING: u32 = 3;
pub const PONG: u32 = 4;

/// Discord's frames stay far below this; a bigger length means the stream is out of step.
const MAX_BODY: u32 = 64 * 1024;

pub fn encode(op: u32, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

pub fn read_frame<R: Read>(r: &mut R) -> io::Result<(u32, Vec<u8>)> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head)?;
    let op = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if len > MAX_BODY {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too long"));
    }
    let mut body = vec![0u8; len as usize];
    r.read_exact(&mut body)?;
    Ok((op, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn encodes_little_endian() {
        let f = encode(1, b"{}");
        assert_eq!(f, [1, 0, 0, 0, 2, 0, 0, 0, b'{', b'}']);
    }

    #[test]
    fn round_trips() {
        let mut bytes = encode(HANDSHAKE, br#"{"v":1}"#);
        bytes.extend(encode(PING, b"x"));
        let mut c = Cursor::new(bytes);
        assert_eq!(read_frame(&mut c).unwrap(), (HANDSHAKE, br#"{"v":1}"#.to_vec()));
        assert_eq!(read_frame(&mut c).unwrap(), (PING, b"x".to_vec()));
        assert!(read_frame(&mut c).is_err());
    }

    #[test]
    fn short_and_oversized_frames_fail() {
        let mut c = Cursor::new(vec![1, 0, 0, 0, 9, 0, 0, 0, b'a']);
        assert!(read_frame(&mut c).is_err());
        let mut big = vec![1, 0, 0, 0];
        big.extend((MAX_BODY + 1).to_le_bytes());
        assert_eq!(read_frame(&mut Cursor::new(big)).unwrap_err().kind(), io::ErrorKind::InvalidData);
    }
}
