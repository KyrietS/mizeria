use std::{fmt::Display, path::PathBuf};

pub enum IntegrityCheckResult {
    Success,
    SnapshotDoesntExist,
    SnapshotNameHasInvalidTimestamp(String),
    IndexFileDoesntExist,
    FilesFolderDoesntExist,
    IndexFileContainsInvalidTimestampInLine(u32),
    IndexFileContainsInvalidPathInLine(u32),
    EntryIndexedButNotExists(PathBuf),
    EntryExistsButNotIndexed(PathBuf),
    UnexpectedError(String),
}

impl IntegrityCheckResult {
    pub fn get_message(&self) -> String {
        match self {
            Self::Success => "No problems found.".into(),
            Self::SnapshotDoesntExist => "Snapshot doesn't exist.".into(),
            Self::SnapshotNameHasInvalidTimestamp(name) => {
                format!("Snapshot's name '{}' is not a correct timestamp.", name)
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
                "Entry '{}' is indexed, but is missing in snapshot.",
                path.display()
            ),
            IntegrityCheckResult::EntryExistsButNotIndexed(path) => format!(
                "Entry '{}' is present in snapshot, but is not indexed.",
                path.display()
            ),
            IntegrityCheckResult::UnexpectedError(message) => {
                format!("Unexpected error occured: {}", message)
            }
        }
    }
}

impl Display for IntegrityCheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_message())
    }
}
