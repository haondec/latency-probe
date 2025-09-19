# Latency Probe - Architecture & Design Documentation

## Overview

The Latency Probe is a high-performance, multi-protocol network latency measurement tool written in Rust. It provides continuous monitoring of network connectivity and performance by executing various types of probes (ICMP, TCP, HTTP, UDP Echo) against configured targets and exposing metrics via Prometheus.

## Application Purpose

This application is designed to:
- Monitor network latency and connectivity across multiple protocols
- Provide real-time network performance metrics
- Support both local configuration files and AWS AppConfig for dynamic configuration
- Export standardized metrics for monitoring and alerting systems
- Scale efficiently with multi-threaded async operations

## Architecture Overview

### High-Level Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Config Mgr    │    │   Scheduler     │    │   Metrics       │
│                 │    │                 │    │   Server        │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │Local File/  │ │    │ │Interval-    │ │    │ │Prometheus   │ │
│ │AWS AppConfig│ │    │ │based Ticker │ │    │ │HTTP Server  │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────┬───────────┘                       │
                     │                                   │
                     ▼                                   │
           ┌─────────────────┐                          │
           │   Probe Engine  │                          │
           │                 │                          │
           │ ┌─────────────┐ │                          │
           │ │ICMP Prober  │ │──────────────────────────┘
           │ └─────────────┘ │
           │ ┌─────────────┐ │
           │ │TCP Prober   │ │
           │ └─────────────┘ │
           │ ┌─────────────┐ │
           │ │HTTP Prober  │ │
           │ └─────────────┘ │
           │ ┌─────────────┐ │
           │ │Echo Prober  │ │
           │ └─────────────┘ │
           └─────────────────┘
```

### Core Modules

#### 1. Configuration Management (`config.rs`)
- **Purpose**: Manages application configuration with support for dynamic updates
- **Features**:
  - Local JSON file configuration
  - AWS AppConfig integration for cloud-native deployments
  - Hot-reload capabilities with background polling
  - Configuration validation and error handling

**Configuration Sources:**
- **Local File Mode**: Reads from `targets.json` or file specified by `TARGET_CONFIG` env var
- **AWS AppConfig Mode**: Integrates with AWS AppConfig for centralized configuration management

#### 2. Scheduler (`scheduler.rs`)
- **Purpose**: Orchestrates probe execution at regular intervals
- **Design**: Simple interval-based scheduler using tokio timers
- **Features**:
  - Configurable probe intervals
  - Non-blocking execution (spawns tasks for each probe cycle)
  - Prevents probe scheduling drift

#### 3. Probe Engine (`prober/`)
Modular probe implementations supporting multiple protocols:

##### ICMP Prober (`icmp.rs`)
- **Protocol**: Internet Control Message Protocol
- **Implementation**: Uses `surge-ping` crate for real ICMP packets
- **Features**:
  - Real network-level ping implementation
  - Process ID-based packet identification
  - Microsecond-precision timing
  - Proper packet verification

##### TCP Connect Prober (`tcp_connect.rs`)
- **Protocol**: TCP connection establishment
- **Measurement**: Time to establish TCP connection
- **Use Case**: Tests TCP reachability and connection setup latency

##### HTTP Prober (`http.rs`)
- **Protocol**: HTTP/HTTPS requests
- **Implementation**: Uses `reqwest` with TLS support
- **Measurement**: Full request-response cycle time
- **Features**: Configurable timeouts, TLS support

##### Echo Prober (`echo.rs`)
- **Protocol**: UDP echo service
- **Implementation**: Simple UDP request/response pattern
- **Use Case**: Custom echo server monitoring

#### 4. Metrics System (`metrics.rs`)
- **Framework**: Prometheus metrics with histogram and counter support
- **Metrics Exposed**:
  - `probe_latency_seconds`: Histogram of probe latencies by target and type
  - `probe_timeout_total`: Counter of probe timeouts by target and type
- **Endpoint**: HTTP server on port 9100 serving `/metrics`

#### 5. Utilities (`util.rs`, `timestamp.rs`)
- **DNS Resolution**: Async hostname-to-IP resolution
- **Host/Port Parsing**: Flexible host:port string parsing
- **Monotonic Timestamps**: High-precision timing using `CLOCK_MONOTONIC_RAW`

## Traffic Flow

### Application Startup Flow
```
1. Initialize tracing/logging
2. Start ConfigManager
   ├─ Load initial configuration (file or AWS AppConfig)
   └─ Spawn background config poller
3. Start Prometheus metrics server (port 9100)
4. Create Scheduler with configured interval
5. Begin probe execution loop
```

### Probe Execution Flow
```
Every probe_interval_ms:
1. Scheduler triggers probe cycle
2. Read current target configuration
3. For each target:
   ├─ Spawn async task for probe execution
   ├─ Select prober based on target.kind
   ├─ Execute probe with configured timeout
   ├─ Record latency metric (success) or timeout counter (failure)
   └─ Log result
4. Sleep until next interval
```

### Configuration Update Flow
```
Background polling (every 30-60 seconds):
1. Check configuration source for changes
2. If changed:
   ├─ Parse new configuration
   ├─ Validate configuration
   ├─ Update in-memory config
   ├─ Update targets list
   └─ Log configuration change
```

## Configuration Format

### JSON Configuration Structure
```json
{
  "probe_interval_ms": 5000,
  "default_timeout_ms": 3000,
  "log_level": "info",
  "targets": [
    {
      "name": "example-icmp",
      "kind": "icmp",
      "host": "example.com"
    },
    {
      "name": "example-tcp",
      "kind": "tcpconnect",
      "host": "example.com",
      "port": 443
    },
    {
      "name": "example-http",
      "kind": "http",
      "host": "https://example.com",
      "port": 443
    },
    {
      "name": "example-echo",
      "kind": "echo",
      "host": "echo.example.com",
      "port": 9000
    }
  ]
}
```

### Environment Variables
- `USE_APP_CONFIG`: Enable AWS AppConfig (default: false)
- `TARGET_CONFIG`: Local config file path (default: targets.json)
- `APP_CONFIG_APPLICATION_ID`: AWS AppConfig application ID
- `APP_CONFIG_ENVIRONMENT_ID`: AWS AppConfig environment ID
- `APP_CONFIG_PROFILE_ID`: AWS AppConfig profile ID
- `APP_CONFIG_POLL_INTERVAL_SECONDS`: AppConfig polling interval (default: 60)
- `CONFIG_POLL_INTERVAL_SECONDS`: Local file polling interval (default: 30)

## Use Cases

### 1. Infrastructure Monitoring
- **Scenario**: Monitor critical infrastructure components
- **Implementation**: Configure ICMP and TCP probes for servers, load balancers, databases
- **Benefits**: Early detection of connectivity issues, network partitions

### 2. SLA Monitoring
- **Scenario**: Track service level agreements for external APIs
- **Implementation**: HTTP probes against API endpoints
- **Metrics**: Latency percentiles, availability percentages
- **Alerting**: Prometheus alerts on SLA violations

### 3. Multi-Region Performance Monitoring
- **Scenario**: Monitor performance across geographic regions
- **Implementation**: Deploy probes in multiple regions, target same endpoints
- **Analysis**: Compare latency patterns, identify regional issues

### 4. CDN Performance Validation
- **Scenario**: Validate CDN performance and failover behavior
- **Implementation**: HTTP probes against CDN endpoints from multiple locations
- **Metrics**: Response time distribution, cache hit rates

### 5. Network Path Analysis
- **Scenario**: Understand network path performance
- **Implementation**: Combine ICMP, TCP, and HTTP probes to same targets
- **Analysis**: Layer 3 vs Layer 4 vs Layer 7 performance comparison

### 6. Microservices Health Monitoring
- **Scenario**: Monitor internal service-to-service communication
- **Implementation**: TCP and HTTP probes for service discovery and health checks
- **Integration**: Kubernetes service discovery, consul integration

## Performance Characteristics

### Scalability
- **Concurrent Probes**: Each probe runs in separate async task
- **Memory Efficiency**: Minimal memory overhead per target
- **CPU Usage**: Efficient async I/O, minimal CPU per probe
- **Network Impact**: Lightweight probes, configurable intervals

### Accuracy
- **Timing Precision**: Microsecond-level timing accuracy
- **Network Layer**: Actual network packets (ICMP, TCP, HTTP, UDP)
- **Minimal Overhead**: Efficient implementation minimizes measurement bias

## Future Enhancements

### 1. Enhanced Probe Types
- **DNS Probe**: Measure DNS resolution latency
- **TLS Handshake Probe**: Separate TLS establishment timing
- **UDP Traceroute**: Multi-hop latency analysis
- **gRPC Probe**: Native gRPC health check support
- **WebSocket Probe**: WebSocket connection establishment and message round-trip

### 2. Advanced Configuration
- **Dynamic Target Discovery**: Kubernetes service discovery, Consul integration
- **Conditional Probing**: Probe targets based on conditions (time, region, etc.)
- **Probe Chaining**: Sequential probe execution with dependencies
- **Load Balancer Aware**: Probe multiple backend instances behind load balancers

### 3. Enhanced Metrics and Observability
- **Custom Percentiles**: Configurable histogram buckets
- **Geolocation Tagging**: Automatic geographic tagging of probe sources
- **Probe Path Metadata**: Track network path information
- **Real-time Dashboards**: Built-in web dashboard for visualization
- **Export Formats**: Support for InfluxDB, CloudWatch, DataDog

### 4. Reliability and Operations
- **Circuit Breaker**: Automatic probe disabling for consistently failing targets
- **Rate Limiting**: Intelligent rate limiting to prevent overwhelming targets
- **Probe Scheduling**: Advanced scheduling (cron-like, backoff strategies)
- **Multi-source Probing**: Coordinate probes from multiple probe instances

### 5. Security and Compliance
- **Probe Authentication**: Support for authenticated HTTP probes
- **TLS Certificate Validation**: Certificate expiry monitoring
- **Network Segmentation**: VLAN-aware probing
- **Audit Logging**: Comprehensive audit trail for compliance

### 6. Cloud-Native Features
- **Kubernetes Operator**: Native Kubernetes CRD support
- **Service Mesh Integration**: Istio/Linkerd integration for service mesh monitoring
- **Cloud Provider Integration**: AWS/GCP/Azure native service monitoring
- **Auto-scaling**: Dynamic probe instance scaling based on target count

### 7. Machine Learning Integration
- **Anomaly Detection**: ML-based detection of unusual latency patterns
- **Predictive Analysis**: Forecast network performance trends
- **Intelligent Alerting**: Reduce false positives with ML-based thresholds
- **Capacity Planning**: ML-driven capacity planning recommendations

### 8. Protocol Extensions
- **IPv6 Support**: Full IPv6 support across all probe types
- **QUIC/HTTP3**: Next-generation HTTP protocol support
- **Custom Protocols**: Plugin architecture for custom protocol probes
- **Binary Protocols**: Support for proprietary binary protocols

## Deployment Considerations

### Resource Requirements
- **Memory**: ~10-50MB base memory usage
- **CPU**: Minimal CPU usage, scales with probe frequency
- **Network**: Lightweight traffic, depends on probe frequency and target count
- **Privileges**: ICMP probes may require elevated privileges

### High Availability
- **Stateless Design**: Probes are stateless, allowing easy horizontal scaling
- **Configuration Redundancy**: AWS AppConfig provides configuration redundancy
- **Metrics Durability**: Prometheus handles metrics persistence and redundancy

### Security
- **Network Access**: Requires outbound network access to probe targets
- **Credentials**: Secure handling of AWS credentials for AppConfig
- **Isolation**: Consider running in isolated network segments for security