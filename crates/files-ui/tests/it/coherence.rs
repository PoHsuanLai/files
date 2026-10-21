//! The four coherence rules (quire's `CONSUMING.md` section 5) for files' window, as quire's
//! consumer example proves them (`examples/consumer/tests/it/coherence.rs` at the pin):
//! 1. [`our_stylesheet_lints_clean`]: no literal design value in the window's own CSS.
//! 2. [`rendered_markup_uses_only_styled_classes`] and [`no_raw_form_control_or_svg_is_rendered`]:
//!    no raw markup, no class nothing styles, in the real window with a folder listed and with a
//!    name being typed.
//! 3. Rule 3 (a missing component is added to quire first) is a review discipline, not a check;
//!    the gaps are listed in the lane's report.
//! 4. Rule 4 (motion only through quire's timers): the window draws no motion of its own.

use std::sync::Arc;
use std::time::Duration;

use ds::prelude::ShortcutKey;
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use ds_lint::{LintConfig, Profile, Rule, assert_clean, markup};
use files_ui::STYLE;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

/// What `markup` needs to know is styled: quire's own stylesheet, plus the window's.
fn window_css() -> String {
    format!("{}\n{STYLE}", ds::stylesheet())
}

/// The window with the scratch folder listed. `renaming` also begins a rename, so the name
/// field is on the page.
fn page(renaming: bool) -> String {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let mut window: Harness = harness(launch(dir.path(), config.path(), &opener));
    settle_until(&mut window, |h| h.html().contains("zebra.png"));
    if renaming {
        let at = window.centre(".ds-vtable-item[*|data-row=\"0\"]").unwrap();
        window.send(Input::click(at));
        window.advance(Duration::from_millis(50));
        window.send(Input::key(ShortcutKey::Enter));
        settle_until(&mut window, |h| h.count(".ds-text-field") > 0);
    }
    window.html()
}

/// Rule 1: the window's CSS under the strictest profile, the call `CONSUMING.md` section 5 shows.
#[test]
fn our_stylesheet_lints_clean() {
    assert_clean(
        STYLE,
        &LintConfig {
            profile: Profile::Strict,
            ..LintConfig::new(&ds::kits())
        },
    );
}

/// Rule 2: every class the page carries is one quire's stylesheet or the window's styles.
#[test]
fn rendered_markup_uses_only_styled_classes() {
    for renaming in [false, true] {
        let html = page(renaming);
        let offences = markup(&html, &window_css(), &LintConfig::new(&ds::kits()));
        let unstyled: Vec<_> = offences
            .iter()
            .filter(|offence| offence.rule == Rule::UnstyledClass)
            .collect();
        assert!(unstyled.is_empty(), "{unstyled:#?}\nin:\n{html}");
    }
}

/// Rule 2: no hand-written `<button>`, `<input>`, `<select>`, `<textarea>` or `<svg>`.
#[test]
fn no_raw_form_control_or_svg_is_rendered() {
    for renaming in [false, true] {
        let html = page(renaming);
        let offences = markup(&html, &window_css(), &LintConfig::new(&ds::kits()));
        let raw: Vec<_> = offences
            .iter()
            .filter(|offence| offence.rule == Rule::RawMarkup)
            .collect();
        assert!(raw.is_empty(), "{raw:#?}\nin:\n{html}");
    }
}

/// Rule 2's inline-style half: the window writes no style attribute of its own with a literal
/// colour or duration (quire's own custom properties are allowed).
#[test]
fn no_inline_style_carries_a_literal_colour_or_duration() {
    let html = page(true);
    let offences = markup(&html, &window_css(), &LintConfig::new(&ds::kits()));
    let inline: Vec<_> = offences
        .iter()
        .filter(|offence| {
            matches!(
                offence.rule,
                Rule::HexColour | Rule::ColourFunction | Rule::NamedColour | Rule::RawDuration
            )
        })
        .collect();
    assert!(inline.is_empty(), "{inline:#?}\nin:\n{html}");
}
