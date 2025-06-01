use reqwest::Client;
use std::error::Error;
use std::process::Command;
use std::sync::Once;

static CONTAINER_BUILD: Once = Once::new();
static CONTAINER_IMAGE: &str = "localhost/cerberus-mergeguard:e2e-test";

#[tokio::test]
async fn container_image_healthcheck_http() {
    let _cert = TlcCertificate::create("tests/e2e/testdata/http/testapp");
    let _container =
        RunningContainer::setup("cerberus-http", "8080:8080", "./tests/e2e/testdata/http/").await;

    let url = "http://localhost:8080/healthz";

    let response = Client::new().get(url).send().await;
    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_success(),
                "Health check failed: {}",
                resp.status()
            );
        }
        Err(e) => {
            panic!("Failed to perform health check: {}", full_error_stack(&e));
        }
    }
}

#[tokio::test]
async fn container_image_healthcheck_https() {
    let _app_cert = TlcCertificate::create("tests/e2e/testdata/https/testapp");
    let server_cert = TlcCertificate::create("tests/e2e/testdata/https/server");
    let _container =
        RunningContainer::setup("cerberus-https", "8443:8443", "./tests/e2e/testdata/https/").await;

    let url = "https://localhost:8443/healthz";

    let certificate = server_cert.certificate();
    let response = Client::builder()
        .add_root_certificate(certificate)
        .build()
        .expect("Failed to build HTTPS client")
        .get(url)
        .send()
        .await;
    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_success(),
                "Health check failed: {}",
                resp.status()
            );
        }
        Err(e) => {
            panic!("Failed to perform health check: {}", full_error_stack(&e));
        }
    }
}

fn build_image() {
    CONTAINER_BUILD.call_once(|| {
        // This function is called only once, even if multiple threads call it.
        // Here you would put the code to build your container image.
        println!("Building container image...");

        let output = Command::new("podman")
            .args(["build", "-t", CONTAINER_IMAGE, "."])
            .output()
            .expect("Failed to execute podman build command");

        if !output.status.success() {
            panic!(
                "Failed to build container image: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!("Container image built successfully.");
    });
}

fn full_error_stack(mut e: &dyn Error) -> String {
    let mut s = format!("{e}");
    while let Some(src) = e.source() {
        s.push_str(&format!(": {}", src));
        e = src;
    }
    s
}

struct RunningContainer {
    name: String,
}

impl RunningContainer {
    /// Start a container
    async fn setup(name: &str, port_binding: &str, config_dir: &str) -> Self {
        build_image();

        println!("Starting container: {}", name);
        let output = Command::new("podman")
            .args([
                "run",
                "-d",
                "-p",
                port_binding,
                "--name",
                name,
                "-v",
                format!("{config_dir}:/config:z").as_str(),
                CONTAINER_IMAGE,
            ])
            .output()
            .expect("Failed to execute podman run command");

        if !output.status.success() {
            panic!(
                "Failed to start container: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        println!("Container {} started successfully.", name);
        RunningContainer {
            name: name.to_string(),
        }
    }

    /// Print the container log
    fn log(&self) {
        println!("Fetching logs for container: {}", self.name);
        let output = Command::new("podman")
            .args(["logs", &self.name])
            .output()
            .expect("Failed to execute podman logs command");

        if !output.status.success() {
            panic!(
                "Failed to fetch logs for container: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!(
            "Logs for container {}:\nstdout:\n{}stderr:\n{}",
            self.name,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for RunningContainer {
    /// Stop and remove the container.
    fn drop(&mut self) {
        self.log();
        println!("Stopping and removing container: {}", self.name);
        let output = Command::new("podman")
            .args(["rm", "-f", &self.name])
            .output()
            .expect("Failed to execute podman rm command");

        if !output.status.success() {
            panic!(
                "Failed to remove container: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!("Container {} removed successfully.", self.name);
    }
}

struct TlcCertificate {
    key: String,
    crt: String,
}

impl TlcCertificate {
    /// Create a self signed TLS certificate and key pair.
    fn create(name: &str) -> Self {
        let key = format!("{name}.key").to_string();
        let crt = format!("{name}.crt").to_string();
        println!("Creating TLS certificate '{crt}' and key '{key}' ");
        let output = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-nodes",
                "-days",
                "1",
                "-newkey",
                "rsa:2048",
                "-keyout",
                &key,
                "-out",
                &crt,
                "-subj",
                "/CN=localhost",
            ])
            .output()
            .expect("Failed to execute openssl command");

        if !output.status.success() {
            panic!(
                "Failed to create TLS certificate: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let output = Command::new("chmod")
            .args(["644", &key])
            .output()
            .expect("Failed to execute chmod command");
        if !output.status.success() {
            panic!(
                "Failed to set permissions for TLS key: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        println!("TLS certificate created successfully.");
        TlcCertificate { key, crt }
    }
    /// Returns the certificate as a reqwest::tls::Certificate
    fn certificate(&self) -> reqwest::tls::Certificate {
        let cert_data = std::fs::read(&self.crt).expect("Failed to read TLS certificate file");
        reqwest::tls::Certificate::from_pem(&cert_data)
            .expect("Failed to create TLS certificate from PEM data")
    }
}

impl Drop for TlcCertificate {
    fn drop(&mut self) {
        println!("Removing TLS certificate: {}", self.crt);

        let res_key = std::fs::remove_file(&self.key);
        let res_crt = std::fs::remove_file(&self.crt);
        res_key.expect("Failed to remove TLS key file");
        res_crt.expect("Failed to remove TLS certificate file");

        println!("TLS certificate removed successfully.");
    }
}
