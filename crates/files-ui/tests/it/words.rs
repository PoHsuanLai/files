//! Sizes, kinds and dates in words: tables.

use std::time::SystemTime;

use chrono::{TimeZone, Utc};
use files_core::{ByteSize, Entry, EntryKind, FileName, LinkTo, Special};
use files_ui::Zone;
use files_ui::words::{NONE, date_words, entry_size_words, kind_words, size_words};

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
        ("a file", entry("a.txt", EntryKind::File(ByteSize(2_000))), "2 KB"),
        ("a folder", entry("d", EntryKind::Folder), NONE),
        (
            "a link to a file",
            entry("l", EntryKind::Link(LinkTo::File(ByteSize(5)))),
            "5 bytes",
        ),
        ("a link to a folder", entry("l", EntryKind::Link(LinkTo::Folder)), NONE),
        ("a broken link", entry("l", EntryKind::Link(LinkTo::Broken)), NONE),
        ("a pipe", entry("p", EntryKind::Special(Special::Fifo)), NONE),
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
        ("the last extension counts", file("archive.tar.gz"), "GZ file"),
        ("case is folded", file("PHOTO.JpG"), "JPG file"),
        ("no extension", file("Makefile"), "Document"),
        ("a dotfile has none", file(".bashrc"), "Document"),
        ("a trailing dot has none", file("odd."), "Document"),
        ("a link to a file", entry("l", EntryKind::Link(LinkTo::File(ByteSize(1)))), "Alias"),
        ("a link to a folder", entry("l", EntryKind::Link(LinkTo::Folder)), "Alias"),
        ("a broken link", entry("l", EntryKind::Link(LinkTo::Broken)), "Broken alias"),
        ("a pipe", entry("p", EntryKind::Special(Special::Fifo)), "Pipe"),
        ("a socket", entry("s", EntryKind::Special(Special::Socket)), "Socket"),
        ("a device", entry("v", EntryKind::Special(Special::Device)), "Device"),
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
