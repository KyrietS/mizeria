# Mizeria – simple backup software

![Lincense](https://img.shields.io/github/license/KyrietS/mizeria)
![Windows](https://img.shields.io/github/workflow/status/KyrietS/mizeria/Windows/master?label=windows)
![Linux](https://img.shields.io/github/workflow/status/KyrietS/mizeria/Linux/master?label=linux)
![macOS](https://img.shields.io/github/workflow/status/KyrietS/mizeria/macOS/master?label=macos)
![Static Analysis](https://img.shields.io/github/workflow/status/KyrietS/mizeria/Static%20analysis/master?label=static%20analysis)

Mizeria is a simple program for making backups. It is written in Rust and it supports Windows, Linux and macOS. The goal of this project is to provide straightforward and easy to understand structure of a backup.

## Basic usage
Create a snapshot of your files:
```
./mizeria <path_to_backup_folder> <path_to_my_files>
```

## Key features

* Single executable file.
* Extremely fast.
* Backed up files and folders are stored as files and folders.

## Planned features

* Incremental backups.
* Merging and removing snapshots.
* Backup restoration procedures.
* Compressing snapshots into zips.
* Repairing corrupted snapshots.
* And more...

## Some terms used in the project

* **Backup** – folder with snapshots. 
* **Snapshot** – folder with a name from time and date when the snapshot was created. Every snapshot contains file `index.txt` and folder `files/`.
* **Index** – text file stored in every snapshot with a name `index.txt`. It is a list of absolute paths to every file that was present at a time when snapshot was made.
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
Copyright © 2020 KyrietS\
Use of this software is granted under the terms of the MIT License.

See the [LICENSE](LICENSE.txt) for the full license text.
