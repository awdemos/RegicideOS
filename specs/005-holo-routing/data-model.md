# Holo Routing Test Suite Data Model

## Overview

This document defines the data models and schemas used throughout the Holo routing test suite, including test configuration, results, network topologies, and performance metrics. The data model is designed to be extensible, versioned, and compatible with both JSON and YAML serialization.

## Core Data Types

### Test Identification

```typescript
interface TestIdentifier {
  id: string;                    // Unique test identifier (UUID)
  name: string;                  // Human-readable test name
  version: string;               // Test version (semver)
  category: TestCategory;         // Test category
  tags: string[];                // Searchable tags
}

enum TestCategory {
  UNIT = "unit",
  INTEGRATION = "integration",
  NETWORK = "network",
  PERFORMANCE = "performance",
  SECURITY = "security"
}
```

### Protocol Specification

```typescript
interface ProtocolSpec {
  protocol: RoutingProtocol;      // Protocol type
  version: string;               // Protocol version
  features: ProtocolFeature[];    // Enabled features
  configuration: ProtocolConfig;  // Protocol-specific config
}

enum RoutingProtocol {
  BGP = "bgp",
  OSPF = "ospf",
  ISIS = "isis",
  BFD = "bfd",
  VRRP = "vrrp"
}

interface ProtocolFeature {
  name: string;                  // Feature name
  enabled: boolean;              // Feature status
  parameters: Record<string, any>; // Feature parameters
}
```

## Test Configuration Models

### Test Suite Configuration

```typescript
interface TestSuiteConfig {
  metadata: SuiteMetadata;        // Suite metadata
  global: GlobalConfig;          // Global configuration
  protocols: ProtocolConfig[];    // Protocol configurations
  topologies: TopologyConfig[];   // Network topologies
  execution: ExecutionConfig;      // Execution parameters
  reporting: ReportingConfig;      // Reporting configuration
}

interface SuiteMetadata {
  name: string;                  // Suite name
  version: string;               // Suite version
  description: string;           // Suite description
  author: string;                // Suite author
  created: DateTime;             // Creation timestamp
  updated: DateTime;             // Last update timestamp
}

interface GlobalConfig {
  timeout: Duration;             // Default test timeout
  retry_count: number;           // Default retry count
  parallel_execution: boolean;    // Enable parallel execution
  log_level: LogLevel;           // Default log level
  resource_limits: ResourceLimits; // Resource constraints
}

interface ResourceLimits {
  max_memory_mb: number;         // Maximum memory per test
  max_cpu_percent: number;       // Maximum CPU percentage
  max_duration: Duration;        // Maximum test duration
  max_network_nodes: number;     // Maximum network nodes
}
```

### Protocol-Specific Configurations

#### BGP Configuration

```typescript
interface BGPConfig extends ProtocolConfig {
  as_number: number;             // Autonomous System number
  router_id: IPv4Address;        // BGP router ID
  local_preference: number;       // Local preference
  med: number;                   // Multi-exit discriminator
  neighbors: BGPNeighbor[];       // BGP neighbors
  communities: BGPCommunity[];    // BGP communities
  route_reflector?: BGPRouteReflector; // Route reflector config
}

interface BGPNeighbor {
  address: IPAddress;             // Neighbor IP address
  remote_as: number;             // Remote AS number
  local_as?: number;             // Local AS number (if different)
  authentication?: Authentication; // Authentication config
  timers: BGPTimers;            // BGP timers
  capabilities: BGPCapabilities;  // BGP capabilities
}

interface BGPTimers {
  keepalive: Duration;           // Keepalive interval
  hold_time: Duration;           // Hold time
  connect_retry: Duration;       // Connect retry time
}

interface BGPCapabilities {
  multiprotocol: boolean;        // Multiprotocol extensions
  route_refresh: boolean;         // Route refresh
  graceful_restart: boolean;      // Graceful restart
  add_paths: boolean;            // Add-paths capability
  four_octet_as: boolean;        // 4-octet AS numbers
}
```

#### OSPF Configuration

```typescript
interface OSPFConfig extends ProtocolConfig {
  router_id: IPv4Address;        // OSPF router ID
  reference_bandwidth: number;     // Reference bandwidth (bps)
  areas: OSPFArea[];             // OSPF areas
  authentication?: OSPFAuthentication; // Authentication config
  timers: OSPFTimers;            // OSPF timers
  graceful_restart?: OSPFGracefulRestart; // Graceful restart
}

interface OSPFArea {
  area_id: number;                // Area ID (0 for backbone)
  networks: IPNetwork[];          // Networks in area
  stub?: OSPFStubArea;           // Stub area configuration
  nssa?: OSPFNSSAArea;          // NSSA configuration
  virtual_links?: OSPFVirtualLink[]; // Virtual links
}

interface OSPFTimers {
  hello_interval: Duration;       // Hello interval
  dead_interval: Duration;        // Dead interval
  wait_interval: Duration;       // Wait interval
  retransmit_interval: Duration; // Retransmit interval
}
```

#### IS-IS Configuration

```typescript
interface ISISConfig extends ProtocolConfig {
  system_id: ISISSystemID;       // IS-IS system ID
  area_addresses: ISISAreaAddress[]; // Area addresses
  level: ISISLevel;              // IS-IS level (1, 2, or 1-2)
  interfaces: ISISInterface[];     // IS-IS interfaces
  authentication?: ISISAuthentication; // Authentication config
  timers: ISESTimers;            // IS-IS timers
}

interface ISISSystemID {
  value: string;                 // System ID (6 octets)
}

interface ISISInterface {
  name: string;                  // Interface name
  level: ISISLevel;              // Interface level
  circuit_type: ISISCircuitType; // Circuit type
  metric: number;                // Interface metric
  authentication?: ISISAuthentication; // Interface authentication
}

enum ISISLevel {
  LEVEL_1 = "level-1",
  LEVEL_2 = "level-2",
  LEVEL_1_2 = "level-1-2"
}
```

## Network Topology Models

### Topology Definition

```typescript
interface TopologyConfig {
  metadata: TopologyMetadata;     // Topology metadata
  nodes: TopologyNode[];         // Topology nodes
  links: TopologyLink[];         // Topology links
  protocols: RoutingProtocol[];    // Supported protocols
  resources: TopologyResources;   // Resource requirements
}

interface TopologyMetadata {
  name: string;                  // Topology name
  description: string;            // Topology description
  type: TopologyType;            // Topology type
  complexity: ComplexityLevel;     // Complexity level
  use_cases: string[];           // Typical use cases
}

enum TopologyType {
  FULL_MESH = "full-mesh",
  HUB_SPOKE = "hub-spoke",
  RING = "ring",
  LINEAR = "linear",
  DATA_CENTER = "datacenter",
  MULTI_AREA = "multi-area"
}

enum ComplexityLevel {
  SIMPLE = "simple",
  MODERATE = "moderate",
  COMPLEX = "complex"
}
```

### Node and Link Models

```typescript
interface TopologyNode {
  id: string;                    // Unique node identifier
  name: string;                  // Node name
  type: NodeType;                // Node type
  image: string;                  // Container image
  config: NodeConfig;            // Node configuration
  resources: NodeResources;       // Resource requirements
  protocols: RoutingProtocol[];    // Enabled protocols
}

enum NodeType {
  ROUTER = "router",
  HOST = "host",
  SWITCH = "switch",
  FIREWALL = "firewall"
}

interface NodeConfig {
  hostname: string;              // Node hostname
  interfaces: NodeInterface[];    // Network interfaces
  routing: ProtocolConfig[];      // Routing configuration
  system: SystemConfig;           // System configuration
}

interface NodeInterface {
  name: string;                  // Interface name
  type: InterfaceType;           // Interface type
  address?: IPAddress;            // IP address (if configured)
  mtu?: number;                 // MTU size
  bandwidth?: number;             // Bandwidth (Mbps)
  latency?: number;              // Latency (ms)
}

interface TopologyLink {
  id: string;                    // Unique link identifier
  endpoints: LinkEndpoint[];      // Link endpoints
  bandwidth: number;             // Link bandwidth (Mbps)
  latency: number;                // Link latency (ms)
  loss?: number;                 // Packet loss percentage
  jitter?: number;               // Jitter (ms)
}

interface LinkEndpoint {
  node_id: string;               // Node identifier
  interface: string;             // Interface name
}
```

## Test Execution Models

### Test Execution

```typescript
interface TestExecution {
  id: string;                    // Execution identifier
  config: TestExecutionConfig;    // Execution configuration
  status: ExecutionStatus;        // Execution status
  started_at: DateTime;          // Start timestamp
  completed_at?: DateTime;        // Completion timestamp
  duration?: Duration;           // Execution duration
  results: TestResult[];         // Test results
  artifacts: ExecutionArtifacts;   // Execution artifacts
}

interface TestExecutionConfig {
  test_ids: string[];            // Test IDs to execute
  topology_id?: string;          // Network topology to use
  parallel: boolean;             // Parallel execution
  timeout: Duration;             // Execution timeout
  retry_count: number;           // Retry count
  log_level: LogLevel;           // Log level
  resource_limits: ResourceLimits; // Resource limits
}

enum ExecutionStatus {
  PENDING = "pending",
  RUNNING = "running",
  COMPLETED = "completed",
  FAILED = "failed",
  CANCELLED = "cancelled",
  TIMEOUT = "timeout"
}
```

### Test Results

```typescript
interface TestResult {
  test_id: string;               // Test identifier
  execution_id: string;          // Execution identifier
  status: TestStatus;            // Test status
  started_at: DateTime;          // Start timestamp
  completed_at?: DateTime;        // Completion timestamp
  duration?: Duration;           // Test duration
  output: TestOutput;            // Test output
  metrics: TestMetrics;           // Test metrics
  artifacts: TestArtifacts;       // Test artifacts
}

enum TestStatus {
  PASSED = "passed",
  FAILED = "failed",
  SKIPPED = "skipped",
  TIMEOUT = "timeout",
  ERROR = "error"
}

interface TestOutput {
  stdout: string;                // Standard output
  stderr: string;                // Standard error
  logs: LogEntry[];              // Structured logs
  events: TestEvent[];            // Test events
}

interface LogEntry {
  timestamp: DateTime;           // Log timestamp
  level: LogLevel;               // Log level
  message: string;               // Log message
  context?: Record<string, any>;  // Log context
}

interface TestEvent {
  timestamp: DateTime;           // Event timestamp
  type: EventType;               // Event type
  source: string;                // Event source
  data: Record<string, any>;      // Event data
}
```

## Performance Models

### Performance Metrics

```typescript
interface PerformanceMetrics {
  protocol: RoutingProtocol;       // Protocol type
  metric_type: MetricType;        // Metric type
  value: number;                  // Metric value
  unit: string;                   // Unit of measurement
  target?: number;                // Target value
  threshold?: number;             // Threshold value
  timestamp: DateTime;            // Measurement timestamp
  context: MetricContext;         // Measurement context
}

enum MetricType {
  CONVERGENCE_TIME = "convergence_time",
  ROUTE_PROCESSING_RATE = "route_processing_rate",
  MEMORY_USAGE = "memory_usage",
  CPU_USAGE = "cpu_usage",
  PACKET_LOSS = "packet_loss",
  LATENCY = "latency",
  THROUGHPUT = "throughput"
}

interface MetricContext {
  test_id: string;               // Test identifier
  topology_id: string;           // Topology identifier
  node_count: number;            // Number of nodes
  route_count: number;           // Number of routes
  load_level: LoadLevel;          // System load level
  environment: Environment;       // Test environment
}

enum LoadLevel {
  IDLE = "idle",
  LIGHT = "light",
  MODERATE = "moderate",
  HEAVY = "heavy",
  PEAK = "peak"
}
```

### Benchmark Results

```typescript
interface BenchmarkResult {
  id: string;                    // Benchmark identifier
  name: string;                  // Benchmark name
  protocol: RoutingProtocol;       // Protocol type
  metric_type: MetricType;        // Metric type
  samples: MetricSample[];        // Metric samples
  statistics: BenchmarkStatistics; // Statistical analysis
  metadata: BenchmarkMetadata;    // Benchmark metadata
}

interface MetricSample {
  value: number;                  // Sample value
  timestamp: DateTime;           // Sample timestamp
  context: MetricContext;         // Sample context
}

interface BenchmarkStatistics {
  sample_count: number;           // Number of samples
  mean: number;                  // Mean value
  median: number;                // Median value
  min: number;                   // Minimum value
  max: number;                   // Maximum value
  std_deviation: number;          // Standard deviation
  percentile_95: number;          // 95th percentile
  percentile_99: number;          // 99th percentile
}
```

## Security Models

### Security Scan Results

```typescript
interface SecurityScanResult {
  id: string;                    // Scan identifier
  scan_type: ScanType;           // Scan type
  target: string;                 // Scan target
  status: ScanStatus;             // Scan status
  started_at: DateTime;          // Start timestamp
  completed_at?: DateTime;        // Completion timestamp
  vulnerabilities: Vulnerability[]; // Found vulnerabilities
  summary: SecuritySummary;       // Security summary
  recommendations: Recommendation[]; // Recommendations
}

enum ScanType {
  CONTAINER = "container",
  PROTOCOL = "protocol",
  CONFIGURATION = "configuration",
  NETWORK = "network",
  COMPREHENSIVE = "comprehensive"
}

interface Vulnerability {
  id: string;                    // Vulnerability identifier
  severity: SeverityLevel;        // Severity level
  cve_id?: string;               // CVE identifier
  title: string;                 // Vulnerability title
  description: string;            // Vulnerability description
  affected_component: string;      // Affected component
  fixed_version?: string;          // Fixed version
  references: string[];           // Reference URLs
  score?: number;                 // CVSS score
}

enum SeverityLevel {
  LOW = "low",
  MEDIUM = "medium",
  HIGH = "high",
  CRITICAL = "critical"
}
```

### Authentication Models

```typescript
interface Authentication {
  type: AuthenticationType;       // Authentication type
  algorithm?: AuthenticationAlgorithm; // Authentication algorithm
  key_id?: string;               // Key identifier
  key?: string;                  // Authentication key
  lifetime?: Duration;            // Key lifetime
}

enum AuthenticationType {
  NONE = "none",
  MD5 = "md5",
  HMAC_SHA = "hmac_sha",
  SHA256 = "sha256",
  SHA384 = "sha384",
  SHA512 = "sha512"
}
```

## Coverage Models

### Code Coverage

```typescript
interface CoverageReport {
  id: string;                    // Report identifier
  timestamp: DateTime;           // Report timestamp
  format: CoverageFormat;         // Report format
  overall: CoverageMetrics;       // Overall coverage
  protocols: ProtocolCoverage[];   // Protocol coverage
  modules: ModuleCoverage[];       // Module coverage
  functions: FunctionCoverage[];   // Function coverage
  lines: LineCoverage[];           // Line coverage
}

interface CoverageMetrics {
  lines_covered: number;          // Lines covered
  lines_total: number;            // Total lines
  lines_percentage: number;       // Lines percentage
  functions_covered: number;      // Functions covered
  functions_total: number;        // Total functions
  functions_percentage: number;   // Functions percentage
  branches_covered: number;       // Branches covered
  branches_total: number;         // Total branches
  branches_percentage: number;    // Branches percentage
}

interface ProtocolCoverage {
  protocol: RoutingProtocol;       // Protocol type
  metrics: CoverageMetrics;       // Coverage metrics
  files: FileCoverage[];          // File coverage
}
```

## Configuration Validation Models

### Validation Results

```typescript
interface ValidationResult {
  valid: boolean;                // Validation result
  errors: ValidationError[];      // Validation errors
  warnings: ValidationWarning[];  // Validation warnings
  suggestions: ValidationSuggestion[]; // Improvement suggestions
}

interface ValidationError {
  path: string;                  // Configuration path
  message: string;               // Error message
  code: string;                  // Error code
  severity: SeverityLevel;        // Error severity
}

interface ValidationWarning {
  path: string;                  // Configuration path
  message: string;               // Warning message
  code: string;                  // Warning code
  recommendation?: string;        // Recommendation
}
```

## Serialization Formats

### JSON Schema Examples

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Holo Routing Test Suite Configuration",
  "type": "object",
  "properties": {
    "metadata": {
      "$ref": "#/definitions/SuiteMetadata"
    },
    "global": {
      "$ref": "#/definitions/GlobalConfig"
    },
    "protocols": {
      "type": "array",
      "items": {
        "$ref": "#/definitions/ProtocolConfig"
      }
    },
    "topologies": {
      "type": "array",
      "items": {
        "$ref": "#/definitions/TopologyConfig"
      }
    }
  },
  "required": ["metadata", "global", "protocols", "topologies"]
}
```

### YAML Configuration Examples

```yaml
# Test suite configuration
metadata:
  name: "Holo Routing Test Suite"
  version: "1.0.0"
  description: "Comprehensive test suite for Holo routing protocols"

global:
  timeout: "10m"
  retry_count: 3
  parallel_execution: true
  log_level: "info"
  resource_limits:
    max_memory_mb: 2048
    max_cpu_percent: 80
    max_duration: "1h"
    max_network_nodes: 10

protocols:
  - protocol: "bgp"
    version: "4"
    configuration:
      as_number: 65001
      router_id: "192.168.1.1"
      neighbors:
        - address: "192.168.1.2"
          remote_as: 65002
          timers:
            keepalive: "60s"
            hold_time: "180s"

topologies:
  - name: "bgp-test"
    type: "full-mesh"
    complexity: "simple"
    nodes:
      - id: "router1"
        type: "router"
        image: "ghcr.io/holo-routing/holo:latest"
        protocols: ["bgp"]
    links:
      - endpoints:
          - node_id: "router1"
            interface: "eth1"
          - node_id: "router2"
            interface: "eth1"
        bandwidth: 1000
        latency: 1
```

## Versioning and Compatibility

### Data Model Versioning

```typescript
interface DataModelVersion {
  major: number;                 // Major version
  minor: number;                 // Minor version
  patch: number;                 // Patch version
  compatibility: CompatibilityLevel; // Compatibility level
}

enum CompatibilityLevel {
  BACKWARD_COMPATIBLE = "backward_compatible",
  FORWARD_COMPATIBLE = "forward_compatible",
  BREAKING = "breaking"
}
```

### Migration Support

```typescript
interface DataMigration {
  from_version: string;           // Source version
  to_version: string;             // Target version
  migration_function: string;     // Migration function name
  automatic: boolean;            // Automatic migration
  manual_steps?: string[];        // Manual migration steps
}
```

---

**Usage Notes**:
1. All timestamps should be in ISO 8601 format with timezone
2. Duration values should follow ISO 8601 duration format
3. IP addresses should be validated according to protocol (IPv4/IPv6)
4. All identifiers should be UUID v4 format unless specified otherwise
5. Enum values should be used exactly as defined (case-sensitive)

**Implementation Notes**:
1. Data models are designed for both JSON and YAML serialization
2. All optional fields should be explicitly marked as such
3. Validation should be performed at both input and output boundaries
4. Version compatibility should be maintained for at least one major version
5. Error handling should include detailed context and suggested resolutions