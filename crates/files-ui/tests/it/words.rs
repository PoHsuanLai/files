//! Sizes, kinds and dates in words: tables.

use std::time::SystemTime;

use chrono::{TimeZone, Utc};
use files_core::{
    ByteSize, Entry, EntryKind, FileName, FolderPath, Info, ItemCount, ItemRef, LinkTo, Special,
};
use files_ui::Zone;
use files_ui::words::{
    NONE, announce_words, busy_words, conflict_words, date_words, entry_size_words, erase_words,
    exact_size_words, info_rows, items_words, kind_words, name_rows, size_words,
};

fn entry(name: &str, kind: EntryKind) -> Entry {
    Entry::new(FileName::new(name).unwrap(), kind)
}

#[test]
fn sizes_are_said_as_finder_says_them() {
    const CASES: &[(&str, u64, &str)] = &[
        ("empty", 0, "Zero bytes"),
        ("one byte", 1, "1 byte"),
        ("a few bytes", 512, "512 bytes"),
        ("the most bytes", 999, "999 bytes"),
        ("a thousand is a kilobyte", 1_000, "1 KB"),
        ("kilobytes are whole", 4_096, "4 KB"),
        ("kilobytes round", 1_499, "1 KB"),
        ("kilobytes round up", 1_500, "2 KB"),
        ("the most kilobytes", 999_499, "999 KB"),
        ("rounds into megabytes", 999_999, "1 MB"),
        ("a whole megabyte drops its decimal", 1_000_000, "1 MB"),
        ("megabytes keep one decimal", 1_500_000, "1.5 MB"),
        ("megabytes round to a tenth", 123_456_789, "123.5 MB"),
        ("gigabytes", 12_340_000_000, "12.3 GB"),
        ("terabytes", 2_000_000_000_000, "2 TB"),
        ("the biggest", u64::MAX, "18446.7 PB"),
    ];
    for &(name, bytes, want) in CASES {
        assert_eq!(size_words(ByteSize(bytes)), want, "{name}");
    }
}

#[test]
fn only_files_have_a_size_to_show() {
    let cases = [
        (
            "a file",
            entry("a.txt", EntryKind::File(ByteSize(2_000))),
            "2 KB",
        ),
        ("a folder", entry("d", EntryKind::Folder), NONE),
        (
            "a link to a file",
            entry("l", EntryKind::Link(LinkTo::File(ByteSize(5)))),
            "5 bytes",
        ),
        (
            "a link to a folder",
            entry("l", EntryKind::Link(LinkTo::Folder)),
            NONE,
        ),
        (
            "a broken link",
            entry("l", EntryKind::Link(LinkTo::Broken)),
            NONE,
        ),
        (
            "a pipe",
            entry("p", EntryKind::Special(Special::Fifo)),
            NONE,
        ),
    ];
    for (name, entry, want) in cases {
        assert_eq!(entry_size_words(&entry), want, "{name}");
    }
}

#[test]
fn kinds_are_said_from_the_entry_and_its_extension() {
    let file = |name: &str| entry(name, EntryKind::File(ByteSize(1)));
    let cases = [
        ("a folder", entry("d", EntryKind::Folder), "Folder"),
        ("a pdf", file("Report.pdf"), "PDF file"),
        (
            "the last extension counts",
            file("archive.tar.gz"),
            "GZ file",
        ),
        ("case is folded", file("PHOTO.JpG"), "JPG file"),
        ("no extension", file("Makefile"), "Document"),
        ("a dotfile has none", file(".bashrc"), "Document"),
        ("a trailing dot has none", file("odd."), "Document"),
        (
            "a link to a file",
            entry("l", EntryKind::Link(LinkTo::File(ByteSize(1)))),
            "Alias",
        ),
        (
            "a link to a folder",
            entry("l", EntryKind::Link(LinkTo::Folder)),
            "Alias",
        ),
        (
            "a broken link",
            entry("l", EntryKind::Link(LinkTo::Broken)),
            "Broken alias",
        ),
        (
            "a pipe",
            entry("p", EntryKind::Special(Special::Fifo)),
            "Pipe",
        ),
        (
            "a socket",
            entry("s", EntryKind::Special(Special::Socket)),
            "Socket",
        ),
        (
            "a device",
            entry("v", EntryKind::Special(Special::Device)),
            "Device",
        ),
    ];
    for (name, entry, want) in cases {
        assert_eq!(kind_words(&entry), want, "{name}");
    }
}

#[test]
fn dates_are_written_in_full_and_a_missing_one_is_a_dash() {
    let at = Utc.with_ymd_and_hms(2026, 10, 21, 14, 3, 59).unwrap();
    assert_eq!(date_words(at), "2026-10-21 14:03");
    let early = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 0).unwrap();
    assert_eq!(date_words(early), "2026-01-02 03:04");

    let system: SystemTime = at.into();
    assert_eq!(Zone::Utc.words(Some(system)), "2026-10-21 14:03");
    assert_eq!(Zone::Utc.words(None), NONE);
}

fn summary(kind: files_core::OpKind, count: usize, first: &str) -> files_core::Summary {
    let first = (count > 0).then(|| FileName::new(first).unwrap());
    files_core::Summary::new(kind, count, first)
}

#[test]
fn a_move_to_the_trash_is_announced_as_finder_does() {
    use files_core::OpKind::Trash;
    assert_eq!(
        announce_words(&summary(Trash, 1, "report.pdf")),
        "Moved \u{201c}report.pdf\u{201d} to the Trash"
    );
    assert_eq!(
        announce_words(&summary(Trash, 3, "report.pdf")),
        "Moved 3 items to the Trash"
    );
}

#[test]
fn work_in_progress_says_what_it_is() {
    use files_core::OpKind::{Copy, Erase, Trash};
    let table = [
        (summary(Copy, 1, "a.txt"), "Copying \u{201c}a.txt\u{201d}"),
        (summary(Copy, 12, "a.txt"), "Copying 12 items"),
        (summary(Trash, 2, "a"), "Moving 2 items to the Trash"),
        (summary(Erase, 0, ""), "Erasing"),
    ];
    for (given, want) in table {
        assert_eq!(busy_words(&given), want);
    }
}

#[test]
fn a_taken_name_is_asked_in_finders_words() {
    use files_core::{Conflict, ConflictKind};
    let here = files_core::FolderPath::new("/d").unwrap();
    let ask = |kind| Conflict::new(FileName::new("a.txt").unwrap(), here.clone(), kind, 0);
    let (title, message) = conflict_words(&ask(ConflictKind::Move));
    assert_eq!(
        title,
        "An item named \u{201c}a.txt\u{201d} already exists in this location."
    );
    assert_eq!(
        message,
        "Do you want to replace it with the one you're moving?"
    );
    let (_, copying) = conflict_words(&ask(ConflictKind::Copy));
    assert_eq!(
        copying,
        "Do you want to replace it with the one you're copying?"
    );
}

#[test]
fn erasing_asks_about_what_it_erases() {
    assert_eq!(
        erase_words(None),
        "Are you sure you want to permanently erase the items in the Trash?"
    );
    assert_eq!(
        erase_words(Some((1, "a.txt".to_owned()))),
        "Are you sure you want to permanently erase \u{201c}a.txt\u{201d}?"
    );
    assert_eq!(
        erase_words(Some((4, "a.txt".to_owned()))),
        "Are you sure you want to permanently erase these 4 items?"
    );
}

#[test]
fn get_info_says_the_words_and_then_the_exact_bytes() {
    const CASES: &[(&str, u64, &str)] = &[
        ("empty", 0, "Zero bytes"),
        ("a few", 999, "999 bytes"),
        ("a thousand", 1_000, "1 KB (1,000 bytes)"),
        ("kilobytes", 12_345, "12 KB (12,345 bytes)"),
        ("megabytes", 1_500_000, "1.5 MB (1,500,000 bytes)"),
        ("a million exactly", 1_000_000, "1 MB (1,000,000 bytes)"),
        (
            "groups of three",
            123_456_789,
            "123.5 MB (123,456,789 bytes)",
        ),
    ];
    for &(name, bytes, want) in CASES {
        assert_eq!(exact_size_words(ByteSize(bytes)), want, "{name}");
    }
    let counts = [(0, "Zero items"), (1, "1 item"), (2, "2 items")];
    for (count, want) in counts {
        assert_eq!(items_words(ItemCount(count)), want);
    }
}

#[test]
fn get_info_rows_follow_the_kind_of_item() {
    let item = |name: &str| {
        ItemRef::new(
            FolderPath::new("/home/p").unwrap(),
            FileName::new(name).unwrap(),
        )
    };
    let when = |at: Option<SystemTime>| at.map_or(NONE.to_owned(), |_| "then".to_owned());
    let labels = |info: &Info| -> Vec<&'static str> {
        info_rows(info, when).iter().map(|row| row.label).collect()
    };
    let file = Info::new(item("Report.pdf"), EntryKind::File(ByteSize(12_345)));
    assert_eq!(labels(&file), ["Kind", "Size", "Where", "Modified"]);
    let rows = info_rows(&file, when);
    assert_eq!(rows[0].value, "PDF file");
    assert_eq!(rows[1].value, "12 KB (12,345 bytes)");
    assert_eq!(rows[2].value, "/home/p");
    assert_eq!(rows[3].value, NONE);

    let made = SystemTime::UNIX_EPOCH;
    let folder = Info::new(item("Projects"), EntryKind::Folder)
        .created_at(made)
        .modified_at(made)
        .holding(ItemCount(3));
    assert_eq!(
        labels(&folder),
        ["Kind", "Contents", "Where", "Created", "Modified"]
    );
    assert_eq!(info_rows(&folder, when)[1].value, "3 items");

    let names = |info: &Info| -> Vec<(&'static str, String)> {
        name_rows(info)
            .into_iter()
            .map(|row| (row.label, row.value))
            .collect()
    };
    assert_eq!(
        names(&file),
        [
            ("Name", "Report.pdf".to_owned()),
            ("Extension", "pdf".to_owned())
        ]
    );
    assert_eq!(names(&folder), [("Name", "Projects".to_owned())]);
}
