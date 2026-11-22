//! `data.acd`: Assetto Corsa's packed, lightly encrypted car data archive.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

const HEADER_MARKER: i32 = -1111;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcdEntry {
    pub name: String,
    pub data: Vec<u8>,
}

pub fn key_source_for(acd_path: &Path) -> String {
    let file_name = acd_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if file_name.to_ascii_lowercase().starts_with("data") {
        acd_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        file_name
    }
}

fn int_to_byte(v: i32) -> u8 {
    (v.wrapping_rem(256).wrapping_add(256)).wrapping_rem(256) as u8
}

fn safe_div(a: i32, b: i32) -> i32 {
    if b == 0 || (a == i32::MIN && b == -1) {
        0
    } else {
        a.wrapping_div(b)
    }
}

fn safe_rem(a: i32, b: i32) -> i32 {
    if b == 0 || (a == i32::MIN && b == -1) {
        0
    } else {
        a.wrapping_rem(b)
    }
}

pub fn derive_key(key_source: &str) -> Vec<u8> {
    let s: Vec<i32> = key_source
        .to_lowercase()
        .encode_utf16()
        .map(i32::from)
        .collect();
    let len = s.len() as i32;
    let at = |i: i32| s[i as usize];

    let b1 = int_to_byte(s.iter().fold(0i32, |a, &c| a.wrapping_add(c)));

    let mut n = 0i32;
    let mut i = 0;
    while i < len - 1 {
        n = n.wrapping_mul(at(i)).wrapping_sub(at(i + 1));
        i += 2;
    }
    let b2 = int_to_byte(n);

    let mut n = 0i32;
    let mut i = 1;
    while i < len - 3 {
        n = n.wrapping_mul(at(i));
        n = safe_div(n, at(i + 1) + 27);
        n = n.wrapping_add(-27 - at(i - 1));
        i += 3;
    }
    let b3 = int_to_byte(n);

    let mut n = 5763i32;
    for i in 1..len {
        n = n.wrapping_sub(at(i));
    }
    let b4 = int_to_byte(n);

    let mut n = 66i32;
    let mut i = 1;
    while i < len - 4 {
        n = (at(i) + 15)
            .wrapping_mul(n)
            .wrapping_mul(at(i - 1) + 15)
            .wrapping_add(22);
        i += 4;
    }
    let b5 = int_to_byte(n);

    let mut n = 101i32;
    let mut i = 0;
    while i < len - 2 {
        n = n.wrapping_sub(at(i));
        i += 2;
    }
    let b6 = int_to_byte(n);

    let mut n = 171i32;
    let mut i = 0;
    while i < len - 2 {
        n = safe_rem(n, at(i));
        i += 2;
    }
    let b7 = int_to_byte(n);

    let mut n = 171i32;
    let mut i = 0;
    while i < len - 1 {
        n = safe_div(n, at(i)).wrapping_add(at(i + 1));
        i += 1;
    }
    let b8 = int_to_byte(n);

    [b1, b2, b3, b4, b5, b6, b7, b8]
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join("-")
        .into_bytes()
}

fn decrypt(data: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return;
    }
    for (b, k) in data.iter_mut().zip(key.iter().cycle()) {
        *b = b.wrapping_sub(*k);
    }
}

fn encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; data.len() * 4];
    if key.is_empty() {
        return out;
    }
    for (i, (b, k)) in data.iter().zip(key.iter().cycle()).enumerate() {
        out[i * 4] = b.wrapping_add(*k);
    }
    out
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn i32(&mut self) -> Result<i32> {
        let end = self.pos + 4;
        ensure!(
            end <= self.bytes.len(),
            "unexpected end of data.acd at {}",
            self.pos
        );
        let slice = &self.bytes[self.pos..end];
        let v = i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        self.pos = end;
        Ok(v)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .context("data.acd length overflow")?;
        ensure!(
            end <= self.bytes.len(),
            "unexpected end of data.acd at {}",
            self.pos
        );
        let s = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(s)
    }
}

pub fn read_bytes(bytes: &[u8], key_source: &str) -> Result<Vec<AcdEntry>> {
    let key = derive_key(key_source);
    let mut c = Cursor { bytes, pos: 0 };
    if bytes.len() >= 8 && c.i32()? == HEADER_MARKER {
        c.i32()?;
    } else {
        c.pos = 0;
    }

    let mut entries = Vec::new();
    while c.pos < bytes.len() {
        let name_len = c.i32()?;
        ensure!(name_len >= 0, "negative entry name length in data.acd");
        let name = String::from_utf8_lossy(c.take(name_len as usize)?).into_owned();
        let data_len = c.i32()?;
        ensure!(
            data_len >= 0,
            "negative entry length in data.acd for {name}"
        );
        let raw_len = usize::try_from(data_len)
            .ok()
            .and_then(|n| n.checked_mul(4))
            .context("data.acd entry length overflow")?;
        let raw = c.take(raw_len)?;
        let mut data: Vec<u8> = raw.iter().step_by(4).copied().collect();
        decrypt(&mut data, &key);
        entries.push(AcdEntry { name, data });
    }
    Ok(entries)
}

pub fn read(acd_path: &Path) -> Result<Vec<AcdEntry>> {
    let bytes = fs::read(acd_path).with_context(|| format!("reading {}", acd_path.display()))?;
    read_bytes(&bytes, &key_source_for(acd_path))
}

pub fn write_bytes(entries: &[AcdEntry], key_source: &str) -> Result<Vec<u8>> {
    let key = derive_key(key_source);
    let mut out = Vec::new();
    for e in entries {
        ensure!(e.name.is_ascii(), "entry name must be ASCII: {}", e.name);
        out.extend_from_slice(&(e.name.len() as i32).to_le_bytes());
        out.extend_from_slice(e.name.as_bytes());
        let len = i32::try_from(e.data.len()).context("entry too large for data.acd")?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&encrypt(&e.data, &key));
    }
    Ok(out)
}

pub fn write(acd_path: &Path, entries: &[AcdEntry]) -> Result<()> {
    let bytes = write_bytes(entries, &key_source_for(acd_path))?;
    fs::write(acd_path, bytes).with_context(|| format!("writing {}", acd_path.display()))
}

pub fn unpack(acd_path: &Path, out_dir: &Path) -> Result<usize> {
    unpack_with_key(acd_path, out_dir, &key_source_for(acd_path))
}

pub fn unpack_with_key(acd_path: &Path, out_dir: &Path, key_source: &str) -> Result<usize> {
    let bytes = fs::read(acd_path).with_context(|| format!("reading {}", acd_path.display()))?;
    let entries = read_bytes(&bytes, key_source)?;
    fs::create_dir_all(out_dir)?;
    for e in &entries {
        let name = Path::new(&e.name);
        if name.is_absolute()
            || name
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            bail!("refusing unsafe entry name {:?}", e.name);
        }
        let target = out_dir.join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, &e.data)?;
    }
    Ok(entries.len())
}

pub fn pack(data_dir: &Path, acd_path: &Path) -> Result<usize> {
    pack_with_key(data_dir, acd_path, &key_source_for(acd_path))
}

pub fn pack_with_key(data_dir: &Path, acd_path: &Path, key_source: &str) -> Result<usize> {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in walkdir::WalkDir::new(data_dir)
        .min_depth(1)
        .follow_links(false)
    {
        let entry = entry.with_context(|| format!("walking {}", data_dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .strip_prefix(data_dir)
            .context("entry outside data directory")?
            .to_string_lossy()
            .replace('\\', "/");
        ensure!(
            !name.starts_with('/') && !name.split('/').any(|part| part == ".." || part.is_empty()),
            "unsafe entry name from {}",
            entry.path().display()
        );
        files.push((name, entry.path().to_path_buf()));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let entries = files
        .iter()
        .map(|(name, path)| {
            Ok(AcdEntry {
                name: name.clone(),
                data: fs::read(path)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let bytes = write_bytes(&entries, key_source)?;
    fs::write(acd_path, bytes).with_context(|| format!("writing {}", acd_path.display()))?;
    Ok(entries.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bytes() {
        let entries = vec![
            AcdEntry {
                name: "a.ini".into(),
                data: b"hello".to_vec(),
            },
            AcdEntry {
                name: "b.txt".into(),
                data: vec![0, 1, 2, 255],
            },
        ];
        let ks = "mycar";
        let bytes = write_bytes(&entries, ks).unwrap();
        let read = read_bytes(&bytes, ks).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].name, "a.ini");
        assert_eq!(read[0].data, b"hello");
        assert_eq!(read[1].name, "b.txt");
        assert_eq!(read[1].data, vec![0, 1, 2, 255]);
    }

    #[test]
    fn key_source_for_data_prefix() {
        let p = std::path::Path::new("/x/cars/mycar/data.acd");
        assert_eq!(key_source_for(p), "mycar");
    }

    #[test]
    fn key_source_for_other_name() {
        let p = std::path::Path::new("/x/foo.acd");
        assert_eq!(key_source_for(p), "foo.acd");
    }

    #[test]
    fn derive_key_nonempty() {
        let k = derive_key("abc");
        assert!(!k.is_empty());
    }

    #[test]
    fn derive_key_with_zero_code_units() {
        let k = derive_key("\0\0\0");
        assert!(!k.is_empty());
    }

    #[test]
    fn malformed_lengths_return_errors() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2i32.to_le_bytes());
        bytes.extend_from_slice(b"a");
        bytes.extend_from_slice(&i32::MAX.to_le_bytes());
        assert!(read_bytes(&bytes, "car").is_err());

        let mut truncated = Vec::new();
        truncated.extend_from_slice(&2i32.to_le_bytes());
        truncated.extend_from_slice(b"a");
        truncated.extend_from_slice(&1i32.to_le_bytes());
        assert!(read_bytes(&truncated, "car").is_err());
    }

    #[test]
    fn nested_unpack_pack_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.acd");
        let entries = vec![
            AcdEntry {
                name: "a.ini".into(),
                data: b"top".to_vec(),
            },
            AcdEntry {
                name: "nested/b.ini".into(),
                data: b"inner".to_vec(),
            },
        ];
        fs::write(&source, write_bytes(&entries, "mycar").unwrap()).unwrap();
        let out = dir.path().join("data");
        unpack_with_key(&source, &out, "mycar").unwrap();
        assert!(out.join("nested/b.ini").is_file());

        let repacked = dir.path().join("repacked.acd");
        pack_with_key(&out, &repacked, "mycar").unwrap();
        let round = read_bytes(&fs::read(&repacked).unwrap(), "mycar").unwrap();
        assert_eq!(round, entries);
    }
}
