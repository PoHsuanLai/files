//! The words cloud drives are named by: small text newtypes, so a drive's id is never mistaken
//! for its label.

use std::fmt;

macro_rules! text {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(String);

        impl $name {
            /// The text `text`.
            pub fn new(text: impl Into<String>) -> $name {
                $name(text.into())
            }

            /// The text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

text! {
    /// What the service calls a drive, kept opaque.
    DriveId
}
text! {
    /// A drive's provider as people say it: "OneDrive", "Google Drive".
    ProviderName
}
text! {
    /// The person's own words for the account behind a drive (often an address).
    AccountLabel
}
text! {
    /// What went wrong, or why an ask was refused, in words a person reads.
    CloudWords
}
