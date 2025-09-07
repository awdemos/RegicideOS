use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn, error, debug};

mod btrfs;
mod learning;
mod actions;
mod config;
mod fragmentation_model;

use btrfs::BtrfsMonitor;
use learning::{ReinforcementLearner, State};
use actions::{ActionExecutor, Action};
use config::Config;

#[derive(Parser)]
#[command(name = "btrmind")]
#[command(about = "AI-powered BTRFS storage monitoring and optimization")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(short, long, default_value = "/etc/btrmind/config.toml")]
    config: PathBuf,
    
    #[arg(short, long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the monitoring daemon
    Run,
    /// Analyze current storage state
    Analyze,
    /// Run cleanup actions manually
    Cleanup {
        #[arg(long)]
        aggressive: bool,
    },
    /// Display current statistics
    Stats,
    /// Validate configuration
    Config,
    /// Manage fragmentation model
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// Train a new fragmentation model
    Train {
        #[arg(long)]
        data: Option<String>,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        validate: bool,
    },
    /// Rollback to a previous model version
    Rollback {
        #[arg(long)]
        backup: String,
    },
    /// List available model backups
    ListBackups,
    /// Show model information
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub disk_usage_percent: f64,
    pub free_space_mb: f64,
    pub metadata_usage_percent: f64,
    pub fragmentation_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub disk_usage_percent: f64,
    pub free_space_mb: f64,
    pub metadata_usage_percent: f64,
    pub fragmentation_percent: f64,
    pub file_count: u64,
    pub avg_file_size_mb: f64,
    pub write_frequency: f64,
    pub fragmentation_proxy: f64,
}

pub struct BtrMindAgent {
    monitor: BtrfsMonitor,
    pub learner: ReinforcementLearner,
    executor: ActionExecutor,
    config: Config,
    last_metrics: Option<SystemMetrics>,
}

impl BtrMindAgent {
    pub fn new(config: Config) -> Result<Self> {
        let monitor = BtrfsMonitor::with_data_collection(
            &config.monitoring.target_path,
            if config.fragmentation_model.enable_data_collection {
                Some(config.fragmentation_model.training_data_path.clone())
            } else {
                None
            },
            config.fragmentation_model.enable_data_collection,
            if config.fragmentation_model.use_model {
                Some(config.fragmentation_model.model_path.clone())
            } else {
                None
            },
            config.fragmentation_model.use_model,
            config.fragmentation_model.fallback_to_heuristic
        )?;
        let learner = ReinforcementLearner::new(&config.learning)?;
        let executor = ActionExecutor::new(config.actions.clone(), config.dry_run);
        
        Ok(Self {
            monitor,
            learner,
            executor,
            config,
            last_metrics: None,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting BtrMind agent");
        info!("Target path: {}", self.config.monitoring.target_path);
        info!("Poll interval: {}s", self.config.monitoring.poll_interval);
        
        let mut interval = time::interval(Duration::from_secs(self.config.monitoring.poll_interval));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.monitoring_cycle().await {
                error!("Monitoring cycle failed: {}", e);
                // Continue running despite errors
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
    
    async fn monitoring_cycle(&mut self) -> Result<()> {
        // 1. Observe current state
        let metrics = self.monitor.collect_metrics().await?;
        debug!("Collected metrics: {:?}", metrics);
        
        // 2. Convert to ML state representation
        let state = State::from_metrics(&metrics);
        
        // 3. Get action from RL agent
        let action = self.learner.select_action(&state)?;
        debug!("Selected action: {:?}", action);
        
        // 4. Execute action
        let action_result = self.executor.execute_action(action).await;
        
        // 5. Calculate reward
        let reward = if let Some(ref prev_metrics) = self.last_metrics {
            self.calculate_reward(prev_metrics, &metrics)
        } else {
            0.0 // No reward for first observation
        };
        
        // 6. Update learning model
        if let Some(ref prev_metrics) = self.last_metrics {
            let prev_state = State::from_metrics(prev_metrics);
            self.learner.update(&prev_state, action, reward, &state)?;
        }
        
        // 7. Log and alert if needed
        self.check_thresholds(&metrics).await?;
        
        // 8. Store metrics for next cycle
        self.last_metrics = Some(metrics);
        
        if action_result.is_err() {
            warn!("Action execution failed: {:?}", action_result);
        }
        
        Ok(())
    }
    
    fn calculate_reward(&self, prev_metrics: &SystemMetrics, curr_metrics: &SystemMetrics) -> f64 {
        let util_delta = prev_metrics.disk_usage_percent - curr_metrics.disk_usage_percent;
        
        // Base reward: positive if space freed
        let mut reward = util_delta * 10.0;
        
        // Penalties for critical thresholds
        if curr_metrics.disk_usage_percent > self.config.thresholds.critical_level {
            reward -= 50.0; // Severe penalty
        } else if curr_metrics.disk_usage_percent > self.config.thresholds.warning_level {
            reward -= 15.0; // Moderate penalty
        }
        
        // Bonus for sustained improvement
        if util_delta > 2.0 {
            reward += 5.0;
        }
        
        debug!("Reward calculation: util_delta={:.2}, reward={:.2}", util_delta, reward);
        reward
    }
    
    async fn check_thresholds(&self, metrics: &SystemMetrics) -> Result<()> {
        if metrics.disk_usage_percent >= self.config.thresholds.emergency_level {
            error!("EMERGENCY: Disk usage at {:.1}%! Immediate action required!", 
                  metrics.disk_usage_percent);
            // TODO: Send system notification
        } else if metrics.disk_usage_percent >= self.config.thresholds.critical_level {
            warn!("CRITICAL: Disk usage at {:.1}%", metrics.disk_usage_percent);
        } else if metrics.disk_usage_percent >= self.config.thresholds.warning_level {
            info!("WARNING: Disk usage at {:.1}%", metrics.disk_usage_percent);
        }
        
        Ok(())
    }
    
    pub async fn analyze(&self) -> Result<()> {
        let metrics = self.monitor.collect_metrics().await?;
        
        println!("=== BtrMind Storage Analysis ===");
        println!("Timestamp: {}", metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("Disk Usage: {:.1}%", metrics.disk_usage_percent);
        println!("Free Space: {:.1} MB", metrics.free_space_mb);
        println!("Metadata Usage: {:.1}%", metrics.metadata_usage_percent);
        println!("Fragmentation: {:.1}%", metrics.fragmentation_percent);
        
        // Threshold status
        if metrics.disk_usage_percent >= self.config.thresholds.emergency_level {
            println!("Status: 🔴 EMERGENCY");
        } else if metrics.disk_usage_percent >= self.config.thresholds.critical_level {
            println!("Status: 🟠 CRITICAL");
        } else if metrics.disk_usage_percent >= self.config.thresholds.warning_level {
            println!("Status: 🟡 WARNING");
        } else {
            println!("Status: 🟢 NORMAL");
        }
        
        Ok(())
    }
    
    pub async fn cleanup(&mut self, aggressive: bool) -> Result<()> {
        info!("Running manual cleanup (aggressive: {})", aggressive);
        
        if aggressive {
            // Run all cleanup actions
            for action in [Action::DeleteTempFiles, Action::CompressFiles, 
                          Action::BalanceMetadata, Action::CleanupSnapshots] {
                info!("Executing action: {:?}", action);
                if let Err(e) = self.executor.execute_action(action).await {
                    warn!("Action failed: {:?}", e);
                }
            }
        } else {
            // Run safe cleanup only
            for action in [Action::DeleteTempFiles, Action::CleanupSnapshots] {
                info!("Executing action: {:?}", action);
                if let Err(e) = self.executor.execute_action(action).await {
                    warn!("Action failed: {:?}", e);
                }
            }
        }
        
        Ok(())
    }
}

async fn handle_model_command(action: ModelAction, agent: &BtrMindAgent) -> Result<()> {
    use std::process::Command;
    
    match action {
        ModelAction::Train { data, output, version, validate } => {
            println!("=== Training Fragmentation Model ===");
            
            // Use config paths if not specified
            let data_path = data.unwrap_or_else(|| agent.config.fragmentation_model.training_data_path.clone());
            
            let output_path = output.unwrap_or_else(|| agent.config.fragmentation_model.model_path.clone());
            
            // Check if training data exists
            if !std::path::Path::new(&data_path).exists() {
                eprintln!("Training data not found: {}", data_path);
                eprintln!("Enable data collection in config and run monitoring to collect data.");
                return Ok(());
            }
            
            // Build Python command
            let mut cmd = Command::new("python3");
            cmd.arg("scripts/train_fragmentation_model.py")
                .arg("--data").arg(&data_path)
                .arg("--output").arg(&output_path);
            
            if let Some(ver) = version {
                cmd.arg("--version").arg(ver);
            }
            
            if validate {
                cmd.arg("--validate");
            }
            
            if agent.config.dry_run {
                cmd.arg("--verbose");
                println!("Dry run: would execute: {:?}", cmd);
                return Ok(());
            }
            
            // Execute training
            let status = cmd.status()
                .context("Failed to execute training script")?;
            
            if status.success() {
                println!("✓ Model training completed successfully");
                println!("Model saved to: {}", output_path);
            } else {
                eprintln!("✗ Model training failed");
                return Err(anyhow::anyhow!("Training script failed"));
            }
        },
        
        ModelAction::Rollback { backup } => {
            println!("=== Rolling Back Fragmentation Model ===");
            
            let output_path = agent.config.fragmentation_model.model_path.clone();
            
            // Check if backup exists
            if !std::path::Path::new(&backup).exists() {
                eprintln!("Backup file not found: {}", backup);
                return Ok(());
            }
            
            if agent.config.dry_run {
                println!("Dry run: would rollback {} to {}", output_path, backup);
                return Ok(());
            }
            
            // Execute rollback
            let mut cmd = Command::new("python3");
            cmd.arg("scripts/train_fragmentation_model.py")
                .arg("--rollback").arg(&backup)
                .arg("--output").arg(&output_path);
            
            let status = cmd.status()
                .context("Failed to execute rollback")?;
            
            if status.success() {
                println!("✓ Rollback completed successfully");
            } else {
                eprintln!("✗ Rollback failed");
                return Err(anyhow::anyhow!("Rollback failed"));
            }
        },
        
        ModelAction::ListBackups => {
            println!("=== Available Model Backups ===");
            
            let output_path = agent.config.fragmentation_model.model_path.clone();
            
            let mut cmd = Command::new("python3");
            cmd.arg("scripts/train_fragmentation_model.py")
                .arg("--list-backups")
                .arg("--output").arg(&output_path);
            
            let output = cmd.output()
                .context("Failed to list backups")?;
            
            if output.status.success() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                eprintln!("Failed to list backups: {}", String::from_utf8_lossy(&output.stderr));
            }
        },
        
        ModelAction::Info => {
            println!("=== Fragmentation Model Information ===");
            
            let model_path = agent.config.fragmentation_model.model_path.clone();
            
            match fragmentation_model::FragmentationModel::load(&model_path) {
                Ok(model) => {
                    let info = model.info();
                    println!("Model Type: {}", info.model_type);
                    println!("Training Date: {}", info.training_date);
                    println!("Version: {}", model.metadata.version);
                    println!("Number of Features: {}", info.n_features);
                    println!("Feature Names: {}", info.feature_names.join(", "));
                    
                    if let Some(metrics) = &model.metrics {
                        println!("\nPerformance Metrics:");
                        println!("  Test RMSE: {:.3}", metrics.get("test_rmse").unwrap_or(&0.0));
                        println!("  Test R²: {:.3}", metrics.get("test_r2").unwrap_or(&0.0));
                        if let Some(n_samples) = metrics.get("n_samples") {
                            println!("  Training Samples: {}", n_samples.round() as i32);
                        }
                    }
                    
                    println!("\nModel File: {}", model_path);
                    println!("Using Model: {}", agent.config.fragmentation_model.use_model);
                    println!("Fallback to Heuristic: {}", agent.config.fragmentation_model.fallback_to_heuristic);
                },
                Err(e) => {
                    println!("Model not loaded: {}", e);
                    println!("Model File: {}", model_path);
                    println!("Status: Using heuristic fallback");
                }
            }
        },
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::load(&cli.config)
        .with_context(|| format!("Failed to load config from {:?}", cli.config))?;
    
    // Override dry_run from CLI
    let mut config = config;
    if cli.dry_run {
        config.dry_run = true;
        info!("Running in DRY-RUN mode - no actions will be executed");
    }
    
    let mut agent = BtrMindAgent::new(config)?;
    
    match cli.command {
        Some(Commands::Run) | None => {
            agent.run().await?;
        },
        Some(Commands::Analyze) => {
            agent.analyze().await?;
        },
        Some(Commands::Cleanup { aggressive }) => {
            agent.cleanup(aggressive).await?;
        },
        Some(Commands::Stats) => {
            let stats = agent.learner.get_learning_stats();
            println!("=== BtrMind Learning Statistics ===");
            println!("Total Steps: {}", stats.total_steps);
            println!("Exploration Rate: {:.3}", stats.exploration_rate);
            println!("Experience Buffer Size: {}", stats.buffer_size);
            println!("Average Reward: {:.2}", stats.average_reward);
            println!("Has Trained Model: {}", stats.has_trained_model);
            println!("\nAction Distribution:");
            for (action, count) in stats.action_distribution {
                println!("  {:?}: {} times", action, count);
            }
        },
        Some(Commands::Config) => {
            println!("Configuration validation:");
            println!("Config file: {:?}", cli.config);
            println!("✓ Configuration loaded successfully");
        },
        Some(Commands::Model { action }) => {
            handle_model_command(action, &agent).await?;
        },
    }
    
    Ok(())
}
