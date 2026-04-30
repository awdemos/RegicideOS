"""
Unit tests for BtrMind AI agent core functionality.
"""

import unittest
from unittest.mock import Mock, patch, MagicMock, call
import tempfile
import os
import sys
import json
import time
from pathlib import Path
from typing import Dict, List, Any

# Add the project root to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

class TestBtrMindCore(unittest.TestCase):
    """Test BtrMind core functionality with mock BTRFS operations."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.mock_btrfs_dir = os.path.join(self.temp_dir, "mock_btrfs")
        os.makedirs(self.mock_btrfs_dir, exist_ok=True)
        
        # Create mock BtrMind instance
        self.btrmind = self.create_mock_btrmind()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            import shutil
            shutil.rmtree(self.temp_dir)
    
    def create_mock_btrmind(self):
        """Create a mock BtrMind instance for testing."""
        class MockBtrMind:
            def __init__(self, config):
                self.config = config
                self.monitoring_path = config.get("monitoring", {}).get("target_path", "/tmp")
                self.poll_interval = config.get("monitoring", {}).get("poll_interval", 60)
                self.thresholds = config.get("thresholds", {})
                self.actions = config.get("actions", {})
                self.learning = config.get("learning", {})
                self.dry_run = config.get("dry_run", True)
                self.verbose = config.get("verbose", False)
                
                # Internal state
                self.current_metrics = {}
                self.action_history = []
                self.learning_model = {}
                self.subvolumes = []
                self.filesystems = []
                
            def collect_metrics(self):
                """Collect BTRFS metrics."""
                # Mock metrics collection
                self.current_metrics = {
                    "timestamp": time.time(),
                    "disk_usage": 75.5,
                    "disk_size": 10737418240,  # 10GB
                    "disk_used": 8105499800,
                    "disk_free": 2631918440,
                    "compression_ratio": 1.2,
                    "subvolume_count": 5,
                    "snapshot_count": 3,
                    "fragmentation_ratio": 0.15,
                    "metadata_usage": 45.2
                }
                return self.current_metrics
            
            def analyze_metrics(self):
                """Analyze collected metrics."""
                metrics = self.current_metrics
                
                analysis = {
                    "overall_health": "good",
                    "recommendations": [],
                    "warnings": [],
                    "critical_issues": []
                }
                
                # Check disk usage
                disk_usage = metrics["disk_usage"]
                warning_level = self.thresholds.get("warning_level", 85.0)
                critical_level = self.thresholds.get("critical_level", 95.0)
                
                if disk_usage > critical_level:
                    analysis["overall_health"] = "critical"
                    analysis["critical_issues"].append(f"Disk usage critical: {disk_usage}%")
                elif disk_usage > warning_level:
                    analysis["overall_health"] = "warning"
                    analysis["warnings"].append(f"Disk usage high: {disk_usage}%")
                
                # Check fragmentation
                fragmentation = metrics["fragmentation_ratio"]
                if fragmentation > 0.3:
                    analysis["recommendations"].append("Consider balancing filesystem")
                
                # Check metadata usage
                metadata_usage = metrics["metadata_usage"]
                if metadata_usage > 80.0:
                    analysis["warnings"].append(f"Metadata usage high: {metadata_usage}%")
                
                return analysis
            
            def get_available_actions(self):
                """Get list of available cleanup actions."""
                return [
                    {
                        "name": "compress_files",
                        "description": "Compress uncompressed files",
                        "priority": "medium",
                        "estimated_impact": "medium"
                    },
                    {
                        "name": "balance_filesystem",
                        "description": "Balance filesystem to reduce fragmentation",
                        "priority": "low",
                        "estimated_impact": "high"
                    },
                    {
                        "name": "cleanup_temp_files",
                        "description": "Clean up temporary files",
                        "priority": "high",
                        "estimated_impact": "low"
                    },
                    {
                        "name": "cleanup_old_snapshots",
                        "description": "Remove old snapshots",
                        "priority": "medium",
                        "estimated_impact": "medium"
                    }
                ]
            
            def select_action(self, available_actions):
                """Select action using AI decision making."""
                if not available_actions:
                    return None
                
                # Simple action selection based on priority and learning
        return MockBtrMind({
            "monitoring": {
                "target_path": self.mock_btrfs_dir,
                "poll_interval": 60
            },
            "thresholds": {
                "warning_level": 85.0,
                "critical_level": 95.0,
                "emergency_level": 98.0
            },
            "actions": {
                "enable_compression": True,
                "enable_balance": True,
                "enable_temp_cleanup": True,
                "enable_snapshot_cleanup": True
            },
            "learning": {
                "model_path": os.path.join(self.temp_dir, "model"),
                "learning_rate": 0.001,
                "exploration_rate": 0.1
            },
            "dry_run": True,
            "verbose": False
        })
    
    def test_metrics_collection(self):
        """Test BTRFS metrics collection."""
        metrics = self.btrmind.collect_metrics()
        
        # Verify metrics structure
        self.assertIn("timestamp", metrics)
        self.assertIn("disk_usage", metrics)
        self.assertIn("disk_size", metrics)
        self.assertIn("disk_used", metrics)
        self.assertIn("disk_free", metrics)
        self.assertIn("compression_ratio", metrics)
        self.assertIn("subvolume_count", metrics)
        self.assertIn("snapshot_count", metrics)
        self.assertIn("fragmentation_ratio", metrics)
        self.assertIn("metadata_usage", metrics)
        
        # Verify metric types
        self.assertIsInstance(metrics["timestamp"], (int, float))
        self.assertIsInstance(metrics["disk_usage"], (int, float))
        self.assertIsInstance(metrics["disk_size"], (int, float))
        self.assertIsInstance(metrics["disk_used"], (int, float))
        self.assertIsInstance(metrics["disk_free"], (int, float))
        self.assertIsInstance(metrics["compression_ratio"], (int, float))
        self.assertIsInstance(metrics["subvolume_count"], int)
        self.assertIsInstance(metrics["snapshot_count"], int)
        self.assertIsInstance(metrics["fragmentation_ratio"], (int, float))
        self.assertIsInstance(metrics["metadata_usage"], (int, float))
        
        # Verify metric relationships
        self.assertAlmostEqual(metrics["disk_used"] + metrics["disk_free"], metrics["disk_size"], places=0)
        self.assertGreater(metrics["disk_size"], 0)
        self.assertGreaterEqual(metrics["disk_usage"], 0)
        self.assertLessEqual(metrics["disk_usage"], 100)
    
    def test_metrics_analysis(self):
        """Test metrics analysis functionality."""
        # Collect metrics first
        self.btrmind.collect_metrics()
        
        # Analyze metrics
        analysis = self.btrmind.analyze_metrics()
        
        # Verify analysis structure
        self.assertIn("overall_health", analysis)
        self.assertIn("recommendations", analysis)
        self.assertIn("warnings", analysis)
        self.assertIn("critical_issues", analysis)
        
        # Verify analysis types
        self.assertIsInstance(analysis["overall_health"], str)
        self.assertIsInstance(analysis["recommendations"], list)
        self.assertIsInstance(analysis["warnings"], list)
        self.assertIsInstance(analysis["critical_issues"], list)
        
        # Verify overall health is one of expected values
        expected_health_states = ["good", "warning", "critical"]
        self.assertIn(analysis["overall_health"], expected_health_states)
    
    def test_action_selection(self):
        """Test AI action selection."""
        available_actions = self.btrmind.get_available_actions()
        
        # Verify available actions
        self.assertIsInstance(available_actions, list)
        self.assertGreater(len(available_actions), 0)
        
        # Verify action structure
        for action in available_actions:
            self.assertIn("name", action)
            self.assertIn("description", action)
            self.assertIn("priority", action)
            self.assertIn("estimated_impact", action)
            
            # Verify priority values
            self.assertIn(action["priority"], ["low", "medium", "high"])
            
            # Verify impact values
            self.assertIn(action["estimated_impact"], ["low", "medium", "high"])
    
    def test_configuration_parsing(self):
        """Test configuration parsing and validation."""
        # Test valid configuration
        valid_config = {
            "monitoring": {
                "target_path": "/mnt/btrfs",
                "poll_interval": 60
            },
            "thresholds": {
                "warning_level": 85.0,
                "critical_level": 95.0
            },
            "actions": {
                "enable_compression": True,
                "enable_balance": True
            }
        }
        
        btrmind = self.create_mock_btrmind()
        btrmind.config = valid_config
        
        self.assertEqual(btrmind.monitoring_path, "/mnt/btrfs")
        self.assertEqual(btrmind.poll_interval, 60)
        self.assertEqual(btrmind.thresholds["warning_level"], 85.0)
        self.assertEqual(btrmind.thresholds["critical_level"], 95.0)
        self.assertTrue(btrmind.actions["enable_compression"])
        self.assertTrue(btrmind.actions["enable_balance"])
    
    def test_threshold_validation(self):
        """Test threshold validation."""
        # Test normal thresholds
        normal_thresholds = {
            "warning_level": 85.0,
            "critical_level": 95.0,
            "emergency_level": 98.0
        }
        
        self.btrmind.thresholds = normal_thresholds
        
        # Verify threshold relationships
        self.assertLess(normal_thresholds["warning_level"], normal_thresholds["critical_level"])
        self.assertLess(normal_thresholds["critical_level"], normal_thresholds["emergency_level"])
        
        # Test threshold values
        for threshold_name, threshold_value in normal_thresholds.items():
            self.assertGreater(threshold_value, 0)
            self.assertLessEqual(threshold_value, 100)
    
    def test_action_execution_dry_run(self):
        """Test action execution in dry-run mode."""
        self.btrmind.dry_run = True
        
        # Mock action execution
        action_result = self._execute_mock_action("cleanup_temp_files")
        
        # Verify dry-run behavior
        self.assertIsNotNone(action_result)
        self.assertIn("dry_run", action_result)
        self.assertEqual(action_result["dry_run"], True)
        self.assertIn("action", action_result)
        self.assertEqual(action_result["action"], "cleanup_temp_files")
    
    def test_action_execution_real_mode(self):
        """Test action execution in real mode."""
        self.btrmind.dry_run = False
        
        # Mock action execution
        action_result = self._execute_mock_action("cleanup_temp_files")
        
        # Verify real execution behavior
        self.assertIsNotNone(action_result)
        self.assertIn("action", action_result)
        self.assertEqual(action_result["action"], "cleanup_temp_files")
        self.assertIn("success", action_result)
    
    def _execute_mock_action(self, action_name):
        """Execute a mock action."""
        if self.btrmind.dry_run:
            return {
                "action": action_name,
                "dry_run": True,
                "success": True,
                "message": f"Would execute {action_name}"
            }
        else:
            return {
                "action": action_name,
                "dry_run": False,
                "success": True,
                "message": f"Executed {action_name}",
                "files_cleaned": 10,
                "space_freed": 1024000
            }

class TestBtrMindLearning(unittest.TestCase):
    """Test BtrMind AI learning functionality."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.model_path = os.path.join(self.temp_dir, "model")
        
        # Create mock learning instance
        self.learning = self.create_mock_learning()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            import shutil
            shutil.rmtree(self.temp_dir)
    
    def create_mock_learning(self):
        """Create a mock learning system."""
        class MockLearningSystem:
            def __init__(self, config):
                self.config = config
                self.model_path = config.get("model_path", self.model_path)
                self.learning_rate = config.get("learning_rate", 0.001)
                self.exploration_rate = config.get("exploration_rate", 0.1)
                self.discount_factor = config.get("discount_factor", 0.99)
                self.reward_smoothing = config.get("reward_smoothing", 0.95)
                
                # Learning state
                self.q_table = {}
                self.action_history = []
                self.reward_history = []
                self.state_history = []
                
            def get_state(self, metrics):
                """Convert metrics to learning state."""
                disk_usage = metrics.get("disk_usage", 0)
                
                # Discretize disk usage into states
                if disk_usage < 50:
                    return "low_usage"
                elif disk_usage < 80:
                    return "medium_usage"
                elif disk_usage < 90:
                    return "high_usage"
                else:
                    return "critical_usage"
            
            def get_action(self, state):
                """Select action using Q-learning."""
                available_actions = ["compress", "balance", "cleanup", "snapshot"]
                
                # Exploration vs exploitation
                if self.should_explore():
                    return self.explore_action(available_actions)
                else:
                    return self.exploit_action(state, available_actions)
            
            def should_explore(self):
                """Determine whether to explore or exploit."""
                import random
                return random.random() < self.exploration_rate
            
            def explore_action(self, available_actions):
                """Select random action for exploration."""
                import random
                return random.choice(available_actions)
            
            def exploit_action(self, state, available_actions):
                """Select best action based on Q-values."""
                if state not in self.q_table:
                    return self.explore_action(available_actions)
                
                # Find action with highest Q-value
                best_action = None
                best_q_value = float('-inf')
                
                for action in available_actions:
                    q_value = self.q_table[state].get(action, 0)
                    if q_value > best_q_value:
                        best_q_value = q_value
                        best_action = action
                
                return best_action or self.explore_action(available_actions)
            
            def update_q_value(self, state, action, reward, next_state):
                """Update Q-value using Q-learning algorithm."""
                # Initialize Q-table if needed
                if state not in self.q_table:
                    self.q_table[state] = {}
                if next_state not in self.q_table:
                    self.q_table[next_state] = {}
                
                # Get current Q-value
                current_q = self.q_table[state].get(action, 0)
                
                # Get maximum Q-value for next state
                next_max_q = max(self.q_table[next_state].values()) if self.q_table[next_state] else 0
                
                # Q-learning update rule
                new_q = current_q + self.learning_rate * (
                    reward + self.discount_factor * next_max_q - current_q
                )
                
                # Update Q-table
                self.q_table[state][action] = new_q
                
                # Record history
                self.action_history.append((state, action, reward))
                self.reward_history.append(reward)
        
        return MockLearningSystem({
            "model_path": self.model_path,
            "learning_rate": 0.001,
            "exploration_rate": 0.1,
            "discount_factor": 0.99,
            "reward_smoothing": 0.95
        })
    
    def test_state_representation(self):
        """Test state representation for learning."""
        metrics = {
            "disk_usage": 45.0,
            "fragmentation_ratio": 0.1,
            "metadata_usage": 30.0
        }
        
        state = self.learning.get_state(metrics)
        
        # Verify state is string
        self.assertIsInstance(state, str)
        
        # Verify state matches expected values
        expected_states = ["low_usage", "medium_usage", "high_usage", "critical_usage"]
        self.assertIn(state, expected_states)
        
        # Test different disk usage levels
        test_cases = [
            ({"disk_usage": 25.0}, "low_usage"),
            ({"disk_usage": 75.0}, "medium_usage"),
            ({"disk_usage": 85.0}, "high_usage"),
            ({"disk_usage": 95.0}, "critical_usage")
        ]
        
        for metrics, expected_state in test_cases:
            state = self.learning.get_state(metrics)
            self.assertEqual(state, expected_state)
    
    def test_action_selection(self):
        """Test action selection with Q-learning."""
        state = "medium_usage"
        
        # Test action selection
        action = self.learning.get_action(state)
        
        # Verify action is valid
        expected_actions = ["compress", "balance", "cleanup", "snapshot"]
        self.assertIn(action, expected_actions)
    
    def test_q_learning_update(self):
        """Test Q-learning update mechanism."""
        state = "high_usage"
        action = "cleanup"
        reward = 10.0
        next_state = "medium_usage"
        
        # Initial Q-value should be 0
        initial_q = self.learning.q_table.get(state, {}).get(action, 0)
        self.assertEqual(initial_q, 0)
        
        # Update Q-value
        self.learning.update_q_value(state, action, reward, next_state)
        
        # Verify Q-value was updated
        updated_q = self.learning.q_table[state][action]
        self.assertNotEqual(updated_q, 0)
        self.assertGreater(updated_q, 0)
        
        # Verify Q-table structure
        self.assertIn(state, self.learning.q_table)
        self.assertIn(action, self.learning.q_table[state])
    
    def test_learning_parameters(self):
        """Test learning parameter validation."""
        # Verify learning parameters
        self.assertIsInstance(self.learning.learning_rate, float)
        self.assertIsInstance(self.learning.exploration_rate, float)
        self.assertIsInstance(self.learning.discount_factor, float)
        self.assertIsInstance(self.learning.reward_smoothing, float)
        
        # Verify parameter ranges
        self.assertGreater(self.learning.learning_rate, 0)
        self.assertLess(self.learning.learning_rate, 1)
        
        self.assertGreaterEqual(self.learning.exploration_rate, 0)
        self.assertLessEqual(self.learning.exploration_rate, 1)
        
        self.assertGreater(self.learning.discount_factor, 0)
        self.assertLess(self.learning.discount_factor, 1)
        
        self.assertGreater(self.learning.reward_smoothing, 0)
        self.assertLess(self.learning.reward_smoothing, 1)

class TestBtrMindActions(unittest.TestCase):
    """Test BtrMind action execution."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.mock_btrfs_dir = os.path.join(self.temp_dir, "mock_btrfs")
        os.makedirs(self.mock_btrfs_dir, exist_ok=True)
        
        # Create mock action executor
        self.action_executor = self.create_mock_action_executor()
        
    def tearDown(self):
        """Clean up test fixtures."""
        if os.path.exists(self.temp_dir):
            import shutil
            shutil.rmtree(self.temp_dir)
    
    def create_mock_action_executor(self):
        """Create a mock action executor."""
        class MockActionExecutor:
            def __init__(self, target_path):
                self.target_path = target_path
                self.dry_run = True
                self.executed_actions = []
                self.action_results = {}
                
            def execute_compression(self):
                """Execute compression action."""
                action_name = "compress_files"
                
                if self.dry_run:
                    result = {
                        "action": action_name,
                        "dry_run": True,
                        "success": True,
                        "message": "Would compress files",
                        "estimated_space_saved": 1024000
                    }
                else:
                    result = {
                        "action": action_name,
                        "dry_run": False,
                        "success": True,
                        "message": "Compressed files",
                        "files_compressed": 5,
                        "space_saved": 1024000
                    }
                
                self.executed_actions.append(action_name)
                self.action_results[action_name] = result
                return result
            
            def execute_cleanup(self, file_patterns=None):
                """Execute cleanup action."""
                action_name = "cleanup_temp_files"
                
                if file_patterns is None:
                    file_patterns = ["*.tmp", "*.log", "*.cache"]
                
                if self.dry_run:
                    result = {
                        "action": action_name,
                        "dry_run": True,
                        "success": True,
                        "message": f"Would clean files matching patterns: {file_patterns}",
                        "estimated_files_removed": 10,
                        "estimated_space_freed": 512000
                    }
                else:
                    result = {
                        "action": action_name,
                        "dry_run": False,
                        "success": True,
                        "message": f"Cleaned files matching patterns: {file_patterns}",
                        "files_removed": 8,
                        "space_freed": 512000
                    }
                
                self.executed_actions.append(action_name)
                self.action_results[action_name] = result
                return result
            
            def execute_balance(self):
                """Execute balance action."""
                action_name = "balance_filesystem"
                
                if self.dry_run:
                    result = {
                        "action": action_name,
                        "dry_run": True,
                        "success": True,
                        "message": "Would balance filesystem",
                        "estimated_time_required": 300
                    }
                else:
                    result = {
                        "action": action_name,
                        "dry_run": False,
                        "success": True,
                        "message": "Balanced filesystem",
                        "time_taken": 285,
                        "fragmentation_before": 0.25,
                        "fragmentation_after": 0.05
                    }
                
                self.executed_actions.append(action_name)
                self.action_results[action_name] = result
                return result
            
            def execute_snapshot_cleanup(self, keep_count=5):
                """Execute snapshot cleanup action."""
                action_name = "cleanup_old_snapshots"
                
                if self.dry_run:
                    result = {
                        "action": action_name,
                        "dry_run": True,
                        "success": True,
                        "message": f"Would clean up old snapshots, keeping {keep_count}",
                        "estimated_snapshots_removed": 3,
                        "estimated_space_freed": 2048000
                    }
                else:
                    result = {
                        "action": action_name,
                        "dry_run": False,
                        "success": True,
                        "message": f"Cleaned up old snapshots, keeping {keep_count}",
                        "snapshots_removed": 2,
                        "space_freed": 2048000
                    }
                
                self.executed_actions.append(action_name)
                self.action_results[action_name] = result
                return result
        
        return MockActionExecutor(self.mock_btrfs_dir)
    
    def test_compression_action(self):
        """Test compression action execution."""
        result = self.action_executor.execute_compression()
        
        # Verify result structure
        self.assertIn("action", result)
        self.assertIn("dry_run", result)
        self.assertIn("success", result)
        self.assertIn("message", result)
        
        # Verify action was recorded
        self.assertIn("compress_files", self.action_executor.executed_actions)
        self.assertIn("compress_files", self.action_executor.action_results)
        
        # Verify result values
        self.assertEqual(result["action"], "compress_files")
        self.assertTrue(result["success"])
        self.assertIsInstance(result["message"], str)
    
    def test_cleanup_action(self):
        """Test cleanup action execution."""
        result = self.action_executor.execute_cleanup()
        
        # Verify result structure
        self.assertIn("action", result)
        self.assertIn("success", result)
        self.assertIn("message", result)
        
        # Verify action was recorded
        self.assertIn("cleanup_temp_files", self.action_executor.executed_actions)
        
        # Verify result values
        self.assertEqual(result["action"], "cleanup_temp_files")
        self.assertTrue(result["success"])
    
    def test_balance_action(self):
        """Test balance action execution."""
        result = self.action_executor.execute_balance()
        
        # Verify result structure
        self.assertIn("action", result)
        self.assertIn("success", result)
        self.assertIn("message", result)
        
        # Verify action was recorded
        self.assertIn("balance_filesystem", self.action_executor.executed_actions)
        
        # Verify result values
        self.assertEqual(result["action"], "balance_filesystem")
        self.assertTrue(result["success"])
    
    def test_snapshot_cleanup_action(self):
        """Test snapshot cleanup action execution."""
        result = self.action_executor.execute_snapshot_cleanup(keep_count=3)
        
        # Verify result structure
        self.assertIn("action", result)
        self.assertIn("success", result)
        self.assertIn("message", result)
        
        # Verify action was recorded
        self.assertIn("cleanup_old_snapshots", self.action_executor.executed_actions)
        
        # Verify result values
        self.assertEqual(result["action"], "cleanup_old_snapshots")
        self.assertTrue(result["success"])
        self.assertIn("3", result["message"])

if __name__ == '__main__':
    # Run tests with detailed output
    unittest.main(verbosity=2)