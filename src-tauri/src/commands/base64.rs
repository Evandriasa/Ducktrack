const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn digit(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub fn decode(input: &str) -> Result<Vec<u8>, String> {
    let compact: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if compact.is_empty() {
        return Ok(Vec::new());
    }
    if compact.len() % 4 != 0 {
        return Err("Invalid base64 length (must be a multiple of 4)".into());
    }

    let mut out: Vec<u8> = Vec::with_capacity(compact.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut padding = false;
    let mut seen_valid = false;

    for &b in &compact {
        if b == b'=' {
            padding = true;
            continue;
        }
        if padding {
            return Err("Unexpected character after padding '='".into());
        }
        let val = digit(b).ok_or_else(|| format!("Invalid base64 char: {:?}", b as char))? as u32;
        seen_valid = true;
        acc = (acc << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    if !seen_valid {
        return Err("Invalid base64 input".into());
    }
    Ok(out)
}

pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).map(|&b| b as u32).unwrap_or(0);
        let b2 = chunk.get(2).map(|&b| b as u32).unwrap_or(0);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(
            if chunk.len() > 1 { TABLE[((n >> 6) & 63) as usize] as char } else { '=' },
        );
        out.push(if chunk.len() > 2 { TABLE[(n & 63) as usize] as char } else { '=' });
    }
    out
}

pub fn encode_data_url(mime: Option<&str>, data: &[u8]) -> String {
    let mime = mime.unwrap_or("image/png");
    format!("data:{mime};base64,{}", encode(data))
}

pub fn decode_data_url(input: &str) -> Result<(Option<String>, Vec<u8>), String> {
    if let Some(rest) = input.strip_prefix("data:") {
        let parts: Vec<&str> = rest.splitn(2, ';').collect();
        let mime = parts[0].to_string();
        let payload = parts.get(1).ok_or("Expected ',base64' in data URL")?;
        let payload = payload
            .strip_prefix("base64,")
            .ok_or("Expected 'base64,' in data URL")?;
        Ok((Some(mime), decode(payload)?))
    } else {
        Ok((None, decode(input)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let binary: String = b"\x00\x01".iter().map(|&b| b as char).collect();
        for s in ["", "f", "fo", "foo", "foob", "fooba", "foobar", binary.as_str()] {
            let bytes = s.as_bytes().to_vec();
            let enc = encode(&bytes);
            assert_eq!(decode(&enc).unwrap(), bytes, "round trip");
        }
    }

    #[test]
    fn data_url() {
        let (mime, bytes) = decode_data_url("data:image/png;base64,aGVsbG8=").unwrap();
        assert_eq!(mime.as_deref(), Some("image/png"));
        assert_eq!(bytes, b"hello");
        assert_eq!(encode_data_url(Some("image/png"), b"hello"), "data:image/png;base64,aGVsbG8=");
    }
}
