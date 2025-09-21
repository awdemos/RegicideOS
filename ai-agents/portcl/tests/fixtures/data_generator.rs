//! Test data generator for PortCL testing
//!
//! This module provides utilities for generating realistic test data
//! for various PortCL components and scenarios.

use crate::fixtures::mock_data::*;
use crate::fixtures::test_models::*;
use rand::Rng;
use std::collections::HashMap;
use uuid::Uuid;

/// Test data generator for creating varied and realistic test scenarios
pub struct TestDataGenerator {
    rng: rand::rngs::ThreadRng,
}

impl TestDataGenerator {
    /// Create a new test data generator with a random seed
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }

    /// Create a new test data generator with a specific seed
    pub fn with_seed(seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Generate a random package
    pub fn generate_package(&mut self) -> MockPackage {
        let categories = [
            "sys-apps", "dev-lang", "net-misc", "www-client", "app-editors",
            "dev-util", "sys-kernel", "media-libs", "x11-libs", "gnome-base"
        ];

        let package_names = [
            "portage", "rust", "firefox", "vim", "git", "curl", "systemd",
            "openssl", "python", "nodejs", "gcc", "clang", "make", "cmake"
        ];

        let licenses = [
            "GPL-2", "GPL-3", "MIT", "Apache-2.0", "BSD", "MPL-2.0", "LGPL-2.1"
        ];

        let use_flags = [
            "X", "gtk", "qt5", "alsa", "pulseaudio", "systemd", "ssl", "threads",
            "static-libs", "debug", "doc", "examples", "test"
        ];

        MockPackage {
            name: package_names[self.rng.gen_range(0..package_names.len())].to_string(),
            category: categories[self.rng.gen_range(0..categories.len())].to_string(),
            version: self.generate_version(),
            description: self.generate_description(),
            homepage: Some(format!("https://example.com/{}", self.generate_random_string(8))),
            license: licenses[self.rng.gen_range(0..licenses.len())].to_string(),
            use_flags: self.generate_use_flags(&use_flags),
            dependencies: self.generate_dependencies(),
        }
    }

    /// Generate multiple random packages
    pub fn generate_packages(&mut self, count: usize) -> Vec<MockPackage> {
        (0..count).map(|_| self.generate_package()).collect()
    }

    /// Generate a random Portage configuration
    pub fn generate_portage_config(&mut self) -> MockPortageConfig {
        MockPortageConfig {
            monitoring: MockMonitoringConfig {
                poll_interval: self.rng.gen_range(10..300),
                portage_path: "/usr/bin/portage".to_string(),
                log_path: format!("/var/log/portcl-{}.log", self.generate_random_string(8)),
                metrics_retention_days: self.rng.gen_range(7..365),
                enable_event_tracking: self.rng.gen_bool(0.8),
            },
            rl: MockRLConfig {
                learning_rate: self.rng.gen_range(0.0001..0.1),
                discount_factor: self.rng.gen_range(0.8..0.99),
                exploration_rate: self.rng.gen_range(0.01..0.3),
                exploration_decay: self.rng.gen_range(0.99..0.9999),
                memory_size: self.rng.gen_range(1000..100000),
                batch_size: self.rng.gen_range(16..256),
                target_update_freq: self.rng.gen_range(50..500),
                model_path: format!("/var/lib/portcl/model-{}.pt", self.generate_random_string(8)),
                enable_continual_learning: self.rng.gen_bool(0.7),
                ewc_importance: self.rng.gen_range(100.0..10000.0),
            },
            actions: MockActionConfig {
                enable_dry_run: self.rng.gen_bool(0.6),
                max_concurrent_actions: self.rng.gen_range(1..10),
                action_timeout: self.rng.gen_range(60..1800),
                rollback_enabled: self.rng.gen_bool(0.8),
                safe_actions_only: self.rng.gen_bool(0.7),
            },
            safety: MockSafetyConfig {
                max_cpu_usage: self.rng.gen_range(50.0..100.0),
                max_memory_usage: self.rng.gen_range(60.0..95.0),
                min_disk_space_gb: self.rng.gen_range(1.0..50.0),
                critical_packages: self.generate_critical_packages(),
                enable_system_checks: self.rng.gen_bool(0.9),
                backup_before_actions: self.rng.gen_bool(0.8),
            },
            general: MockGeneralConfig {
                log_level: self.generate_log_level(),
                data_directory: format!("/var/lib/portcl-{}", self.generate_random_string(8)),
                user: if self.rng.gen_bool(0.8) { "portcl".to_string() } else { "root".to_string() },
                group: if self.rng.gen_bool(0.8) { "portcl".to_string() } else { "root".to_string() },
                enable_metrics_collection: self.rng.gen_bool(0.9),
            },
        }
    }

    /// Generate a random action
    pub fn generate_action(&mut self) -> MockAction {
        let actions = [
            MockAction::NoOp,
            MockAction::AdjustParallelism { jobs: self.rng.gen_range(1..32) },
            MockAction::OptimizeBuildOrder {
                package_list: self.generate_package_list(self.rng.gen_range(1..10))
            },
            MockAction::ScheduleOperation { delay_seconds: self.rng.gen_range(1..3600) },
            MockAction::PreFetchDependencies {
                packages: self.generate_package_list(self.rng.gen_range(1..20))
            },
            MockAction::CleanObsoletePackages { force: self.rng.gen_bool(0.3) },
        ];

        actions[self.rng.gen_range(0..actions.len())].clone()
    }

    /// Generate multiple random actions
    pub fn generate_actions(&mut self, count: usize) -> Vec<MockAction> {
        (0..count).map(|_| self.generate_action()).collect()
    }

    /// Generate a test result with random data
    pub fn generate_test_result(&mut self) -> TestResult {
        let mut result = TestResult::new(
            self.generate_test_name(),
            self.generate_test_type()
        );

        result.status = self.generate_test_status();
        result.duration_ms = self.rng.gen_range(1..30000);
        result.metrics = TestMetrics {
            memory_peak_bytes: self.rng.gen_range(1024..1024 * 1024 * 1024),
            cpu_time_ms: self.rng.gen_range(1..5000),
            allocations: self.rng.gen_range(10..100000),
            disk_io_bytes: self.rng.gen_range(0..1024 * 1024 * 100),
            network_io_bytes: self.rng.gen_range(0..1024 * 1024 * 10),
            custom_metrics: self.generate_custom_metrics(),
        };

        result
    }

    /// Generate a test configuration with random settings
    pub fn generate_test_config(&mut self) -> TestConfig {
        TestConfig {
            test_suite_name: format!("{} Test Suite", self.generate_random_string(10)),
            test_types: self.generate_test_types(),
            execution_config: TestExecutionConfig {
                max_concurrent_tests: self.rng.gen_range(1..16),
                test_timeout_seconds: self.rng.gen_range(30..1800),
                retry_failed_tests: self.rng.gen_bool(0.6),
                max_retries: self.rng.gen_range(1..5),
                fail_fast: self.rng.gen_bool(0.3),
                shuffle_tests: self.rng.gen_bool(0.4),
                test_order: self.generate_test_order(),
                include_ignored: self.rng.gen_bool(0.2),
            },
            reporting_config: TestReportingConfig {
                output_format: self.generate_output_format(),
                report_path: format!("./test-reports-{}", self.generate_random_string(8)),
                include_stdout: self.rng.gen_bool(0.8),
                include_stderr: self.rng.gen_bool(0.8),
                include_logs: self.rng.gen_bool(0.7),
                include_metrics: self.rng.gen_bool(0.6),
                generate_html_report: self.rng.gen_bool(0.9),
                generate_json_report: self.rng.gen_bool(0.9),
                generate_junit_xml: self.rng.gen_bool(0.7),
            },
            coverage_config: TestCoverageConfig {
                enabled: self.rng.gen_bool(0.8),
                coverage_type: self.generate_coverage_type(),
                target_percentage: self.rng.gen_range(50.0..95.0),
                fail_below_target: self.rng.gen_bool(0.7),
                output_path: format!("./coverage-{}", self.generate_random_string(8)),
                include_patterns: vec!["src/**/*.rs".to_string()],
                exclude_patterns: vec!["tests/fixtures/**/*".to_string()],
            },
            performance_config: TestPerformanceConfig {
                enabled: self.rng.gen_bool(0.6),
                benchmark_iterations: self.rng.gen_range(10..1000),
                warmup_iterations: self.rng.gen_range(1..50),
                sample_size: self.rng.gen_range(10..200),
                confidence_level: self.rng.gen_range(0.8..0.99),
                max_execution_time_seconds: self.rng.gen_range(60..3600),
                memory_limit_mb: self.rng.gen_range(512..8192),
                cpu_limit_percent: self.rng.gen_range(50.0..100.0),
            },
            filters: TestFilters {
                include_patterns: vec!["**/*test*.rs".to_string()],
                exclude_patterns: vec![],
                include_tags: vec![],
                exclude_tags: vec!["slow".to_string()],
                include_test_types: vec![TestType::Unit, TestType::Integration],
                exclude_test_types: vec![],
                min_severity: LogLevel::Info,
            },
            environment: TestEnvironment {
                variables: self.generate_env_vars(),
                temp_dir: format!("/tmp/portcl-tests-{}", self.generate_random_string(8)),
                work_dir: std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .to_string_lossy()
                    .to_string(),
                log_dir: format!("./test-logs-{}", self.generate_random_string(8)),
                artifact_dir: format!("./test-artifacts-{}", self.generate_random_string(8)),
                cleanup_after: self.rng.gen_bool(0.8),
                preserve_temp_dirs: self.rng.gen_bool(0.2),
            },
        }
    }

    // Helper methods

    fn generate_version(&mut self) -> String {
        let major = self.rng.gen_range(0..10);
        let minor = self.rng.gen_range(0..100);
        let patch = self.rng.gen_range(0..100);
        let suffix = if self.rng.gen_bool(0.3) {
            let suffixes = ["_alpha", "_beta", "_rc", "_p"];
            let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
            let num = self.rng.gen_range(1..10);
            format!("{}{}", suffix, num)
        } else {
            String::new()
        };
        format!("{}.{}.{}{}", major, minor, patch, suffix)
    }

    fn generate_description(&mut self) -> String {
        let adjectives = [
            "Advanced", "Modern", "Fast", "Secure", "Reliable", "Efficient",
            "Lightweight", "Powerful", "Flexible", "Robust"
        ];
        let nouns = [
            "library", "tool", "framework", "system", "application", "service",
            "daemon", "utility", "component", "module"
        ];
        let purposes = [
            "for development", "for testing", "for production", "for security",
            "for performance", "for reliability", "for flexibility"
        ];

        let adjective = adjectives[self.rng.gen_range(0..adjectives.len())];
        let noun = nouns[self.rng.gen_range(0..nouns.len())];
        let purpose = purposes[self.rng.gen_range(0..purposes.len())];

        format!("{} {} {}", adjective, noun, purpose)
    }

    fn generate_use_flags(&mut self, available_flags: &[&str]) -> Vec<String> {
        let count = self.rng.gen_range(0..std::cmp::min(6, available_flags.len()));
        let mut flags = Vec::new();

        for _ in 0..count {
            let flag = available_flags[self.rng.gen_range(0..available_flags.len())];
            if !flags.contains(&flag.to_string()) {
                flags.push(flag.to_string());
            }
        }

        flags
    }

    fn generate_dependencies(&mut self) -> Vec<String> {
        let count = self.rng.gen_range(0..8);
        let base_deps = ["virtual/libc", "sys-libs/zlib", "dev-libs/openssl"];
        let mut deps = Vec::new();

        for _ in 0..count {
            if self.rng.gen_bool(0.7) && !base_deps.is_empty() {
                deps.push(base_deps[self.rng.gen_range(0..base_deps.len())].to_string());
            } else {
                deps.push(format!("{}/{}",
                    self.generate_random_string(6).to_lowercase(),
                    self.generate_random_string(8).to_lowercase()
                ));
            }
        }

        deps
    }

    fn generate_critical_packages(&mut self) -> Vec<String> {
        let base_packages = [
            "sys-kernel/gentoo-kernel",
            "sys-apps/systemd",
            "sys-apps/portage",
            "sys-libs/glibc",
            "net-misc/curl"
        ];

        let count = self.rng.gen_range(3..base_packages.len());
        let mut packages = Vec::new();

        for _ in 0..count {
            let pkg = base_packages[self.rng.gen_range(0..base_packages.len())];
            if !packages.contains(&pkg.to_string()) {
                packages.push(pkg.to_string());
            }
        }

        packages
    }

    fn generate_log_level(&mut self) -> String {
        let levels = ["error", "warn", "info", "debug", "trace"];
        levels[self.rng.gen_range(0..levels.len())].to_string()
    }

    fn generate_package_list(&mut self, count: usize) -> Vec<String> {
        let categories = ["sys-apps", "dev-lang", "net-misc", "app-editors"];
        let packages = ["portage", "rust", "curl", "vim", "git"];

        (0..count).map(|_| {
            let category = categories[self.rng.gen_range(0..categories.len())];
            let package = packages[self.rng.gen_range(0..packages.len())];
            format!("{}/{}", category, package)
        }).collect()
    }

    fn generate_test_name(&mut self) -> String {
        let prefixes = ["test_", "should_", "verify_", "check_", "validate_"];
        let verbs = ["handle", "process", "execute", "manage", "perform"];
        let nouns = ["input", "output", "request", "response", "error"];
        let suffixes = ["_correctly", "_successfully", "_properly", "_efficiently"];

        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        let verb = verbs[self.rng.gen_range(0..verbs.len())];
        let noun = nouns[self.rng.gen_range(0..nouns.len())];
        let suffix = if self.rng.gen_bool(0.6) {
            suffixes[self.rng.gen_range(0..suffixes.len())]
        } else {
            ""
        };

        format!("{}{}{}_{}", prefix, verb, noun, suffix)
    }

    fn generate_test_type(&mut self) -> TestType {
        let types = [
            TestType::Unit,
            TestType::Integration,
            TestType::Performance,
            TestType::Contract,
            TestType::Property,
            TestType::Benchmark,
        ];
        types[self.rng.gen_range(0..types.len())].clone()
    }

    fn generate_test_status(&mut self) -> TestStatus {
        let statuses = [
            TestStatus::Passed,
            TestStatus::Failed,
            TestStatus::Skipped,
            TestStatus::Timeout,
            TestStatus::Error,
        ];
        statuses[self.rng.gen_range(0..statuses.len())].clone()
    }

    fn generate_custom_metrics(&mut self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        let count = self.rng.gen_range(0..5);

        for _ in 0..count {
            let name = format!("metric_{}", self.generate_random_string(6));
            let value = self.rng.gen_range(0.0..1000.0);
            metrics.insert(name, value);
        }

        metrics
    }

    fn generate_test_types(&mut self) -> Vec<TestType> {
        let all_types = vec![
            TestType::Unit,
            TestType::Integration,
            TestType::Performance,
            TestType::Contract,
            TestType::Property,
            TestType::Benchmark,
        ];

        let count = self.rng.gen_range(1..all_types.len());
        let mut selected = Vec::new();

        for _ in 0..count {
            let test_type = &all_types[self.rng.gen_range(0..all_types.len())];
            if !selected.contains(test_type) {
                selected.push(test_type.clone());
            }
        }

        selected
    }

    fn generate_test_order(&mut self) -> TestOrder {
        let orders = [
            TestOrder::Sequential,
            TestOrder::Random,
            TestOrder::Parallel,
            TestOrder::DependencyOrder,
        ];
        orders[self.rng.gen_range(0..orders.len())].clone()
    }

    fn generate_output_format(&mut self) -> OutputFormat {
        let formats = [
            OutputFormat::Pretty,
            OutputFormat::Terse,
            OutputFormat::Json,
            OutputFormat::JUnit,
            OutputFormat::Html,
        ];
        formats[self.rng.gen_range(0..formats.len())].clone()
    }

    fn generate_coverage_type(&mut self) -> CoverageType {
        let types = [
            CoverageType::Line,
            CoverageType::Branch,
            CoverageType::Function,
            CoverageType::Condition,
            CoverageType::Full,
        ];
        types[self.rng.gen_range(0..types.len())].clone()
    }

    fn generate_env_vars(&mut self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        let count = self.rng.gen_range(2..8);

        let common_vars = [
            ("RUST_LOG", "info"),
            ("RUST_BACKTRACE", "1"),
            ("PORTCL_DEBUG", "false"),
            ("PORTCL_CONFIG_PATH", "/etc/portcl/config.toml"),
        ];

        for _ in 0..count {
            if self.rng.gen_bool(0.6) && !common_vars.is_empty() {
                let (key, value) = common_vars[self.rng.gen_range(0..common_vars.len())];
                vars.insert(key.to_string(), value.to_string());
            } else {
                let key = format!("VAR_{}", self.generate_random_string(4).to_uppercase());
                let value = self.generate_random_string(8);
                vars.insert(key, value);
            }
        }

        vars
    }

    fn generate_random_string(&mut self, length: usize) -> String {
        use rand::distributions::Alphanumeric;
        self.rng
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creation() {
        let gen = TestDataGenerator::new();
        let gen_seeded = TestDataGenerator::with_seed(42);
        assert!(true); // Just verify creation works
    }

    #[test]
    fn test_package_generation() {
        let mut gen = TestDataGenerator::with_seed(42);
        let package = gen.generate_package();

        assert!(!package.name.is_empty());
        assert!(!package.category.is_empty());
        assert!(!package.version.is_empty());
        assert!(!package.license.is_empty());
    }

    #[test]
    fn test_package_generation_consistency() {
        let mut gen1 = TestDataGenerator::with_seed(42);
        let mut gen2 = TestDataGenerator::with_seed(42);

        let pkg1 = gen1.generate_package();
        let pkg2 = gen2.generate_package();

        assert_eq!(pkg1.name, pkg2.name);
        assert_eq!(pkg1.category, pkg2.category);
        assert_eq!(pkg1.version, pkg2.version);
    }

    #[test]
    fn test_action_generation() {
        let mut gen = TestDataGenerator::new();
        let action = gen.generate_action();

        // Just verify it generates some action
        match action {
            MockAction::NoOp => {},
            MockAction::AdjustParallelism { jobs } => assert!(jobs > 0),
            MockAction::OptimizeBuildOrder { package_list } => assert!(!package_list.is_empty()),
            MockAction::ScheduleOperation { delay_seconds } => assert!(delay_seconds > 0),
            MockAction::PreFetchDependencies { packages } => assert!(!packages.is_empty()),
            MockAction::CleanObsoletePackages { .. } => {},
        }
    }

    #[test]
    fn test_test_result_generation() {
        let mut gen = TestDataGenerator::new();
        let result = gen.generate_test_result();

        assert!(!result.test_name.is_empty());
        assert!(result.duration_ms > 0);
        assert!(result.metrics.memory_peak_bytes > 0);
    }

    #[test]
    fn test_bulk_generation() {
        let mut gen = TestDataGenerator::new();
        let packages = gen.generate_packages(10);
        let actions = gen.generate_actions(5);

        assert_eq!(packages.len(), 10);
        assert_eq!(actions.len(), 5);
    }
}