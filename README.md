# latency-probe

A Rust probe for monitor latency on multiple target host

## Features

- Support for multiple target types: ICMP, TCP, HTTP, and Echo
- Support for AWS AppConfig (on going) for dynamic configuration or local file
- Support for Prometheus metrics (Gauge, Histogram, Counter)
- Monotonic timestamps
- Low jitter

## Usage

Example config file:

```json
{
  "probe_interval_ms": 5000,
  "default_timeout_ms": 3000,
  "log_level": "error",
  "enable_latency_history": false,
  "targets": [
    {
      "name": "echo-websocket-icmp",
      "kind": "icmp",
      "host": "echo.websocket.org"
    },
    {
      "name": "echo-websocket-tcpconnect",
      "kind": "tcpconnect",
      "host": "echo.websocket.org",
      "port": 443
    },
    {
      "name": "echo-websocket-http",
      "kind": "http",
      "host": "https://echo.websocket.org",
      "port": 443
    }
  ]
}
```

```bash
export TARGET_CONFIG=sample-target.json

./latency-probe
```

## Build

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Ensure cargo exists
. "$HOME/.cargo/env" || true
echo "Cargo version: $(cargo --version || true)"
echo "Rust version: $(rustc --version || true)"

# Build
cargo build --release
```

## Documentation

- [Architecture](docs/architecture.md)
- [Requirement](docs/requirement.md)