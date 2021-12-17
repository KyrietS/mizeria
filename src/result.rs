use std::{fmt::Display, path::PathBuf};

pub enum IntegrityCheckResult {
    Success,
    SnapshotDoesntExist,
    SnapshotNameIsInvalidTimestamp,
    IndexFileDoesntExist,
    FilesFolderDoesntExist,
    IndexFileContainsInvalidTimestampInLine(u32),
    IndexFileContainsInvalidPathInLine(u32),
    EntryIndexedButNotExists(PathBuf),
    EntryExistsButNotIndexed(PathBuf),
}

impl IntegrityCheckResult {
    pub fn get_message(&self) -> String {
        match self {
            Self::Success => "No problems found.".into(),
            Self::SnapshotDoesntExist => "Snapshot doesn't exist.".into(),
            Self::SnapshotNameIsInvalidTimestamp => {
                "Snapshot's name is not a correct timestamp.".into()
            }
            Self::IndexFileDoesntExist => "Files index.txt is missing.".into(),
            Self::FilesFolderDoesntExist => "Folder files is missing.".into(),
            IntegrityCheckResult::IndexFileContainsInvalidTimestampInLine(line) => {
                format!("Invalid timestamp in line {} of index.txt.", line)
            }
            IntegrityCheckResult::IndexFileContainsInvalidPathInLine(line) => {
                format!("Invalid path in line {} of index.txt.", line)
            }
            IntegrityCheckResult::EntryIndexedButNotExists(path) => format!(
                "Entry '{}' is indexed, but missing in snapshot.",
                path.display()
            ),
            IntegrityCheckResult::EntryExistsButNotIndexed(path) => format!(
                "Entry '{}' found in snapshot, but not indexed.",
                path.display()
            ),
        }
    }
}

impl Display for IntegrityCheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_message())
    }
}
