use std::borrow::Borrow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::timestamp::Timestamp;

#[derive(Clone)]
pub struct Index {
    pub location: PathBuf,
    pub entries: Vec<IndexEntry>,
}

impl Index {
    pub fn new(location: PathBuf) -> Self {
        Self {
            location,
            entries: vec![],
        }
    }

    pub fn open(path: PathBuf) -> Result<Self, String> {
        let file = File::open(&path).or(Err("Cannot open index.txt"))?;
        let file = BufReader::new(&file);
        let mut entries = Vec::new();
        for line in file.lines() {
            let line = line.or(Err("Error while reading index.txt"))?;
            let index_entry = IndexEntry::from_line(line.borrow()).ok_or("index.txt is broken")?;
            entries.push(index_entry);
        }
        let index = Index {
            location: path,
            entries,
        };
        Ok(index)
    }

    pub fn push(&mut self, timestamp: Timestamp, path: PathBuf) {
        self.entries.push(IndexEntry { timestamp, path });
    }

    pub fn save(&self) -> io::Result<()> {
        let file = File::create(&self.location)?;
        let mut file = BufWriter::new(file);
        for index_entry in &self.entries {
            file.write_all(index_entry.to_string().as_bytes())?;
            file.write_all(&[b'\n'])?;
        }
        file.flush()?;
        Ok(())
    }

    pub fn find(&self, entry: &Path) -> Option<Timestamp> {
        let absolute_entry = entry.canonicalize().ok()?;
        let result = self.entries.iter().find(|e| e.path == absolute_entry)?;
        Some(result.timestamp.clone())
    }
}

#[derive(Clone)]
pub struct IndexEntry {
    pub timestamp: Timestamp,
    pub path: PathBuf,
}

impl IndexEntry {
    fn from_line(line: &str) -> Option<Self> {
        let (timestamp_slice, path_slice) = line.split_once(' ')?;
        let entry = Self {
            timestamp: Timestamp::parse_from(timestamp_slice)?,
            path: PathBuf::from(path_slice.trim()),
        };
        Some(entry)
    }
}

impl ToString for IndexEntry {
    fn to_string(&self) -> String {
        format!("{} {}", self.timestamp, self.path.display())
    }
}

#[cfg(test)]
mod index_tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn open_index_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("index.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "2021-07-16_18.34 /some path/with spaces").unwrap();
        writeln!(file, "2021-07-17_18.34 /another path").unwrap();

        let index = Index::open(file_path.clone()).unwrap();

        assert_eq!(file_path, index.location);
        assert_eq!(index.entries[0].path, Path::new("/some path/with spaces"));
        assert_eq!(index.entries[1].path, Path::new("/another path"));
        assert_eq!(index.entries[0].timestamp.to_string(), "2021-07-16_18.34");
        assert_eq!(index.entries[1].timestamp.to_string(), "2021-07-17_18.34");
    }

    #[test]
    fn open_invalid_file_returns_none() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("index.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "foo").unwrap();

        let result = Index::open(file_path.clone());
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod index_entry_tests {
    use super::*;

    #[test]
    fn index_entry_from_valid_line() {
        let valid_line = "2021-07-15_18.34 /dir 1/file.txt\n";

        let index_entry = IndexEntry::from_line(valid_line).unwrap();
        assert_eq!("2021-07-15_18.34", format!("{}", index_entry.timestamp));
        assert_eq!("/dir 1/file.txt", format!("{}", index_entry.path.display()));
    }

    #[test]
    fn index_entry_from_invalid_line() {
        assert!(IndexEntry::from_line("foo").is_none());
        // invalid timestamp
        assert!(IndexEntry::from_line("2021-07-15_18:34  /dir_1/file.txt\n").is_none());
    }
}
