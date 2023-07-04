use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use log::{debug, trace};

use crate::result::{IntegrityCheckError, IntegrityCheckResult};

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
            let index_entry =
                IndexEntry::from_line(line.borrow()).or(Err("index.txt is broken"))?;
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

    pub fn check_integrity(location: PathBuf) -> IntegrityCheckResult {
        if !location.exists() {
            return Err(IntegrityCheckError::IndexFileDoesntExist);
        }

        let file = File::open(&location).or(Err(IntegrityCheckError::UnexpectedError(
            "Cannot open index.txt".into(),
        )))?;
        let file = BufReader::new(&file);

        debug!("Traversing index has started");
        for (line_num, read_line) in file.lines().enumerate() {
            let line_num = line_num + 1;
            let line = read_line.or(Err(IntegrityCheckError::UnexpectedError(
                "Error while reading index.txt".into(),
            )))?;

            trace!("Line {}: {}", line_num, line);
            match IndexEntry::from_line(line.borrow()) {
                Ok(_) => (),
                Err(IndexEntryParseError::SyntaxError)
                | Err(IndexEntryParseError::InvalidTimestamp) => {
                    return Err(
                        IntegrityCheckError::IndexFileContainsInvalidTimestampInLine(line_num),
                    )
                }
                Err(IndexEntryParseError::InvalidPath) => {
                    return Err(IntegrityCheckError::IndexFileContainsInvalidPathInLine(
                        line_num,
                    ))
                }
            }
        }

        Ok(())
    }
}

enum IndexEntryParseError {
    SyntaxError,
    InvalidTimestamp,
    InvalidPath,
}

impl std::fmt::Debug for IndexEntryParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SyntaxError => write!(f, "SyntaxError"),
            Self::InvalidTimestamp => write!(f, "InvalidTimestamp"),
            Self::InvalidPath => write!(f, "InvalidPath"),
        }
    }
}

#[derive(Clone)]
pub struct IndexEntry {
    pub timestamp: Timestamp,
    pub path: PathBuf,
}

impl IndexEntry {
    fn from_line(line: &str) -> Result<Self, IndexEntryParseError> {
        let (timestamp_slice, path_slice) = line
            .split_once(' ')
            .ok_or(IndexEntryParseError::SyntaxError)?;

        let timestamp =
            Timestamp::parse_from(timestamp_slice).ok_or(IndexEntryParseError::InvalidTimestamp)?;
        let path = PathBuf::from(path_slice.trim());
        if !path.is_absolute() {
            return Err(IndexEntryParseError::InvalidPath);
        }

        Ok(Self { timestamp, path })
    }
}

impl ToString for IndexEntry {
    fn to_string(&self) -> String {
        format!("{} {}", self.timestamp, self.path.display())
    }
}

pub struct IndexPreview {
    inner: HashMap<PathBuf, Timestamp, ahash::RandomState>,
}

impl IndexPreview {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = File::open(path).or(Err("Cannot open index.txt"))?;
        let file = BufReader::new(&file);
        let mut entries = HashMap::default();
        for line in file.lines() {
            let line = line.or(Err("Error while reading index.txt"))?;
            let (timestamp_slice, path_slice) = line
                .split_once(' ')
                .ok_or("Index line has invalid format")?;
            let timestamp = Timestamp::parse_from(timestamp_slice).ok_or("Invalid timestamp")?;
            entries.insert(PathBuf::from(path_slice), timestamp);
        }
        Ok(Self { inner: entries })
    }

    pub fn find(&self, entry: &Path) -> Option<&Timestamp> {
        let absolute_entry = entry.canonicalize().ok()?;
        self.inner.get(&absolute_entry)
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;

    #[test]
    fn open_index_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("index.txt");

        let test_path_1 = tempdir.path().join("some entry/with spaces");
        let test_path_2 = tempdir.path().join("zażółć gęślą jaźń");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "2021-07-16_18.34 {}", test_path_1.display()).unwrap();
        writeln!(file, "2021-07-17_18.34 {}", test_path_2.display()).unwrap();

        let index = Index::open(file_path.clone()).unwrap();

        assert_eq!(file_path, index.location);
        assert_eq!(index.entries[0].path, test_path_1);
        assert_eq!(index.entries[1].path, test_path_2);
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
        let some_valid_absolute_path = tempfile::tempdir().unwrap();
        let valid_path = some_valid_absolute_path
            .path()
            .as_os_str()
            .to_string_lossy();
        let valid_line = format!("2021-07-15_18.34 {}\n", valid_path);

        let index_entry = IndexEntry::from_line(valid_line.as_str()).unwrap();
        assert_eq!("2021-07-15_18.34", format!("{}", index_entry.timestamp));
        assert_eq!(valid_path, format!("{}", index_entry.path.display()));
    }

    #[test]
    fn index_entry_from_invalid_line() {
        assert!(matches!(
            IndexEntry::from_line("foo"),
            Err(IndexEntryParseError::SyntaxError)
        ));
        let some_valid_absolute_path = tempfile::tempdir().unwrap();

        // invalid timestamp
        let entry_with_invalid_timestamp = format!(
            "2021-07-15_18:34  {}\n",
            some_valid_absolute_path.path().display()
        );
        assert!(matches!(
            IndexEntry::from_line(entry_with_invalid_timestamp.as_str()),
            Err(IndexEntryParseError::InvalidTimestamp)
        ));

        // invalid path
        let entry_with_invalid_path = "2021-07-15_18.34 this/path/is/local";
        assert!(matches!(
            IndexEntry::from_line(entry_with_invalid_path),
            Err(IndexEntryParseError::InvalidPath)
        ));
    }
}
