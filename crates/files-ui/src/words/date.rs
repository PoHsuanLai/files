//! Dates as the Date Modified column shows them.

use std::fmt::Display;

use chrono::{DateTime, TimeZone};

/// `at` as `2026-10-21 14:03`, in the zone `at` carries.
///
/// quire has no date formatter yet (the relative "Today, 14:03" Finder shows is its to add), so
/// the column writes the full date.
pub fn date_words<Tz>(at: DateTime<Tz>) -> String
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    at.format("%Y-%m-%d %H:%M").to_string()
}
