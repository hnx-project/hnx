#![allow(dead_code)]

pub fn find_file(bytes: &[u8], path: &str) -> Option<(usize, usize)> {
    if bytes.len() < 6 {
        return None;
    }
    if &bytes[0..6] != b"070701" {
        return None;
    }
    let mut i: usize = 0;
    while i + 110 <= bytes.len() {
        if &bytes[i..i + 6] != b"070701" {
            break;
        }
        let namesize = read_hex(&bytes[i + 94..i + 102]);
        let filesize = read_hex(&bytes[i + 54..i + 62]);
        let mut pos = i + 110;
        if pos + namesize - 1 > bytes.len() {
            break;
        }
        let raw_name = &bytes[pos..pos + namesize - 1];
        pos += namesize;
        if !pos.is_multiple_of(4) {
            pos += 4 - (pos % 4);
        }
        if pos + filesize > bytes.len() {
            break;
        }
        let file_start = pos;
        let file_end = pos + filesize;
        if raw_name == b"TRAILER!!!" {
            break;
        }
        let name = trim_name(raw_name);
        if let Ok(s) = core::str::from_utf8(name) {
            if s == path {
                return Some((bytes.as_ptr() as usize + file_start, filesize));
            }
        }
        pos = file_end;
        if !pos.is_multiple_of(4) {
            pos += 4 - (pos % 4);
        }
        i = pos;
    }
    None
}

fn trim_name(name: &[u8]) -> &[u8] {
    if name.len() >= 2 && name[0] == b'.' && name[1] == b'/' {
        &name[2..]
    } else {
        name
    }
}

fn read_hex(s: &[u8]) -> usize {
    let mut n: usize = 0;
    for &c in s {
        n <<= 4;
        let v = hex_val(c);
        n |= v;
    }
    n
}

#[inline]
fn hex_val(c: u8) -> usize {
    if c.is_ascii_digit() {
        (c - b'0') as usize
    } else if (b'a'..=b'f').contains(&c) {
        (c - b'a' + 10) as usize
    } else if (b'A'..=b'F').contains(&c) {
        (c - b'A' + 10) as usize
    } else {
        0
    }
}
