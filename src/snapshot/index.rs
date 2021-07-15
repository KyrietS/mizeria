use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use super::timestamp::Timestamp;

#[allow(dead_code)]
pub struct Index {
    timestamp: Timestamp,
    location: PathBuf,
    entries: Vec<IndexEntry>,
}

impl Index {
    pub fn new(timestamp: Timestamp, location: PathBuf) -> Self {
        Self {
            timestamp,
            location,
            entries: vec![],
        }
    }
    pub fn push(&mut self, timestamp: &Timestamp, path: PathBuf) {
        self.entries.push(IndexEntry {
            timestamp: timestamp.to_owned(),
            path,
        });
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
}

struct IndexEntry {
    timestamp: Timestamp,
    path: PathBuf,
}

impl ToString for IndexEntry {
    fn to_string(&self) -> String {
        format!("{} {}", self.timestamp, self.path.display())
    }
}
