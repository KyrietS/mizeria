# Mizeria – simple backup software

[![Mizeria release](https://img.shields.io/github/v/release/KyrietS/mizeria?include_prereleases&sort=semver)](https://github.com/KyrietS/mizeria/releases)
[![Lincense](https://img.shields.io/github/license/KyrietS/mizeria)](LICENSE.txt)
[![Windows](https://github.com/KyrietS/mizeria/actions/workflows/windows.yml/badge.svg)](https://github.com/KyrietS/mizeria/actions/workflows/windows.yml)
[![Linux](https://github.com/KyrietS/mizeria/actions/workflows/linux.yml/badge.svg)](https://github.com/KyrietS/mizeria/actions/workflows/linux.yml)
[![macOS](https://github.com/KyrietS/mizeria/actions/workflows/macos.yml/badge.svg)](https://github.com/KyrietS/mizeria/actions/workflows/macos.yml)
[![Static analysis](https://github.com/KyrietS/mizeria/actions/workflows/static-analysis.yml/badge.svg)](https://github.com/KyrietS/mizeria/actions/workflows/static-analysis.yml)

Mizeria is a simple program for making backups. It is written in Rust and it supports Windows, Linux and macOS. The goal of this project is to provide straightforward and easy to understand structure of a backup.

## Basic usage
Create a snapshot of your files:
```
mizeria backup <BACKUP> <INPUT>...
```

## Help

```
USAGE:
    mizeria backup [FLAGS] <BACKUP> <INPUT>...

FLAGS:
        --full       Force creating full snapshot
    -h, --help       Prints help information
    -v               Sets the level of verbosity

ARGS:
    <BACKUP>      A folder where snapshot will be stored
    <INPUT>...    Files or folders to be backed up
```

General help about the program:
```
mizeria --help
```

backup sumcommand help:
```
mizeria help backup
```
more detailed help about backup subcommand:
```
mizeria backup --help
```

## Key features

* Single executable file.
* Extremely fast.
* Backed up files and folders are stored as files and folders.
* Incremental backups. 🚀

## Planned features

* Merging and removing snapshots. 🚧
* Backup restoration procedures.
* Compressing snapshots into zips.
* Repairing corrupted snapshots.
* And more...

## Definitions

* **Backup** – folder with snapshots. 
* **Snapshot** - filder with backed up files and index for them. Snapshot's name consists of date and time when it was created.
* **Index** – text file stored in every snapshot under the name `index.txt`. It is a list of absolute paths to every file that was present at a time when snapshot was made.
* **Files** – folder with files that were copied from their origins. The absolute folder structure is preserved.

## Backup structure

Consider the following example
```
.
└── my_backup/
    ├── 2021-07-26_13.45/
    │   ├── index.txt
    │   └── files/
    │       └── C/
    │           └── my_folder/
    │               └── my_file.txt
    ├── 2021-07-27_13.45/
    │   ├── index.txt
    │   └── files/
    │       └── C/
    │           └── my_folder/
    │               └── my_modified_file.txt
    └── 2021-07-28_13.45/
        ├── index.txt
        └── files/
```

Backup presented above has 3 snapshots. Each snapshot except the last one consists of one file. Note how the absolute directory structures of a backed up files are preserved.

Let's look at the contents of a particular index.txt files from the backup above.

**2021-07-26_13.45/index.txt**
```
2021-07-26_13.45 C:\\my_folder\my_file.txt
```

**2021-07-27_13.45/index.txt**
```
2021-07-27_13.45 C:\\my_folder\my_modified_file.txt
```

**2021-07-28_13.45/index.txt**
```
2021-07-27_13.45 C:\\my_folder\my_modified_file.txt
```

The last snapshot does not have any files because `my_modified_file.txt` hasn't changed since last snapshot so incremental backup is performed. The unmodified file is noted in the index but it's pointing into the previous snapshot (see date before the file path).

## Tests

Every module has its own unit tests. This project has also integration/e2e tests to verify given user-cases and scenarios.

## License
Copyright © 2021 KyrietS\
Use of this software is granted under the terms of the MIT License.

See the [LICENSE](LICENSE.txt) for the full license text.
