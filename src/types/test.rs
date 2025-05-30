use super::*;

#[test]
fn parse_check_runs() {
    let test_body = include_str!("testdata/check-runs-response.json");

    let check_runs: CheckRunsResponse = match serde_json::from_str(test_body) {
        Ok(check_runs) => check_runs,
        Err(e) => panic!("Failed to parse check runs response: {e}"),
    };

    assert_eq!(2, check_runs.total_count);
    assert_eq!(42974723261, check_runs.check_runs[0].id);
    assert_eq!(
        "1cda07a836f5567466f55c35a0838df4ee20b2f8",
        check_runs.check_runs[0].head_sha
    );
}
