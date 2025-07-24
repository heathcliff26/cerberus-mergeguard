use super::*;

#[test]
fn test_periodic_refresh() {
    let cfg = match Configuration::load("src/config/testdata/periodic-refresh.yaml") {
        Ok(cfg) => cfg,
        Err(e) => panic!("Failed to load configuration: {e:?}"),
    };

    println!("Loaded configuration: {cfg:?}");

    assert_eq!(
        60, cfg.server.periodic_refresh,
        "Periodic refresh rate should be set to 60 seconds"
    );
}

#[test]
fn test_config_without_log_level() {
    // Test loading a config file with no log level set (uses default)
    let cfg = match Configuration::load("src/config/testdata/periodic-refresh.yaml") {
        Ok(cfg) => cfg,
        Err(e) => panic!("Failed to load configuration: {e:?}"),
    };

    assert_eq!(
        cfg.log_level, "info",
        "Should use default log level when not specified"
    );
}

#[test]
fn test_load_nonexistent_file() {
    let result = Configuration::load("/nonexistent/path/config.yaml");
    assert!(result.is_err());
    match result {
        Err(Error::ReadConfigFile(path, _)) => {
            assert_eq!(path, "/nonexistent/path/config.yaml");
        }
        _ => panic!("Expected ReadConfigFile error"),
    }
}
