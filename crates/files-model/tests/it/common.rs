//! Small constructors the tests share.

use std::time::Duration;

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, Location};
use files_model::{Browser, Effect, Input, Now};

pub fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

pub fn file(text: &str) -> Entry {
    Entry::new(name(text), EntryKind::File(ByteSize(1)))
}

pub fn folder(text: &str) -> Entry {
    Entry::new(name(text), EntryKind::Folder)
}

pub fn dir(path: &str) -> Location {
    Location::Folder(FolderPath::new(path).unwrap())
}

pub fn at(ms: u64) -> Now {
    Now(Duration::from_millis(ms))
}

pub fn home() -> Location {
    dir("/home/p")
}

/// A browser at `/home/p` whose listing has arrived.
pub fn loaded(entries: Vec<Entry>) -> Browser {
    let (browser, _) = Browser::new(home());
    let (browser, _) = browser.step(Input::Listed(home(), Ok(entries)), at(0));
    browser
}

/// Step through `inputs` at time zero, collecting every effect.
pub fn run(browser: Browser, inputs: Vec<Input>) -> (Browser, Vec<Effect>) {
    inputs.into_iter().fold((browser, Vec::new()), |(b, mut all), input| {
        let (b, effects) = b.step(input, at(0));
        all.extend(effects);
        (b, all)
    })
}

/// The shown rows' names, in order.
pub fn shown(browser: &Browser) -> Vec<String> {
    browser.rows().iter().map(|e| e.name.display().into_owned()).collect()
}

/// The selected names, in name order.
pub fn selected(browser: &Browser) -> Vec<String> {
    browser.selection().names().map(|n| n.display().into_owned()).collect()
}

/// The path of a folder location.
pub fn path_of(location: &Location) -> String {
    match location {
        Location::Folder(p) => p.as_path().display().to_string(),
        other => format!("{other:?}"),
    }
}
