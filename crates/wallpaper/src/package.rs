use std::{
    collections::HashSet,
    io::{self, Read, Seek, SeekFrom},
};

const MAX_ENTRIES: u32 = 100_000;
const MAX_INDEX_SIZE: u64 = 16 * 1024 * 1024;
const MAX_NAME: u32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug)]
pub struct Package {
    pub version: String,
    pub entries: Vec<Entry>,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn number<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
fn string<R: Read>(reader: &mut R, limit: u32) -> io::Result<String> {
    let length = number(reader)?;
    if length == 0 || length > limit {
        return Err(invalid("Invalid package string length"));
    }
    let mut bytes = vec![0; length as usize];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| invalid("Invalid package text"))
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['\\', ':', '\0'])
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
impl Package {
    pub fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let version = string(reader, 8)?;
        if version.len() != 8
            || !version.starts_with("PKGV")
            || !version[4..].bytes().all(|c| c.is_ascii_digit())
        {
            return Err(invalid("Unsupported package version"));
        }
        let count = number(reader)?;
        if count > MAX_ENTRIES {
            return Err(invalid("Too many package entries"));
        }
        let mut entries = Vec::new();
        let mut names = HashSet::new();
        for _ in 0..count {
            if reader.stream_position()? > MAX_INDEX_SIZE {
                return Err(invalid("Package index is too large"));
            }
            let name = string(reader, MAX_NAME)?;
            if !valid_name(&name) || !names.insert(name.to_ascii_lowercase()) {
                return Err(invalid("Invalid or duplicate package entry"));
            }
            let offset = number(reader)? as u64;
            let length = number(reader)? as u64;
            entries.push(Entry {
                name,
                offset,
                length,
            });
        }
        let data = reader.stream_position()?;
        if data > MAX_INDEX_SIZE {
            return Err(invalid("Package index is too large"));
        }
        for entry in &mut entries {
            let start = data
                .checked_add(entry.offset)
                .ok_or_else(|| invalid("Invalid package offset"))?;
            let end = start
                .checked_add(entry.length)
                .ok_or_else(|| invalid("Invalid package size"))?;
            if end > len {
                return Err(invalid("Package entry exceeds file"));
            }
            entry.offset = start;
        }
        Ok(Self { version, entries })
    }
    pub fn read_entry<R: Read + Seek>(
        &self,
        reader: &mut R,
        name: &str,
        limit: u64,
    ) -> io::Result<Vec<u8>> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Package entry not found"))?;
        if entry.length > limit || entry.length > usize::MAX as u64 {
            return Err(invalid("Package entry is too large"));
        }
        reader.seek(SeekFrom::Start(entry.offset))?;
        let mut data = vec![0; entry.length as usize];
        reader.read_exact(&mut data)?;
        Ok(data)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn package(name: &str, offset: u32, size: u32, data: &[u8]) -> Cursor<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&8u32.to_le_bytes());
        bytes.extend_from_slice(b"PKGV0005");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(data);
        Cursor::new(bytes)
    }
    #[test]
    fn reads_entries_without_loading_whole_archive() {
        let mut file = package("scene.json", 0, 2, b"{}");
        let pkg = Package::read(&mut file).unwrap();
        assert_eq!(pkg.version, "PKGV0005");
        assert_eq!(pkg.read_entry(&mut file, "scene.json", 2).unwrap(), b"{}");
        assert!(pkg.read_entry(&mut file, "scene.json", 1).is_err());
    }
    #[test]
    fn rejects_bad_paths_and_out_of_bounds_entries() {
        assert!(Package::read(&mut package("../scene.json", 0, 2, b"{}")).is_err());
        assert!(Package::read(&mut package("C:/scene.json", 0, 2, b"{}")).is_err());
        assert!(Package::read(&mut package("scene.json", u32::MAX, 2, b"{}")).is_err());
        assert!(Package::read(&mut package("scene.json", 0, 3, b"{}")).is_err());
    }
}
