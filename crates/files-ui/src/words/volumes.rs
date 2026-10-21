//! What is said when a disk would not eject or open, in Finder's words. Pure.

use files_core::{EjectFailure, MountFailure, VolumeName, VolumeTrouble};

/// The sentence for a disk that did not eject or open.
///
/// A busy disk says so as Finder does, and names the programs when they could be found.
pub fn volume_trouble_words(name: &VolumeName, trouble: &VolumeTrouble) -> String {
    let disk = format!("The disk \u{201c}{name}\u{201d}");
    match trouble {
        VolumeTrouble::NotEjected(EjectFailure::InUse(programs)) => {
            let busy = format!(
                "{disk} wasn\u{2019}t ejected because one or more programs may be using it"
            );
            if programs.is_empty() {
                format!("{busy}.")
            } else {
                let names: Vec<&str> = programs.iter().map(|p| p.as_str()).collect();
                format!("{busy}: {}.", names.join(", "))
            }
        }
        VolumeTrouble::NotEjected(EjectFailure::NotAllowed) => {
            format!("{disk} wasn\u{2019}t ejected because you don\u{2019}t have permission.")
        }
        VolumeTrouble::NotEjected(EjectFailure::Gone) => {
            format!("{disk} is not there any more.")
        }
        VolumeTrouble::NotEjected(EjectFailure::Other | _) => {
            format!("{disk} wasn\u{2019}t ejected.")
        }
        VolumeTrouble::NotOpened(MountFailure::NotAllowed) => {
            format!("{disk} wasn\u{2019}t opened because you don\u{2019}t have permission.")
        }
        VolumeTrouble::NotOpened(MountFailure::Gone) => {
            format!("{disk} is not there any more.")
        }
        VolumeTrouble::NotOpened(MountFailure::Other | _) => {
            format!("{disk} couldn\u{2019}t be opened.")
        }
        // A trouble files-core adds later is a plain failure until this names it.
        _ => format!("{disk} couldn\u{2019}t be used."),
    }
}
