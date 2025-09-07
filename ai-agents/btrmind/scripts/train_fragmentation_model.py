#!/usr/bin/env python3
"""
BtrMind Fragmentation Model Training Script

Trains a linear regression model for fragmentation estimation using
Maximum Likelihood Estimation (MLE) with Gaussian noise assumption.
"""

import json
import argparse
import logging
from pathlib import Path
from datetime import datetime
from typing import Tuple, Dict, Any

import pandas as pd
import numpy as np
from sklearn.linear_model import LinearRegression
from sklearn.preprocessing import StandardScaler
from sklearn.model_selection import train_test_split
from sklearn.metrics import mean_squared_error, mean_absolute_error, r2_score

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def load_data(data_path: str) -> pd.DataFrame:
    """Load and preprocess training data from CSV file."""
    logger.info(f"Loading training data from {data_path}")
    
    if not Path(data_path).exists():
        raise FileNotFoundError(f"Training data file not found: {data_path}")
    
    df = pd.read_csv(data_path)
    logger.info(f"Loaded {len(df)} samples")
    
    # Convert timestamp to datetime
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    
    # Sort by timestamp to ensure temporal order
    df = df.sort_values('timestamp')
    
    # Remove duplicates
    df = df.drop_duplicates()
    
    logger.info(f"After preprocessing: {len(df)} samples")
    return df


def prepare_features(df: pd.DataFrame) -> Tuple[np.ndarray, np.ndarray]:
    """Prepare features and target for training."""
    # Feature engineering
    feature_cols = [
        'disk_usage_percent',
        'free_space_mb', 
        'metadata_usage_percent',
        'file_count',
        'avg_file_size_mb',
        'write_frequency'
    ]
    
    # Log-transform skewed features
    df['file_count_log'] = np.log1p(df['file_count'])
    df['avg_file_size_log'] = np.log1p(df['avg_file_size_mb'])
    df['write_frequency_log'] = np.log1p(df['write_frequency'])
    
    # Updated features with log transforms
    feature_cols = [
        'disk_usage_percent',
        'free_space_mb', 
        'metadata_usage_percent',
        'file_count_log',
        'avg_file_size_log',
        'write_frequency_log'
    ]
    
    X = df[feature_cols].values
    y = df['fragmentation_proxy'].values
    
    logger.info(f"Feature matrix shape: {X.shape}")
    logger.info(f"Target shape: {y.shape}")
    
    return X, y


def train_model(X: np.ndarray, y: np.ndarray) -> Tuple[LinearRegression, StandardScaler, Dict[str, float]]:
    """Train linear regression model with MLE."""
    logger.info("Training linear regression model")
    
    # Split data for validation
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, shuffle=False
    )
    
    # Standardize features
    scaler = StandardScaler()
    X_train_scaled = scaler.fit_transform(X_train)
    X_test_scaled = scaler.transform(X_test)
    
    # Train model (MLE for Gaussian noise)
    model = LinearRegression()
    model.fit(X_train_scaled, y_train)
    
    # Evaluate model
    y_pred_train = model.predict(X_train_scaled)
    y_pred_test = model.predict(X_test_scaled)
    
    # Calculate metrics
    metrics = {
        'train_rmse': np.sqrt(mean_squared_error(y_train, y_pred_train)),
        'test_rmse': np.sqrt(mean_squared_error(y_test, y_pred_test)),
        'train_mae': mean_absolute_error(y_train, y_pred_train),
        'test_mae': mean_absolute_error(y_test, y_pred_test),
        'train_r2': r2_score(y_train, y_pred_train),
        'test_r2': r2_score(y_test, y_pred_test),
        'n_samples': len(X_train),
        'n_features': X.shape[1]
    }
    
    logger.info("Model training completed")
    logger.info(f"Training RMSE: {metrics['train_rmse']:.3f}")
    logger.info(f"Test RMSE: {metrics['test_rmse']:.3f}")
    logger.info(f"Training R²: {metrics['train_r2']:.3f}")
    logger.info(f"Test R²: {metrics['test_r2']:.3f}")
    
    return model, scaler, metrics


def save_model(model: LinearRegression, scaler: StandardScaler, 
               metrics: Dict[str, float], output_path: str) -> None:
    """Save model parameters to JSON file."""
    logger.info(f"Saving model to {output_path}")
    
    model_params = {
        'metadata': {
            'model_type': 'linear_regression',
            'training_date': datetime.now().isoformat(),
            'framework': 'scikit-learn',
            'algorithm': 'MLE with Gaussian noise'
        },
        'coefficients': model.coef_.tolist(),
        'intercept': float(model.intercept_),
        'feature_means': scaler.mean_.tolist(),
        'feature_scales': scaler.scale_.tolist(),
        'feature_names': [
            'disk_usage_percent',
            'free_space_mb', 
            'metadata_usage_percent',
            'file_count_log',
            'avg_file_size_log',
            'write_frequency_log'
        ],
        'metrics': metrics,
        'preprocessing': {
            'log_transform': ['file_count', 'avg_file_size_mb', 'write_frequency'],
            'standardization': True
        }
    }
    
    # Create output directory if it doesn't exist
    output_file = Path(output_path)
    output_file.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w') as f:
        json.dump(model_params, f, indent=2)
    
    logger.info("Model saved successfully")


def validate_model(model_params: Dict[str, Any], X: np.ndarray, y: np.ndarray) -> None:
    """Validate model performance and check for issues."""
    logger.info("Validating model")
    
    # Reconstruct model from parameters
    from sklearn.linear_model import LinearRegression
    from sklearn.preprocessing import StandardScaler
    
    # Recreate scaler
    scaler = StandardScaler()
    scaler.mean_ = np.array(model_params['feature_means'])
    scaler.scale_ = np.array(model_params['feature_scales'])
    
    # Recreate model
    model = LinearRegression()
    model.coef_ = np.array(model_params['coefficients'])
    model.intercept_ = model_params['intercept']
    
    # Make predictions
    X_scaled = scaler.transform(X)
    y_pred = model.predict(X_scaled)
    
    # Check prediction range
    pred_min, pred_max = y_pred.min(), y_pred.max()
    logger.info(f"Prediction range: [{pred_min:.2f}, {pred_max:.2f}]")
    
    if pred_min < 0 or pred_max > 100:
        logger.warning("Predictions outside [0, 100] range detected")
    
    # Check for outliers
    residuals = y - y_pred
    outlier_mask = np.abs(residuals) > 3 * np.std(residuals)
    n_outliers = outlier_mask.sum()
    
    if n_outliers > 0:
        logger.warning(f"Found {n_outliers} outliers (|residual| > 3σ)")
    
    logger.info("Model validation completed")


def main():
    """Main training pipeline."""
    parser = argparse.ArgumentParser(description='Train BtrMind fragmentation model')
    parser.add_argument('--data', required=True, help='Path to training data CSV file')
    parser.add_argument('--output', required=True, help='Path to output model JSON file')
    parser.add_argument('--validate', action='store_true', help='Run validation after training')
    parser.add_argument('--verbose', action='store_true', help='Enable verbose logging')
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    try:
        # Load and prepare data
        df = load_data(args.data)
        X, y = prepare_features(df)
        
        # Check minimum sample size
        if len(X) < 100:
            logger.error(f"Insufficient data: {len(X)} samples, minimum 100 required")
            return 1
        
        # Train model
        model, scaler, metrics = train_model(X, y)
        
        # Save model
        model_params = {
            'coefficients': model.coef_.tolist(),
            'intercept': float(model.intercept_),
            'feature_means': scaler.mean_.tolist(),
            'feature_scales': scaler.scale_.tolist(),
            'feature_names': [
                'disk_usage_percent',
                'free_space_mb', 
                'metadata_usage_percent',
                'file_count_log',
                'avg_file_size_log',
                'write_frequency_log'
            ],
            'metrics': metrics
        }
        
        save_model(model, scaler, metrics, args.output)
        
        # Run validation if requested
        if args.validate:
            validate_model(model_params, X, y)
        
        logger.info("Training pipeline completed successfully")
        return 0
        
    except Exception as e:
        logger.error(f"Training failed: {e}")
        return 1


if __name__ == '__main__':
    exit(main())