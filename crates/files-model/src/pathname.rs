//! Copy as Pathname: the full paths of the selection, as text.

use crate::browser::Browser;
use crate::effect::Effect;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// Ask for the full paths of the selected items to be put on the clipboard, one per line in
    /// shown order. With nothing selected, or outside a folder on disk, nothing happens.
    pub(crate) fn copy_as_pathname(self) -> Stepped {
        let lines: Vec<String> = self
            .selected_targets()
            .iter()
            .map(|target| target.path().display().to_string())
            .collect();
        if lines.is_empty() {
            return (self, Vec::new());
        }
        let effects = vec![Effect::CopyText(lines.join("\n"))];
        (self, effects)
    }
}
