use super::*;

#[test]
fn test_periodic_refresh() {
    let cfg = match Configuration::load("src/config/testdata/periodic-refresh.yaml") {
        Ok(cfg) => cfg,
        Err(e) => panic!("Failed to load configuration: {:?}", e),
    };

    println!("Loaded configuration: {:?}", cfg);

    assert_eq!(
        60, cfg.server.periodic_refresh,
        "Periodic refresh rate should be set to 60 seconds"
    );
}
