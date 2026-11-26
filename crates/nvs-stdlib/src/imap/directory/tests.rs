//! A `directory` block's mailbox in a scratch directory: its folders, the search, and the files
//! a flag, a move and a delete change.

use std::path::Path;

use super::{Directory, VALIDITY};
use crate::imap::session::Filter;

const MEMBER: &str = "Core\\Imap\\Folder::search";

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).expect("a file the test owns");
}

/// A mailbox with `INBOX` holding messages 1, 2 and 10, an empty `Archive`, and files that are
/// not folders or messages.
fn mailbox(dir: &Path) -> Directory {
    let inbox = dir.join("INBOX");
    std::fs::create_dir_all(&inbox).expect("the inbox");
    std::fs::create_dir_all(dir.join("Archive")).expect("the archive");
    write(
        &inbox.join("1.eml"),
        "From: Ada <ada@example.com>\r\nSubject: Invoice 7\r\nDate: Mon, 5 Jan 2026 08:30:00 +0100\r\n\r\nhi\r\n",
    );
    write(
        &inbox.join("2.eml"),
        "From: Bob <bob@example.com>\r\nSubject: Lunch\r\nDate: Tue, 10 Feb 2026 12:00:00 +0000\r\n\r\nhi\r\n",
    );
    write(
        &inbox.join("10.eml"),
        "From: ada@example.com\r\nSubject: Re: invoice 7\r\n\r\nhi\r\n",
    );
    write(&inbox.join("2.flags"), "\\Seen\n");
    write(&inbox.join("010.eml"), "not a message: a leading zero\r\n");
    write(&inbox.join("notes.txt"), "not a message\r\n");
    write(&dir.join("readme.txt"), "not a folder\r\n");
    Directory::new(dir.to_path_buf(), "inbox".to_owned())
}

fn search(mailbox: &Directory, filter: &Filter) -> Vec<u32> {
    mailbox
        .search("INBOX", filter, u64::MAX, MEMBER)
        .expect("the search")
}

/// Each subdirectory is a folder, sorted by name, and a file beside them is not. A folder name
/// that would leave the directory is a `RuntimeError`, and one that does not exist an
/// `IOError`. A search reads the flags and the headers of each `<uid>.eml` file.
// covers: Core\Imap\Mailbox::folders, Core\Imap\Mailbox::folder, Core\Imap\Folder::search
#[cfg_attr(miri, ignore = "reads a directory, which Miri does not emulate")]
#[test]
fn a_directory_block_lists_its_subdirectories_as_folders() {
    let dir = nvs_repo::scratch("imap-directory-folders");
    let mailbox = mailbox(&dir);
    assert_eq!(
        mailbox.folders(MEMBER).expect("the folders"),
        ["Archive", "INBOX"]
    );
    assert_eq!(
        mailbox.select("INBOX", MEMBER).expect("the folder"),
        VALIDITY
    );
    for outside in ["..", "../INBOX", "INBOX/..", "a\\b", "C:", ""] {
        let Err(fault) = mailbox.select(outside, MEMBER) else {
            panic!("`{outside}` was opened as a folder");
        };
        assert!(
            format!("{fault:?}").contains("one directory name"),
            "{outside}: {fault:?}"
        );
    }
    let Err(fault) = mailbox.select("Spam", MEMBER) else {
        panic!("a folder that does not exist was opened");
    };
    assert!(
        format!("{fault:?}").contains("no folder `Spam`"),
        "{fault:?}"
    );

    assert_eq!(search(&mailbox, &Filter::default()), [1, 2, 10]);
    let unseen = Filter {
        unseen: true,
        ..Filter::default()
    };
    assert_eq!(search(&mailbox, &unseen), [1, 10]);
    let invoices = Filter {
        subject: Some("INVOICE".to_owned()),
        from: Some("ada@".to_owned()),
        ..Filter::default()
    };
    assert_eq!(search(&mailbox, &invoices), [1, 10]);
    let february = Filter {
        since: Some((2026, 2, 1)),
        before: Some((2026, 3, 1)),
        ..Filter::default()
    };
    assert_eq!(search(&mailbox, &february), [2]);
    let first = Filter {
        limit: Some(2),
        ..Filter::default()
    };
    assert_eq!(search(&mailbox, &first), [1, 2]);

    let raw = mailbox
        .fetch("INBOX", 2, u64::MAX, MEMBER)
        .expect("message 2");
    assert!(raw.starts_with(b"From: Bob"));
    let Err(fault) = mailbox.fetch("INBOX", 2, 4, MEMBER) else {
        panic!("a message over the ceiling was read");
    };
    assert!(format!("{fault:?}").contains("was not read"), "{fault:?}");
    let Err(fault) = mailbox.fetch("INBOX", 3, u64::MAX, MEMBER) else {
        panic!("a missing message was read");
    };
    assert!(
        format!("{fault:?}").contains("no message with the UID 3"),
        "{fault:?}"
    );
}

/// A flag is a line of `<uid>.flags`, and the file is removed with its last flag. A move renames
/// both files into the target under its next free UID, and a delete removes them.
// covers: Core\Imap\Folder::move, Core\Imap\Folder::markSeen, Core\Imap\Folder::markUnseen, Core\Imap\Folder::flag, Core\Imap\Folder::unflag, Core\Imap\Folder::delete
#[cfg_attr(miri, ignore = "reads a directory, which Miri does not emulate")]
#[test]
fn a_directory_block_moves_flags_and_deletes_files() {
    let dir = nvs_repo::scratch("imap-directory-changes");
    let mailbox = mailbox(&dir);
    let inbox = dir.join("INBOX");
    let archive = dir.join("Archive");
    let flags = |path: &Path| std::fs::read_to_string(path).ok();

    mailbox
        .store("INBOX", 1, "\\Seen", true, MEMBER)
        .expect("a flag");
    mailbox
        .store("INBOX", 1, "$Important", true, MEMBER)
        .expect("a keyword");
    mailbox
        .store("INBOX", 1, "\\seen", true, MEMBER)
        .expect("the same flag again");
    assert_eq!(
        flags(&inbox.join("1.flags")).as_deref(),
        Some("\\Seen\n$Important\n")
    );
    mailbox
        .store("INBOX", 1, "\\SEEN", false, MEMBER)
        .expect("a flag removed");
    mailbox
        .store("INBOX", 1, "$important", false, MEMBER)
        .expect("a keyword removed");
    assert_eq!(flags(&inbox.join("1.flags")), None);
    let Err(fault) = mailbox.store("INBOX", 3, "\\Seen", true, MEMBER) else {
        panic!("a missing message was flagged");
    };
    assert!(
        format!("{fault:?}").contains("no message with the UID 3"),
        "{fault:?}"
    );
    assert!(!inbox.join("3.flags").exists());

    mailbox
        .move_to("INBOX", 2, "Archive", MEMBER)
        .expect("message 2 moves");
    mailbox
        .move_to("INBOX", 10, "Archive", MEMBER)
        .expect("message 10 moves");
    assert!(!inbox.join("2.eml").exists() && !inbox.join("2.flags").exists());
    assert!(archive.join("1.eml").is_file() && archive.join("2.eml").is_file());
    assert_eq!(flags(&archive.join("1.flags")).as_deref(), Some("\\Seen\n"));
    assert!(flags(&archive.join("2.eml")).is_some_and(|text| text.contains("Re: invoice 7")));
    let Err(fault) = mailbox.move_to("INBOX", 1, "Spam", MEMBER) else {
        panic!("a message moved to a folder that does not exist");
    };
    assert!(
        format!("{fault:?}").contains("no folder `Spam`"),
        "{fault:?}"
    );
    assert!(inbox.join("1.eml").is_file());

    mailbox
        .store("INBOX", 1, "\\Flagged", true, MEMBER)
        .expect("a flag");
    mailbox
        .delete("INBOX", 1, MEMBER)
        .expect("message 1 is deleted");
    assert!(!inbox.join("1.eml").exists() && !inbox.join("1.flags").exists());
    assert!(
        inbox.join("010.eml").is_file(),
        "only its own files are removed"
    );
    let Err(fault) = mailbox.delete("INBOX", 1, MEMBER) else {
        panic!("a message was deleted twice");
    };
    assert!(
        format!("{fault:?}").contains("no message with the UID 1"),
        "{fault:?}"
    );
}
