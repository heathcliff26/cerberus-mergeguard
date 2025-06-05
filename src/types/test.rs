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

#[test]
fn parse_pull_request_event() {
    let test_body = include_str!("testdata/pr-synchronize.json");

    let event: PullRequestEvent = match serde_json::from_str(test_body) {
        Ok(check_runs) => check_runs,
        Err(e) => panic!("Failed to parse pull_request event: {e}"),
    };

    assert_eq!("synchronize", event.action);
}

#[test]
fn parse_check_run_event() {
    let test_body = include_str!("testdata/own-check-run-event.json");

    let event: CheckRunEvent = match serde_json::from_str(test_body) {
        Ok(check_runs) => check_runs,
        Err(e) => panic!("Failed to parse check_run event: {e}"),
    };

    assert_eq!("completed", event.action);
}

#[test]
fn check_run_new() {
    let run = CheckRun::new("test-sha");

    assert_eq!("test-sha", run.head_sha);

    check_run_assert_initial_fields(&run);
}

fn check_run_assert_initial_fields(run: &CheckRun) {
    assert_eq!(CHECK_RUN_NAME, run.name);
    assert_eq!(CHECK_RUN_INITIAL_STATUS, run.status);
    let output = run.output.as_ref().expect("Should have output");
    assert!(output.title.is_some(), "Should have title");
    assert_eq!(
        CHECK_RUN_SUMMARY,
        output.summary.as_ref().expect("Should have summary")
    );
    assert!(run.conclusion.is_none(), "Conclusion should be None");
}

#[test]
fn check_run_update_status() {
    let mut run = CheckRun::new("test-sha");

    assert!(run.update_status(0), "Should have changed status");
    assert_eq!(CHECK_RUN_NAME, run.name);
    assert_eq!(CHECK_RUN_COMPLETED_STATUS, run.status);
    assert_eq!(
        CHECK_RUN_CONCLUSION,
        run.conclusion.as_ref().expect("Should have conclusion")
    );
    let output = run.output.as_ref().expect("Should have output");
    assert_eq!(
        CHECK_RUN_COMPLETED_TITLE,
        output.title.as_ref().expect("Should have title")
    );
    assert_eq!(
        CHECK_RUN_SUMMARY,
        output.summary.as_ref().expect("Should have summary")
    );

    assert!(run.update_status(10), "Should have changed status again");
    check_run_assert_initial_fields(&run);

    assert!(
        !run.update_status(10),
        "Should not have changed status again"
    );
}

#[test]
fn parse_token_response() {
    let test_body = include_str!("testdata/token-response.json");

    let token: TokenResponse = match serde_json::from_str(test_body) {
        Ok(token) => token,
        Err(e) => panic!("Failed to parse token: {e}"),
    };

    assert_eq!("ghs_16C7e42F292c6912E7710c838347Ae178B4a", token.token);
}
