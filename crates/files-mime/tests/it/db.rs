//! Aliases, subclasses with the implicit rules, and icon names.

use files_mime::{MimeDb, MimeLayer, Relation};

use crate::mt;

fn layer<'a>(aliases: &'a str, subclasses: &'a str, icons: &'a str, generic: &'a str) -> MimeLayer<'a> {
    MimeLayer { aliases, subclasses, icons, generic_icons: generic, ..MimeLayer::default() }
}

#[test]
fn aliases_resolve_and_the_higher_folder_wins() {
    let high = layer("text/x-py text/x-python\n", "", "", "");
    let low = layer("text/x-py text/x-other\ntext/x-rb text/x-ruby\n", "", "", "");
    let db = MimeDb::from_layers(&[high, low]);
    let rows = [
        ("text/x-py", "text/x-python"),
        ("text/x-rb", "text/x-ruby"),
        ("text/x-python", "text/x-python"),
    ];
    for (given, want) in rows {
        assert_eq!(db.canonical(&mt(given)), mt(want), "{given}");
    }
}

#[test]
fn subclasses_walk_transitively_with_implicit_rules() {
    let subclasses = "text/x-python3 text/x-python\ntext/x-python application/x-script\n\
                      application/x-script application/x-executable\n\
                      cyc/a cyc/b\ncyc/b cyc/a\n";
    let aliases = "text/x-py text/x-python\n";
    let db = MimeDb::from_layers(&[layer(aliases, subclasses, "", "")]);
    let rows = [
        ("text/x-python3", "text/x-python", Relation::Is),
        ("text/x-python3", "application/x-executable", Relation::Is),
        ("text/x-py", "application/x-script", Relation::Is),
        ("text/x-python3", "text/x-py", Relation::Is),
        ("application/x-executable", "text/x-python3", Relation::IsNot),
        // implicit: every text type is text/plain, every non-inode type is octet-stream
        ("text/x-unlisted", "text/plain", Relation::Is),
        ("text/plain", "text/plain", Relation::Is),
        ("image/png", "text/plain", Relation::IsNot),
        ("image/png", "application/octet-stream", Relation::Is),
        ("text/x-unlisted", "application/octet-stream", Relation::Is),
        ("inode/directory", "application/octet-stream", Relation::IsNot),
        ("cyc/a", "cyc/b", Relation::Is),
        ("cyc/a", "cyc/c", Relation::IsNot),
    ];
    for (child, parent, want) in rows {
        assert_eq!(db.is_a(&mt(child), &mt(parent)), want, "{child} is_a {parent}");
    }
}

#[test]
fn ancestors_are_nearest_first_and_end_with_octet_stream() {
    let db = MimeDb::from_layers(&[layer("", "text/x-python3 text/x-python\ntext/x-python text/plain\n", "", "")]);
    let want = vec![mt("text/x-python"), mt("text/plain"), mt("application/octet-stream")];
    assert_eq!(db.ancestors(&mt("text/x-python3")), want);
}

#[test]
fn icon_names_use_the_tables_then_fall_back() {
    let icons = "image/png:my-png\n";
    let generic = "image/png:image-x-generic\napplication/pdf:x-office-document\n";
    let aliases = "image/x-png image/png\n";
    let db = MimeDb::from_layers(&[layer(aliases, "", icons, generic)]);
    let rows = [
        ("image/png", "my-png", "image-x-generic"),
        ("image/x-png", "my-png", "image-x-generic"),
        ("application/pdf", "application-pdf", "x-office-document"),
        ("video/x-foo", "video-x-foo", "video-x-generic"),
    ];
    for (given, specific, generic) in rows {
        let icon = db.icon_name(&mt(given));
        assert_eq!((icon.specific(), icon.generic()), (specific, generic), "{given}");
        assert_eq!(icon.in_order(), [specific, generic]);
    }
}
