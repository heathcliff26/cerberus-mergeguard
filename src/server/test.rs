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

macro_rules! verify_webhook_test {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let (signature, secret, payload, res) = $value;

            let signature: Option<HeaderValue> = match signature {
                Some(sig) => Some(HeaderValue::from_str(sig).unwrap()),
                None => None,
            };

            let output = verify_webhook(signature.as_ref(), secret, payload);

            match res {
                Ok(()) => assert!(output.is_ok(), "Expected Ok, got: {:?}", output),
                Err(res) => {
                    if let Err((status, message)) = output {
                        let (res_status, res_message) = res;
                        assert_eq!(res_status, status, "Status code mismatch");
                        assert_eq!(res_message.message, message.message, "Wrong message");
                    } else {
                        panic!("Expected error, got Ok");
                    }
                },
            };
        }
    )*
    }
}

verify_webhook_test! {
    verify_webhook_valid_signature: (
        Some("sha256=2f94a757d2246073e26781d117ce0183ebd87b4d66c460494376d5c37d71985b"),
        Some("test-secret"),
        "test payload",
        verify_webhook_ok_result(),
    ),
    verify_webhook_invalid_signature: (
        Some("sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        Some("test-secret"),
        "test payload",
        verify_webhook_error_result("Invalid webhook signature"),
    ),
    verify_webhook_malformed_signature: (
        Some("sha256=invalid-signature"),
        Some("test-secret"),
        "test payload",
        verify_webhook_error_result("Invalid X-Hub-Signature-256 header"),
    ),
    verify_webhook_missing_signature: (
        None,
        Some("test-secret"),
        "test payload",
        verify_webhook_error_result("Missing X-Hub-Signature-256 header"),
    ),
    verify_webhook_no_secret: (
        Some("sha256=invalid-signature"),
        None,
        "test payload",
        verify_webhook_ok_result(),
    ),
    verify_webhook_no_secret_or_signature: (
        None,
        None,
        "test payload",
        verify_webhook_ok_result(),
    ),

}

fn verify_webhook_ok_result() -> Result<(), (StatusCode, Json<Response>)> {
    Ok(())
}

fn verify_webhook_error_result(message: &str) -> Result<(), (StatusCode, Json<Response>)> {
    Err((StatusCode::FORBIDDEN, Json(Response::error(message))))
}
