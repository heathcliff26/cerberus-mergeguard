use super::*;

#[tokio::test]
async fn ignore_own_check_run() {
    let test_body = include_str!("../types/testdata/own-check-run-event.json");

    let github = Client::new_for_testing(
        "test-client-id",
        "test-client-secret",
        "https://noops.example.com",
    );

    let (status, response) = handle_check_run_event(&github, test_body).await;
    if status != StatusCode::OK {
        panic!(
            "Should have ignored event and returned OK, got: {}, message={:?}",
            status, response
        );
    }
}
