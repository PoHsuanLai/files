//! What is said when a disk would not eject or open, in Finder's words. Pure.

use files_core::{EjectFailure, MountFailure, ProgramName, VolumeName, VolumeTrouble};

/// The sentence for a disk that did not eject or open.
///
/// A busy disk says so as Finder does, and names the programs when they could be found.
pub fn volume_trouble_words(name: &VolumeName, trouble: &VolumeTrouble) -> String {
    let disk = format!("The disk \u{201c}{name}\u{201d}");
    match trouble {
        VolumeTrouble::NotEjected(failure) => match failure {
            EjectFailure::InUse(programs) => {
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
            EjectFailure::NotAllowed => {
                format!("{disk} wasn\u{2019}t ejected because you don\u{2019}t have permission.")
            }
            EjectFailure::Gone => format!("{disk} is not there any more."),
            EjectFailure::Other | _ => format!("{disk} wasn\u{2019}t ejected."),
        },
        VolumeTrouble::NotOpened(failure) => match failure {
            MountFailure::NotAllowed => {
                format!("{disk} wasn\u{2019}t opened because you don\u{2019}t have permission.")
            }
            MountFailure::Gone => format!("{disk} is not there any more."),
            MountFailure::Other | _ => format!("{disk} couldn\u{2019}t be opened."),
        },
        // A trouble files-core adds later is a plain failure until this names it.
        _ => format!("{disk} couldn\u{2019}t be used."),
    }
}

/// Finder's question for a disk that is in use: the title, and the programs found as the
/// informative text (a line about forcing when none could be found).
pub fn force_eject_words(name: &VolumeName, programs: &[ProgramName]) -> (String, String) {
    let title = format!(
        "The disk \u{201c}{name}\u{201d} wasn\u{2019}t ejected because one or more programs may be \
         using it."
    );
    let message = if programs.is_empty() {
        "To eject the disk immediately, click Force Eject.".to_owned()
    } else {
        let names: Vec<&str> = programs.iter().map(|p| p.as_str()).collect();
        format!("Programs using it: {}.", names.join(", "))
    };
    (title, message)
}
