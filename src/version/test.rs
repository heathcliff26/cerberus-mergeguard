use super::*;

#[test]
fn test_version_information() {
    let info = version_information();
    let lines: Vec<&str> = info.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains(NAME));
    assert!(lines[1].contains("Version:"));
    assert!(lines[2].contains("Commit:"));
}
