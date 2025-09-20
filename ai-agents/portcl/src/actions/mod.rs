pub mod executor;
pub mod portage_actions;
pub mod safety;

pub use executor::{ActionExecutor, ExecutorConfig, ActionResult};
pub use portage_actions::{PortageAction, ActionType, ActionParams};
pub use safety::{SafetyChecker, SafetyCheck, RollbackManager};

use crate::config::ActionConfig;
use crate::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    NoOp,
    AdjustParallelism { jobs: u32 },
    OptimizeBuildOrder { package_list: Vec<String> },
    ScheduleOperation { delay_seconds: u64 },
    PreFetchDependencies { packages: Vec<String> },
    CleanObsoletePackages { force: bool },
}

impl Action {
    pub fn action_type(&self) -> ActionType {
        match self {
            Action::NoOp => ActionType::NoOp,
            Action::AdjustParallelism { .. } => ActionType::AdjustParallelism,
            Action::OptimizeBuildOrder { .. } => ActionType::OptimizeBuildOrder,
            Action::ScheduleOperation { .. } => ActionType::ScheduleOperation,
            Action::PreFetchDependencies { .. } => ActionType::PreFetchDependencies,
            Action::CleanObsoletePackages { .. } => ActionType::CleanObsoletePackages,
        }
    }

    pub fn is_safe(&self) -> bool {
        match self {
            Action::NoOp => true,
            Action::AdjustParallelism { jobs } => *jobs >= 1 && *jobs <= 32,
            Action::OptimizeBuildOrder { .. } => true,
            Action::ScheduleOperation { .. } => true,
            Action::PreFetchDependencies { .. } => true,
            Action::CleanObsoletePackages { force } => !force, // Force clean is less safe
        }
    }

    pub fn description(&self) -> String {
        match self {
            Action::NoOp => "No operation".to_string(),
            Action::AdjustParallelism { jobs } => format!("Adjust compilation parallelism to {} jobs", jobs),
            Action::OptimizeBuildOrder { package_list } => {
                format!("Optimize build order for {} packages", package_list.len())
            },
            Action::ScheduleOperation { delay_seconds } => {
                format!("Schedule operation with {}s delay", delay_seconds)
            },
            Action::PreFetchDependencies { packages } => {
                format!("Pre-fetch dependencies for {} packages", packages.len())
            },
            Action::CleanObsoletePackages { force } => {
                format!("Clean obsolete packages (force: {})", force)
            },
        }
    }
}