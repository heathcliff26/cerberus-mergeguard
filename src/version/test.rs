use super::*;

#[test]
fn test_version_information() {
    let info = version_information();
    assert!(info.contains(NAME));
    assert!(info.contains(VERSION));
    assert!(info.contains("Commit:"));
    assert!(info.contains(COMMIT.unwrap_or("unknown")));
}
