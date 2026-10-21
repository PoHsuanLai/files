//! The Trash as a trait.

use files_core::{ItemRef, TrashedItem};

use crate::error::TrashError;

/// The Trash. Real: [`SystemTrash`](crate::SystemTrash); for tests: `FakeTrash` (feature
/// `testing`).
pub trait Trash: Send + Sync {
    /// Moves `items` to the Trash and says where each now is.
    ///
    /// The real Trash does not hand its items back, so they are found by listing it afterwards
    /// and matching each by its original path with the newest deletion time. An item that cannot
    /// be matched that way (two items of the same path thrown away within one second) is left
    /// out of the answer; it is in the Trash all the same.
    fn trash(&self, items: &[ItemRef]) -> Result<Vec<TrashedItem>, TrashError>;

    /// Every item in the Trash, in no particular order. An item whose name or place cannot be
    /// read is left out.
    fn list(&self) -> Result<Vec<TrashedItem>, TrashError>;

    /// Moves `items` back to where they came from. Nothing is moved when something is already
    /// in the place of any of them: that is [`TrashError::Occupied`].
    fn put_back(&self, items: &[TrashedItem]) -> Result<(), TrashError>;

    /// Erases `items` for good.
    fn delete_for_good(&self, items: &[TrashedItem]) -> Result<(), TrashError>;

    /// Erases everything in the Trash for good.
    fn empty(&self) -> Result<(), TrashError>;
}
