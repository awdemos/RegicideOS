# MLE-Based Fragmentation Estimation Development Plan

## Overview
Replace the current heuristic fragmentation estimation in BtrMind with a Maximum Likelihood Estimation (MLE) trained linear model for improved accuracy and better reinforcement learning decisions.

## Current State Analysis
**Current Implementation** (btrfs.rs:153-167):
- Simple heuristic: 0% below 80% usage, linear increase above 80%
- Formula: `fragmentation = (disk_usage.used_percent - 80.0) * 2.0`
- Limitations: Doesn't account for actual filesystem fragmentation patterns

## Implementation Plan

### Phase 1: Data Collection & Ground Truth (Week 1)
**Tasks:**
1. **Enhance BtrfsMonitor** to collect comprehensive metrics:
   - Current: disk_usage, free_space, metadata_usage
   - Add: file_count, avg_file_size, write_frequency, age_distribution
2. **Implement ground truth proxy** using `btrfs filesystem df` and `btrfs inspect-internal`
3. **Add CSV logging** with structured training data
4. **Create data validation** to ensure quality samples

**Key Files to Modify:**
- `src/btrfs.rs`: Add enhanced metrics collection
- `src/config.rs`: Add data collection config
- `config/btrmind.toml`: Add data collection settings

### Phase 2: Model Development & Training (Week 2)
**Tasks:**
1. **Create Python training pipeline**:
   - Data preprocessing and feature engineering
   - Linear regression with MLE optimization
   - Model validation and performance metrics
   - Parameter serialization to JSON

2. **Implement Rust model integration**:
   - `FragmentationModel` struct with prediction logic
   - Feature standardization matching Python preprocessing
   - JSON model loading and error handling

**Key Files to Create:**
- `scripts/train_fragmentation_model.py`: Training pipeline
- `src/fragmentation_model.rs`: Model implementation
- `tests/fragmentation_model_tests.rs`: Unit tests

### Phase 3: Integration & Testing (Week 3)
**Tasks:**
1. **Replace heuristic** in `BtrfsMonitor::get_fragmentation()`
2. **Add fallback mechanism** to original heuristic if model fails
3. **Implement comprehensive testing**:
   - Unit tests for model predictions
   - Integration tests with dry-run mode
   - Performance benchmarks (<1ms prediction time)
4. **Add monitoring** for model accuracy and drift

**Key Files to Modify:**
- `src/btrfs.rs`: Replace fragmentation calculation
- `src/main.rs`: Add model loading at startup
- `src/config.rs`: Add model path configuration

### Phase 4: Deployment & Monitoring (Week 4)
**Tasks:**
1. **Update installation script** to include model file
2. **Add configuration options** for model vs heuristic
3. **Implement model retraining** automation
4. **Add performance metrics** tracking
5. **Documentation updates**

**Key Files to Modify:**
- `install.sh`: Add model file installation
- `config/btrmind.toml`: Add model configuration
- `README.md`: Update documentation
- `systemd/btrmind.service`: Add model file permissions

## Technical Implementation Details

### Data Collection Enhancement
```rust
// Enhanced metrics structure
pub struct EnhancedSystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub disk_usage_percent: f64,
    pub free_space_mb: f64,
    pub metadata_usage_percent: f64,
    pub file_count: u64,
    pub avg_file_size_mb: f64,
    pub write_frequency: f64, // writes per hour
    pub fragmentation_proxy: f64, // ground truth
}
```

### MLE Model Architecture
```rust
pub struct FragmentationModel {
    coefficients: Vec<f64>,     // [w1, w2, w3, ...]
    intercept: f64,            // bias term
    means: Vec<f64>,           // feature means for standardization
    scales: Vec<f64>,          // feature std for standardization
}

// Prediction formula:
// fragmentation = intercept + w1*z1 + w2*z2 + w3*z3 + ...
// where zi = (feature_i - mean_i) / scale_i
```

### Feature Engineering
**Primary Features:**
- Disk usage percentage
- Free space (normalized)
- Metadata usage percentage
- File count (log-transformed)
- Average file size (log-transformed)
- Write frequency (normalized)

**Target Variable:**
- Ground truth from `btrfs inspect-internal fragment-analysis`
- Fallback to file age distribution analysis

### Configuration Additions
```toml
[fragmentation_model]
# Model configuration
model_path = "/etc/btrmind/fragmentation_model.json"
use_model = true  # fallback to heuristic if false

# Data collection for training
enable_data_collection = false
training_data_path = "/var/lib/btrmind/training_data.csv"
min_samples_for_training = 500

# Fallback behavior
fallback_to_heuristic = true
prediction_timeout_ms = 100
```

## Risk Mitigation

| Risk | Mitigation Strategy |
|------|-------------------|
| Poor model accuracy | Extensive validation, fallback to heuristic |
| Data scarcity | Start with existing data, synthetic data generation |
| Performance issues | Simple linear model, benchmarking, caching |
| Model drift | Continuous monitoring, retraining triggers |
| Deployment failures | Staged rollout, configuration flags |

## Success Metrics

**Model Performance:**
- RMSE < 5% on holdout validation set
- Prediction time < 1ms
- Memory overhead < 1MB
- Model availability > 99.9%

**System Impact:**
- Improved RL decision accuracy (target: 15% better actions)
- Reduced false positive fragmentation alerts
- Better storage optimization outcomes

## Testing Strategy

1. **Unit Tests**: Model prediction accuracy, edge cases
2. **Integration Tests**: End-to-end fragmentation estimation
3. **Performance Tests**: Prediction time, memory usage
4. **Validation Tests**: Model accuracy against ground truth
5. **Fallback Tests**: Graceful degradation when model unavailable

## Rollout Plan

1. **Week 1-2**: Development and testing in feature branch
2. **Week 3**: Staged rollout with `use_model = false`
3. **Week 4**: Enable model with monitoring and fallback
4. **Month 2**: Full deployment with automated retraining

## Future Enhancements

1. **Advanced Models**: Random forest, gradient boosting
2. **Real-time Learning**: Online model updates
3. **Feature Expansion**: I/O patterns, file type analysis
4. **Cross-filesystem Generalization**: Support for non-BTRFS systems
5. **Model Compression**: Quantization for embedded deployment

## Dependencies

**New Rust Dependencies:**
- `csv` (for data collection)
- `serde_json` (for model loading)

**Python Dependencies:**
- `pandas`, `numpy`, `scikit-learn`
- `matplotlib` (for visualization)

## Resource Requirements

**Development:**
- 1 developer-week for implementation
- 500+ filesystem samples for training
- Test environment with BTRFS filesystem

**Production:**
- < 1MB additional storage for model
- Minimal CPU overhead for predictions
- Periodic retraining (weekly/monthly)

## Conclusion

The MLE-based fragmentation estimation will significantly improve BtrMind's accuracy and effectiveness. The implementation plan prioritizes safety, maintainability, and incremental deployment while providing clear success metrics and fallback mechanisms.