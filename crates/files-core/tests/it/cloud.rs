//! Cloud vocabulary: names among drives, shares and addresses.

use files_core::{
    AccountLabel, CloudDrive, DriveId, FolderPath, OnlineUrl, ProviderName, Share, UrlError,
};

fn drive(id: &str, provider: &str, label: &str, at: &str) -> CloudDrive {
    CloudDrive::new(
        DriveId::new(id),
        ProviderName::new(provider),
        FolderPath::new(at).unwrap(),
    )
    .labelled(AccountLabel::new(label))
}

#[test]
fn a_drive_is_named_by_its_provider_and_by_its_account_only_when_two_share_one() {
    let one = drive("a", "OneDrive", "ada@example.test", "/c/a");
    let two = drive("b", "OneDrive", "work@example.test", "/c/b");
    let google = drive("c", "Google Drive", "ada@example.test", "/c/c");
    let unlabelled = drive("d", "OneDrive", "", "/c/d");
    let all = [one.clone(), two.clone(), google.clone(), unlabelled.clone()];
    let cases = [
        ("shared provider", &one, "OneDrive (ada@example.test)"),
        ("the other account", &two, "OneDrive (work@example.test)"),
        ("a lone provider", &google, "Google Drive"),
        ("shared but no label", &unlabelled, "OneDrive"),
    ];
    for (name, drive, want) in cases {
        assert_eq!(drive.name_among(&all), want, "{name}");
    }
    assert_eq!(one.name_among(&[one.clone(), google]), "OneDrive");
}

#[test]
fn a_drive_holds_its_folder_and_what_is_inside_it() {
    let one = drive("a", "OneDrive", "", "/c/OneDrive");
    let cases = [
        ("itself", "/c/OneDrive", true),
        ("inside", "/c/OneDrive/Docs", true),
        ("a sibling with the same start", "/c/OneDrive2", false),
        ("elsewhere", "/home", false),
    ];
    for (name, path, want) in cases {
        assert_eq!(one.holds(&FolderPath::new(path).unwrap()), want, "{name}");
    }
}

#[test]
fn a_share_is_thousandths_kept_in_range() {
    let cases = [
        (0.0, 0),
        (0.5, 500),
        (0.0004, 0),
        (1.0, 1000),
        (3.0, 1000),
        (-1.0, 0),
    ];
    for (ratio, want) in cases {
        assert_eq!(Share::from_ratio(ratio).permille(), want, "{ratio}");
    }
    assert_eq!(Share::from_ratio(f64::NAN).permille(), 0);
}

#[test]
fn only_web_addresses_without_stray_characters_open_online() {
    assert!(OnlineUrl::new("https://docs.example.test/d/1").is_ok());
    assert_eq!(OnlineUrl::new("file:///etc/passwd"), Err(UrlError::NotWeb));
    assert_eq!(OnlineUrl::new("javascript:alert(1)"), Err(UrlError::NotWeb));
    assert_eq!(OnlineUrl::new("https://a b"), Err(UrlError::Stray));
}
