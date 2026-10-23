//! Why an answer of the cloud service could not be used.

/// Why the cloud service could not be asked, or an answer of it not read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    /// The service did not answer, or is not there.
    #[error("The cloud drives can't be reached right now: {0}")]
    Bus(#[from] zbus::Error),
    /// An answer lacks something every answer has.
    #[error("A cloud answer has no {0}.")]
    Missing(&'static str),
    /// An answer says a word this version does not know (a newer service's).
    #[error("A cloud answer says {word:?} for its {part}, which this version does not know.")]
    Unknown {
        /// What the word describes.
        part: &'static str,
        /// The word.
        word: String,
    },
    /// An answer names a path that is not a full path.
    #[error("A cloud answer holds {0:?}, which is not a full path.")]
    Path(String),
}
