#!/bin/bash

# MLE Fragmentation Estimation Demo Script
# Demonstrates the new MLE-based fragmentation estimation features

set -e

echo "=== BtrMind MLE Fragmentation Estimation Demo ==="
echo

# Build the project
echo "Building BtrMind..."
cargo build --release
echo

# Test basic functionality
echo "1. Testing basic functionality..."
echo "Running: btrmind --dry-run --config config/btrmind.toml config"
./target/release/btrmind --dry-run --config config/btrmind.toml config
echo

# Test storage analysis
echo "2. Testing storage analysis..."
echo "Running: btrmind --dry-run --config config/btrmind.toml analyze"
./target/release/btrmind --dry-run --config config/btrmind.toml analyze
echo

# Test with data collection enabled
echo "3. Testing with data collection enabled..."
echo "Creating test config with data collection..."

# Create test config with data collection enabled
cat > /tmp/btrmind_test_config.toml << EOF
[monitoring]
target_path = "/tmp"
poll_interval = 60
trend_analysis_window = 24

[thresholds]
warning_level = 85.0
critical_level = 95.0
emergency_level = 98.0

[actions]
enable_compression = true
enable_balance = true
enable_snapshot_cleanup = true
enable_temp_cleanup = true
temp_paths = ["/tmp", "/var/tmp", "/var/cache"]
snapshot_keep_count = 10

[learning]
model_path = "/var/lib/btrmind/model.safetensors"
model_update_interval = 3600
reward_smoothing = 0.95
exploration_rate = 0.1
learning_rate = 0.001
discount_factor = 0.99

[fragmentation_model]
model_path = "/tmp/fragmentation_model.json"
use_model = true
enable_data_collection = true
training_data_path = "/tmp/training_data.csv"
min_samples_for_training = 10
fallback_to_heuristic = true
prediction_timeout_ms = 100

dry_run = true
EOF

echo "Running analysis with data collection..."
./target/release/btrmind --dry-run --config /tmp/btrmind_test_config.toml analyze
echo

# Check if training data was created
if [ -f "/tmp/training_data.csv" ]; then
    echo "4. Training data collection test..."
    echo "✓ Training data file created"
    echo "First few lines of training data:"
    head -n 5 /tmp/training_data.csv
    echo
else
    echo "4. Training data collection test..."
    echo "⚠ Training data file not created (this may be expected on non-BTRFS systems)"
    echo
fi

# Test Python training script
echo "5. Testing Python training script..."
echo "Creating sample training data..."

# Create sample training data
cat > /tmp/sample_training_data.csv << EOF
timestamp,disk_usage_percent,free_space_mb,metadata_usage_percent,file_count,avg_file_size_mb,write_frequency,fragmentation_proxy
2024-01-01 00:00:00,50.0,10000.0,5.0,1000,1.0,10.0,15.0
2024-01-01 01:00:00,55.0,9000.0,5.5,1100,1.1,12.0,18.0
2024-01-01 02:00:00,60.0,8000.0,6.0,1200,1.2,15.0,22.0
2024-01-01 03:00:00,65.0,7000.0,6.5,1300,1.3,18.0,28.0
2024-01-01 04:00:00,70.0,6000.0,7.0,1400,1.4,20.0,35.0
2024-01-01 05:00:00,75.0,5000.0,7.5,1500,1.5,22.0,42.0
2024-01-01 06:00:00,80.0,4000.0,8.0,1600,1.6,25.0,50.0
2024-01-01 07:00:00,85.0,3000.0,8.5,1700,1.7,28.0,60.0
2024-01-01 08:00:00,90.0,2000.0,9.0,1800,1.8,30.0,72.0
2024-01-01 09:00:00,95.0,1000.0,9.5,1900,1.9,32.0,85.0
EOF

echo "Sample training data created"
echo "Running training script..."
python3 scripts/train_fragmentation_model.py \
    --data /tmp/sample_training_data.csv \
    --output /tmp/test_fragmentation_model.json \
    --verbose

if [ -f "/tmp/test_fragmentation_model.json" ]; then
    echo "✓ Model training successful"
    echo "Model file created: /tmp/test_fragmentation_model.json"
    echo
    
    # Test model validation
    echo "6. Testing model validation..."
    echo "Model structure:"
    python3 -c "
import json
with open('/tmp/test_fragmentation_model.json', 'r') as f:
    model = json.load(f)
print(f'  Model type: {model[\"metadata\"][\"model_type\"]}')
print(f'  Training date: {model[\"metadata\"][\"training_date\"]}')
print(f'  Number of features: {len(model[\"coefficients\"])}')
print(f'  Features: {model[\"feature_names\"]}')
print(f'  Test RMSE: {model[\"metrics\"][\"test_rmse\"]:.3f}')
"
else
    echo "⚠ Model training failed"
fi

echo
echo "7. Running unit tests..."
cargo test fragmentation_model -- --nocapture
echo

echo "8. Performance benchmarks..."
echo "Testing model prediction performance..."
python3 -c "
import time
import json
import sys
sys.path.append('scripts')

# Load model
with open('/tmp/test_fragmentation_model.json', 'r') as f:
    model = json.load(f)

# Simulate prediction performance
def predict_fragmentation(features, model):
    # Simple prediction simulation
    raw_features = [
        features[0],  # disk_usage_percent
        features[1],  # free_space_mb
        features[2],  # metadata_usage_percent
        (1.0 + features[3]),  # file_count_log
        (1.0 + features[4]),  # avg_file_size_log
        (1.0 + features[5]),  # write_frequency_log
    ]
    
    # Standardize
    standardized = []
    for i, x in enumerate(raw_features):
        z = (x - model['feature_means'][i]) / model['feature_scales'][i]
        standardized.append(z)
    
    # Predict
    prediction = model['intercept']
    for i, coef in enumerate(model['coefficients']):
        prediction += coef * standardized[i]
    
    return max(0.0, min(100.0, prediction))

# Test features
test_features = [80.0, 2000.0, 8.0, 1500.0, 1.5, 25.0]

# Benchmark
start_time = time.time()
for _ in range(1000):
    prediction = predict_fragmentation(test_features, model)
end_time = time.time()

avg_time = (end_time - start_time) / 1000 * 1000  # Convert to ms
print(f'  Average prediction time: {avg_time:.3f} ms')
print(f'  Sample prediction: {prediction:.2f}%')
"
echo

# Cleanup
echo "9. Cleanup..."
rm -f /tmp/btrmind_test_config.toml
rm -f /tmp/training_data.csv
rm -f /tmp/sample_training_data.csv
rm -f /tmp/test_fragmentation_model.json
echo "✓ Cleanup completed"
echo

echo "=== Demo Complete ==="
echo
echo "Summary of implemented features:"
echo "✓ Enhanced metrics collection with CSV logging"
echo "✓ Python training script with MLE linear regression"
echo "✓ Rust FragmentationModel with JSON serialization"
echo "✓ Cross-platform support (macOS/Linux)"
echo "✓ Comprehensive configuration options"
echo "✓ Fallback to heuristic estimation"
echo "✓ Unit tests and validation"
echo "✓ Performance benchmarks"
echo
echo "Next steps:"
echo "1. Collect real training data from BTRFS systems"
echo "2. Train model with actual fragmentation measurements"
echo "3. Deploy to production with monitoring"
echo "4. Implement model retraining automation"