//! The person's icon theme, behind the launcher's [`IconLookup`]: which theme is set is read from
//! the GTK and KDE settings files once, and icons are looked up in it with `freedesktop-icons`.

use std::path::{Path, PathBuf};

use files_desktop::IconLookup;

/// The theme every other one falls back to, and the one used when none is set.
const FALLBACK: &str = "hicolor";

/// How big the pictures are asked for, in pixels: twice the 16 px a menu draws, so they are sharp
/// on a scaled screen.
const SIZE: u16 = 32;

/// The image kinds quire's external icons draw.
const DRAWN: [&str; 2] = ["png", "svg"];

/// Finds icons in the person's theme, then in hicolor and the pixmaps folders.
///
/// `freedesktop-icons` finds the icon folders itself, from `$XDG_DATA_DIRS` and the home folder,
/// once per process; only the theme's name is passed in here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeIcons {
    theme: String,
}

impl ThemeIcons {
    /// Icons in the theme called `theme` (its folder name), or in hicolor when it is empty.
    pub fn new(theme: &str) -> ThemeIcons {
        let theme = if theme.is_empty() { FALLBACK } else { theme };
        ThemeIcons {
            theme: theme.to_owned(),
        }
    }

    /// The theme the settings files under `config_home` name: GTK 3's `settings.ini`, else KDE's
    /// `kdeglobals`, else hicolor.
    pub fn from_settings(config_home: &Path) -> ThemeIcons {
        let read = |file: &str| std::fs::read_to_string(config_home.join(file)).unwrap_or_default();
        let named = key_in(
            &read("gtk-3.0/settings.ini"),
            "Settings",
            "gtk-icon-theme-name",
        )
        .or_else(|| key_in(&read("kdeglobals"), "Icons", "Theme"));
        ThemeIcons::new(named.as_deref().unwrap_or_default())
    }
}

impl IconLookup for ThemeIcons {
    fn find(&self, name: &str) -> Option<PathBuf> {
        freedesktop_icons::lookup(name)
            .with_theme(&self.theme)
            .with_size(SIZE)
            .find()
            .filter(|file| drawn(file))
    }
}

/// Whether quire can draw the file: a PNG or an SVG.
fn drawn(file: &Path) -> bool {
    file.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            DRAWN
                .iter()
                .any(|kind| extension.eq_ignore_ascii_case(kind))
        })
}

/// The value of `key` in `[section]` of a key file; `None` when it is not there or is empty.
fn key_in(text: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            inside = header == section;
        } else if inside && let Some((name, value)) = line.split_once('=') {
            let value = value.trim();
            if name.trim() == key && !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_read_from_its_own_section_only() {
        let text = "[Other]\nTheme=wrong\n\n[Icons]\n# comment\nTheme = breeze-dark \n";
        // (section, key, what is read)
        let table = [
            ("Icons", "Theme", Some("breeze-dark")),
            ("Other", "Theme", Some("wrong")),
            ("Icons", "Size", None),
            ("Missing", "Theme", None),
        ];
        for (section, key, want) in table {
            assert_eq!(
                key_in(text, section, key).as_deref(),
                want,
                "{section} {key}"
            );
        }
        assert_eq!(key_in("[Icons]\nTheme=\n", "Icons", "Theme"), None);
    }

    #[test]
    fn the_theme_comes_from_gtk_then_kde_then_hicolor() {
        let gtk = "[Settings]\ngtk-icon-theme-name=Papirus\n";
        let kde = "[Icons]\nTheme=breeze\n";
        // (gtk settings, kdeglobals, theme)
        let table = [
            (Some(gtk), Some(kde), "Papirus"),
            (None, Some(kde), "breeze"),
            (None, None, "hicolor"),
        ];
        for (gtk, kde, want) in table {
            let home = tempfile::tempdir().unwrap();
            if let Some(text) = gtk {
                std::fs::create_dir(home.path().join("gtk-3.0")).unwrap();
                std::fs::write(home.path().join("gtk-3.0/settings.ini"), text).unwrap();
            }
            if let Some(text) = kde {
                std::fs::write(home.path().join("kdeglobals"), text).unwrap();
            }
            assert_eq!(ThemeIcons::from_settings(home.path()).theme, want);
        }
    }

    #[test]
    fn only_png_and_svg_are_drawn() {
        let table = [
            ("/i/a.png", true),
            ("/i/a.SVG", true),
            ("/i/a.xpm", false),
            ("/i/a", false),
        ];
        for (file, want) in table {
            assert_eq!(drawn(Path::new(file)), want, "{file}");
        }
    }
}
