use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentationModel {
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub feature_means: Vec<f64>,
    pub feature_scales: Vec<f64>,
    pub feature_names: Vec<String>,
    pub metadata: ModelMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_type: String,
    pub training_date: String,
    pub framework: String,
    pub algorithm: String,
}

impl FragmentationModel {
    /// Load model from JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading fragmentation model from {:?}", path);
        
        let start_time = Instant::now();
        
        if !path.exists() {
            anyhow::bail!("Fragmentation model file not found: {:?}", path);
        }
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read model file: {:?}", path))?;
        
        let model: FragmentationModel = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse model file: {:?}", path))?;
        
        let load_time = start_time.elapsed();
        debug!("Model loaded in {:?}", load_time);
        
        // Validate model structure
        model.validate()?;
        
        info!("Fragmentation model loaded successfully");
        info!("Model type: {}", model.metadata.model_type);
        info!("Training date: {}", model.metadata.training_date);
        info!("Features: {}", model.feature_names.len());
        
        Ok(model)
    }
    
    /// Predict fragmentation percentage using the trained model
    pub fn predict(&self, features: &FragmentationFeatures) -> Result<f64> {
        let start_time = Instant::now();
        
        // Prepare feature vector with log transforms
        let raw_features = vec![
            features.disk_usage_percent,
            features.free_space_mb,
            features.metadata_usage_percent,
            (1.0 + features.file_count).ln(),  // log(1 + x)
            (1.0 + features.avg_file_size_mb).ln(),
            (1.0 + features.write_frequency).ln(),
        ];
        
        if raw_features.len() != self.coefficients.len() {
            anyhow::bail!(
                "Feature mismatch: expected {} features, got {}",
                self.coefficients.len(),
                raw_features.len()
            );
        }
        
        // Standardize features: z = (x - mean) / scale
        let standardized_features: Vec<f64> = raw_features
            .iter()
            .enumerate()
            .map(|(i, &x)| (x - self.feature_means[i]) / self.feature_scales[i])
            .collect();
        
        // Calculate prediction: y = intercept + Σ(coefficient * feature)
        let mut prediction = self.intercept;
        for (i, &coef) in self.coefficients.iter().enumerate() {
            prediction += coef * standardized_features[i];
        }
        
        // Clamp prediction to valid range [0, 100]
        prediction = prediction.max(0.0).min(100.0);
        
        let prediction_time = start_time.elapsed();
        debug!("Fragmentation prediction: {:.2} (computed in {:?})", prediction, prediction_time);
        
        Ok(prediction)
    }
    
    /// Validate model structure and parameters
    fn validate(&self) -> Result<()> {
        if self.coefficients.is_empty() {
            anyhow::bail!("Model has no coefficients");
        }
        
        if self.feature_means.len() != self.coefficients.len() {
            anyhow::bail!("Feature means length mismatch");
        }
        
        if self.feature_scales.len() != self.coefficients.len() {
            anyhow::bail!("Feature scales length mismatch");
        }
        
        if self.feature_names.len() != self.coefficients.len() {
            anyhow::bail!("Feature names length mismatch");
        }
        
        // Check for zero scales (would cause division by zero)
        for (i, &scale) in self.feature_scales.iter().enumerate() {
            if scale == 0.0 {
                anyhow::bail!("Feature scale at index {} is zero", i);
            }
        }
        
        debug!("Model validation passed");
        Ok(())
    }
    
    /// Get model information
    pub fn info(&self) -> ModelInfo {
        ModelInfo {
            model_type: self.metadata.model_type.clone(),
            training_date: self.metadata.training_date.clone(),
            n_features: self.coefficients.len(),
            feature_names: self.feature_names.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FragmentationFeatures {
    pub disk_usage_percent: f64,
    pub free_space_mb: f64,
    pub metadata_usage_percent: f64,
    pub file_count: f64,
    pub avg_file_size_mb: f64,
    pub write_frequency: f64,
}

impl FragmentationFeatures {
    pub fn new(
        disk_usage_percent: f64,
        free_space_mb: f64,
        metadata_usage_percent: f64,
        file_count: f64,
        avg_file_size_mb: f64,
        write_frequency: f64,
    ) -> Self {
        Self {
            disk_usage_percent,
            free_space_mb,
            metadata_usage_percent,
            file_count,
            avg_file_size_mb,
            write_frequency,
        }
    }
    
    /// Create from enhanced system metrics
    pub fn from_metrics(metrics: &crate::EnhancedSystemMetrics) -> Self {
        Self {
            disk_usage_percent: metrics.disk_usage_percent,
            free_space_mb: metrics.free_space_mb,
            metadata_usage_percent: metrics.metadata_usage_percent,
            file_count: metrics.file_count as f64,
            avg_file_size_mb: metrics.avg_file_size_mb,
            write_frequency: metrics.write_frequency,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub model_type: String,
    pub training_date: String,
    pub n_features: usize,
    pub feature_names: Vec<String>,
}

/// Fallback heuristic when model is not available
pub fn heuristic_fragmentation(disk_usage_percent: f64) -> f64 {
    // Original heuristic: 0% below 80% usage, linear increase above 80%
    if disk_usage_percent > 80.0 {
        (disk_usage_percent - 80.0) * 2.0
    } else {
        0.0
    }.min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    fn create_test_model() -> FragmentationModel {
        FragmentationModel {
            coefficients: vec![0.5, -0.3, 0.2, 0.1, -0.1, 0.05],
            intercept: 10.0,
            feature_means: vec![50.0, 5000.0, 5.0, 8.0, 2.0, 1.0],
            feature_scales: vec![20.0, 2000.0, 2.0, 3.0, 1.0, 0.5],
            feature_names: vec![
                "disk_usage_percent".to_string(),
                "free_space_mb".to_string(),
                "metadata_usage_percent".to_string(),
                "file_count_log".to_string(),
                "avg_file_size_log".to_string(),
                "write_frequency_log".to_string(),
            ],
            metadata: ModelMetadata {
                model_type: "linear_regression".to_string(),
                training_date: "2024-01-01T00:00:00Z".to_string(),
                framework: "scikit-learn".to_string(),
                algorithm: "MLE with Gaussian noise".to_string(),
            },
        }
    }
    
    #[test]
    fn test_model_prediction() {
        let model = create_test_model();
        
        let features = FragmentationFeatures {
            disk_usage_percent: 80.0,
            free_space_mb: 1000.0,
            metadata_usage_percent: 10.0,
            file_count: 1000.0,
            avg_file_size_mb: 5.0,
            write_frequency: 2.0,
        };
        
        let prediction = model.predict(&features).unwrap();
        assert!(prediction >= 0.0 && prediction <= 100.0);
    }
    
    #[test]
    fn test_prediction_clamping() {
        let model = create_test_model();
        
        // Test with extreme values that would produce out-of-range predictions
        let features = FragmentationFeatures {
            disk_usage_percent: 200.0,  // Very high usage
            free_space_mb: 0.0,        // No free space
            metadata_usage_percent: 100.0,
            file_count: 1000000.0,
            avg_file_size_mb: 1000.0,
            write_frequency: 1000.0,
        };
        
        let prediction = model.predict(&features).unwrap();
        assert!(prediction >= 0.0 && prediction <= 100.0);
    }
    
    #[test]
    fn test_model_validation() {
        let model = create_test_model();
        assert!(model.validate().is_ok());
        
        // Test invalid model (zero scale)
        let mut invalid_model = model.clone();
        invalid_model.feature_scales[0] = 0.0;
        assert!(invalid_model.validate().is_err());
    }
    
    #[test]
    fn test_heuristic_fragmentation() {
        assert_eq!(heuristic_fragmentation(70.0), 0.0);
        assert_eq!(heuristic_fragmentation(80.0), 0.0);
        assert_eq!(heuristic_fragmentation(85.0), 10.0);
        assert_eq!(heuristic_fragmentation(95.0), 30.0);
        assert_eq!(heuristic_fragmentation(150.0), 100.0); // Clamped
    }
    
    #[test]
    fn test_feature_creation() {
        let features = FragmentationFeatures::new(
            50.0, 1000.0, 5.0, 100.0, 1.0, 0.5
        );
        
        assert_eq!(features.disk_usage_percent, 50.0);
        assert_eq!(features.free_space_mb, 1000.0);
        assert_eq!(features.metadata_usage_percent, 5.0);
        assert_eq!(features.file_count, 100.0);
        assert_eq!(features.avg_file_size_mb, 1.0);
        assert_eq!(features.write_frequency, 0.5);
    }
    
    #[test]
    fn test_model_serialization() {
        let model = create_test_model();
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: FragmentationModel = serde_json::from_str(&json).unwrap();
        
        assert_eq!(model.coefficients, deserialized.coefficients);
        assert_eq!(model.intercept, deserialized.intercept);
        assert_eq!(model.feature_names, deserialized.feature_names);
    }
    
    #[test]
    fn test_model_info() {
        let model = create_test_model();
        let info = model.info();
        
        assert_eq!(info.model_type, "linear_regression");
        assert_eq!(info.n_features, 6);
        assert_eq!(info.feature_names.len(), 6);
    }
}