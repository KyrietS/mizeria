use std::{fmt::Display, path::PathBuf};

pub type IntegrityCheckResult = std::result::Result<(), IntegrityCheckError>;

pub enum IntegrityCheckError {
    SnapshotDoesntExist,
    SnapshotNameHasInvalidTimestamp(String),
    IndexFileDoesntExist,
    FilesFolderDoesntExist,
    IndexFileContainsInvalidTimestampInLine(usize),
    IndexFileContainsInvalidPathInLine(usize),
    EntryIndexedButNotExists(PathBuf),
    EntryExistsButNotIndexed(PathBuf),
    UnexpectedError(String),
}

impl IntegrityCheckError {
    pub fn get_message(&self) -> String {
        match self {
            Self::SnapshotDoesntExist => "Snapshot doesn't exist.".into(),
            Self::SnapshotNameHasInvalidTimestamp(name) => {
                format!("Snapshot's name '{}' is not a correct timestamp.", name)
            }
            Self::IndexFileDoesntExist => "Files index.txt is missing.".into(),
            Self::FilesFolderDoesntExist => "Folder files is missing.".into(),
            IntegrityCheckError::IndexFileContainsInvalidTimestampInLine(line) => {
                format!("Invalid timestamp in line {} of index.txt.", line)
            }
            IntegrityCheckError::IndexFileContainsInvalidPathInLine(line) => {
                format!("Invalid path in line {} of index.txt.", line)
            }
            IntegrityCheckError::EntryIndexedButNotExists(path) => format!(
                "Entry '{}' is indexed, but is missing in snapshot.",
                path.display()
            ),
            IntegrityCheckError::EntryExistsButNotIndexed(path) => format!(
                "Entry '{}' is present in snapshot, but is not indexed.",
                path.display()
            ),
            IntegrityCheckError::UnexpectedError(message) => {
                format!("Unexpected error occured: {}", message)
            }
        }
    }
}

impl Display for IntegrityCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_message())
    }
}
