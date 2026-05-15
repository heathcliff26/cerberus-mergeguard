# cerberus-mergeguard Helm Chart

This Helm chart deploys the cerberus-mergeguard - Block GitHub pull request merges until all status checks have passed

## Prerequisites

- Kubernetes 1.34+
- Helm 3.19+
- FluxCD installed in the cluster (recommended)

## Installation

### Installing from OCI Registry (GitHub Packages)

```bash
# Install the chart
helm install cerberus-mergeguard oci://ghcr.io/heathcliff26/manifests/cerberus-mergeguard --version <version>
```

## Configuration

### Minimal Configuration (No Ingress)

You need to provide a secret for the github app's private key.
```yaml
config:
  github:
    client-id: "<your-app-id>"
    private-key: "/config/app-private-key.pem"
volumes:
  - name: key
    secret:
      secretName: cerberus-mergeguard-key
      items:
        - key: private-key.pem
          path: app-private-key.pem
volumeMounts:
  - name: key
    mountPath: /config/app-private-key.pem
    subPath: app-private-key.pem
```

## Values Reference

See [values.yaml](./values.yaml) for all available configuration options.

### Key Parameters

| Parameter                | Description                                         | Default                                   |
| ------------------------ | --------------------------------------------------- | ----------------------------------------- |
| `image.repository`       | Container image repository                          | `ghcr.io/heathcliff26/cerberus-mergeguard` |
| `image.tag`              | Container image tag                                 | Same as chart version                     |
| `ingress.enabled`        | Enable ingress                                      | `false`                                   |

## Support

For more information, visit: https://github.com/heathcliff26/cerberus-mergeguard
