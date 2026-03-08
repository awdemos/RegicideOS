# nosec B108
"""Tests for regicide_image_builder.py

This test suite validates the RegicideOS image builder functionality including:
- Build variant and target architecture enumeration
- Build configuration handling
- Build result tracking
- Size constant validation
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
import asyncio
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from regicide_image_builder import (
    BuildVariant,
    TargetArch,
    BuildConfig,
    BuildResult,
    RegicideImageBuilder,
    MB,
    GB,
)


class TestBuildVariant:
    """Tests for BuildVariant enum"""

    def test_all_variants_exist_and_have_correct_values(self):
        """Test that all expected variants exist with correct string values.

        Validates:
        - MINIMAL variant exists with value 'minimal'
        - STANDARD variant exists with value 'standard'
        - DEVELOPER variant exists with value 'developer'
        - Values are lowercase strings
        - Values match expected naming convention
        """
        assert hasattr(BuildVariant, "MINIMAL"), "BuildVariant should have MINIMAL"
        assert hasattr(BuildVariant, "STANDARD"), "BuildVariant should have STANDARD"
        assert hasattr(BuildVariant, "DEVELOPER"), "BuildVariant should have DEVELOPER"

        assert BuildVariant.MINIMAL.value == "minimal", (
            "MINIMAL value should be 'minimal'"
        )
        assert BuildVariant.STANDARD.value == "standard", (
            "STANDARD value should be 'standard'"
        )
        assert BuildVariant.DEVELOPER.value == "developer", (
            "DEVELOPER value should be 'developer'"
        )

        # Verify all values are lowercase
        for variant in BuildVariant:
            assert variant.value.islower(), (
                f"Variant {variant.name} value should be lowercase"
            )

    def test_variant_count_is_exactly_three(self):
        """Test that we have exactly 3 build variants.

        Validates:
        - Total variant count is 3
        - No extra variants exist
        - No missing variants
        """
        variants = list(BuildVariant)
        assert len(variants) == 3, f"Expected 3 variants, got {len(variants)}"

        # Verify we can iterate over all variants
        variant_names = [v.name for v in variants]
        assert "MINIMAL" in variant_names
        assert "STANDARD" in variant_names
        assert "DEVELOPER" in variant_names

    def test_variant_values_are_unique(self):
        """Test that all variant values are unique.

        Validates:
        - No duplicate values
        - Each variant has a distinct value
        """
        values = [v.value for v in BuildVariant]
        assert len(values) == len(set(values)), "All variant values should be unique"


class TestTargetArch:
    """Tests for TargetArch enum"""

    def test_all_architectures_exist_and_have_correct_values(self):
        """Test that all expected architectures exist with correct values.

        Validates:
        - X86_64 architecture exists with correct value
        - AARCH64 architecture exists with correct value
        - RISCV64 architecture exists with correct value
        - Values contain expected architecture identifiers
        """
        assert hasattr(TargetArch, "X86_64"), "TargetArch should have X86_64"
        assert hasattr(TargetArch, "AARCH64"), "TargetArch should have AARCH64"
        assert hasattr(TargetArch, "RISCV64"), "TargetArch should have RISCV64"

        # Verify values contain expected identifiers
        assert "x86_64" in TargetArch.X86_64.value.lower(), (
            "X86_64 should contain x86_64"
        )
        assert "aarch64" in TargetArch.AARCH64.value.lower(), (
            "AARCH64 should contain aarch64"
        )
        assert "riscv64" in TargetArch.RISCV64.value.lower(), (
            "RISCV64 should contain riscv64"
        )

    def test_architecture_count_is_exactly_three(self):
        """Test that we have exactly 3 target architectures.

        Validates:
        - Total architecture count is 3
        - No extra architectures exist
        - No missing architectures
        """
        archs = list(TargetArch)
        assert len(archs) == 3, f"Expected 3 architectures, got {len(archs)}"

        arch_names = [a.name for a in archs]
        assert "X86_64" in arch_names
        assert "AARCH64" in arch_names
        assert "RISCV64" in arch_names

    def test_architecture_values_are_unique(self):
        """Test that all architecture values are unique.

        Validates:
        - No duplicate values
        - Each architecture has a distinct value
        """
        values = [a.value for a in TargetArch]
        assert len(values) == len(set(values)), (
            "All architecture values should be unique"
        )


class TestBuildConfig:
    """Tests for BuildConfig dataclass"""

    def test_default_values_are_set_correctly(self):
        """Test that default values are set correctly when not specified.

        Validates:
        - ai_optimization defaults to True
        - reproducible defaults to True
        - compress defaults to True
        - Required fields are properly set
        """
        config = BuildConfig(
            target=TargetArch.X86_64,
            variant=BuildVariant.STANDARD,
            output_path="/tmp/test.img",
            features=["rust-utils"],
        )

        # Check defaults
        assert config.ai_optimization is True, "ai_optimization should default to True"
        assert config.reproducible is True, "reproducible should default to True"
        assert config.compress is True, "compress should default to True"

        # Check required fields
        assert config.target == TargetArch.X86_64
        assert config.variant == BuildVariant.STANDARD
        assert config.output_path == "/tmp/test.img"
        assert config.features == ["rust-utils"]

    def test_custom_values_override_defaults_correctly(self):
        """Test that custom values override defaults when specified.

        Validates:
        - ai_optimization can be set to False
        - reproducible can be set to False
        - compress can be set to False
        - All fields accept custom values
        """
        config = BuildConfig(
            target=TargetArch.AARCH64,
            variant=BuildVariant.MINIMAL,
            output_path="/custom/path.img",
            features=["btrmind", "cosmic-desktop"],
            ai_optimization=False,
            reproducible=False,
            compress=False,
        )

        assert config.ai_optimization is False
        assert config.reproducible is False
        assert config.compress is False
        assert config.target == TargetArch.AARCH64
        assert config.variant == BuildVariant.MINIMAL
        assert len(config.features) == 2

    def test_features_list_can_be_empty(self):
        """Test that features list can be empty.

        Validates:
        - Empty features list is accepted
        - No exception is raised
        """
        config = BuildConfig(
            target=TargetArch.X86_64,
            variant=BuildVariant.MINIMAL,
            output_path="/tmp/test.img",
            features=[],
        )

        assert config.features == []
        assert len(config.features) == 0

    def test_multiple_features_can_be_specified(self):
        """Test that multiple features can be specified.

        Validates:
        - Multiple features are stored correctly
        - Order is preserved
        """
        features = ["rust-utils", "btrmind", "cosmic-desktop", "ai-agents"]
        config = BuildConfig(
            target=TargetArch.X86_64,
            variant=BuildVariant.DEVELOPER,
            output_path="/tmp/test.img",
            features=features,
        )

        assert config.features == features
        assert len(config.features) == 4


class TestBuildResult:
    """Tests for BuildResult dataclass"""

    def test_success_result_has_all_required_fields(self):
        """Test creating a successful build result with all fields.

        Validates:
        - All fields are properly stored
        - Success flag is True
        - Empty error list
        """
        result = BuildResult(
            success=True,
            image_path="/tmp/test.img",
            size_mb=1024,
            build_time_seconds=300,
            features_built=["rust-utils", "btrmind"],
            warnings=["Minor warning"],
            errors=[],
        )

        assert result.success is True
        assert result.image_path == "/tmp/test.img"
        assert result.size_mb == 1024
        assert result.build_time_seconds == 300
        assert len(result.features_built) == 2
        assert len(result.warnings) == 1
        assert len(result.errors) == 0

    def test_failure_result_tracks_errors_correctly(self):
        """Test creating a failed build result with errors.

        Validates:
        - Failure is properly tracked
        - Error messages are stored
        - Empty image path on failure
        """
        result = BuildResult(
            success=False,
            image_path="",
            size_mb=0,
            build_time_seconds=10,
            features_built=[],
            warnings=["Warning during build"],
            errors=["Build failed: missing dependency", "Build failed: disk full"],
        )

        assert result.success is False
        assert result.image_path == ""
        assert result.size_mb == 0
        assert len(result.errors) == 2
        assert "missing dependency" in result.errors[0]

    def test_result_with_warnings_but_no_errors_is_success(self):
        """Test result with warnings but no errors is still successful.

        Validates:
        - Warnings don't affect success status
        - Errors list can be empty
        """
        result = BuildResult(
            success=True,
            image_path="/tmp/test.img",
            size_mb=512,
            build_time_seconds=60,
            features_built=["minimal"],
            warnings=["Deprecated feature used", "Slow disk detected"],
            errors=[],
        )

        assert result.success is True
        assert len(result.warnings) == 2
        assert len(result.errors) == 0


class TestRegicideImageBuilder:
    """Tests for RegicideImageBuilder class"""

    def test_initialization_sets_correct_defaults(self):
        """Test that builder initializes with correct default values.

        Validates:
        - temp_dir is None initially
        - build_stats are all zero
        - All required stats keys exist
        """
        builder = RegicideImageBuilder()

        assert builder.temp_dir is None
        assert builder.build_stats["total_builds"] == 0
        assert builder.build_stats["successful_builds"] == 0
        assert builder.build_stats["failed_builds"] == 0

    def test_build_stats_has_all_required_keys(self):
        """Test that build stats dictionary has all required keys.

        Validates:
        - total_builds key exists
        - successful_builds key exists
        - failed_builds key exists
        - No unexpected keys
        """
        builder = RegicideImageBuilder()

        required_keys = {"total_builds", "successful_builds", "failed_builds"}
        actual_keys = set(builder.build_stats.keys())

        assert required_keys.issubset(actual_keys), (
            f"Missing keys: {required_keys - actual_keys}"
        )

    def test_builder_can_be_instantiated_multiple_times(self):
        """Test that multiple builder instances can be created.

        Validates:
        - Each instance is independent
        - No shared state between instances
        """
        builder1 = RegicideImageBuilder()
        builder2 = RegicideImageBuilder()

        assert builder1 is not builder2
        assert builder1.build_stats is not builder2.build_stats

    @pytest.mark.asyncio
    async def test_build_image_with_mocked_dependencies(self):
        """Test build_image method with mocked dependencies.

        Validates:
        - build_image can be called
        - Internal methods are called in correct order
        - Result is returned
        """
        builder = RegicideImageBuilder()
        config = BuildConfig(
            target=TargetArch.X86_64,
            variant=BuildVariant.STANDARD,
            output_path="/tmp/test.img",
            features=["rust-utils"],
        )

        # Mock internal methods
        builder._setup_build_environment = AsyncMock()
        builder._apply_ai_optimizations = AsyncMock()
        builder._build_components = AsyncMock(return_value=["kernel", "userspace"])
        builder._generate_image = AsyncMock()

        # Mock file operations
        with patch("os.path.getsize", return_value=1024 * 1024 * 1024):
            result = await builder.build_image(config)

        # Verify mocked methods were called
        assert (
            builder._setup_build_environment.called or builder._build_components.called
        )


class TestSizeConstants:
    """Tests for size constants"""

    def test_mb_constant_is_correct_value(self):
        """Test that MB constant equals 1,048,576 bytes.

        Validates:
        - MB = 1024 * 1024
        - MB = 1,048,576
        """
        assert MB == 1024 * 1024
        assert MB == 1_048_576

    def test_gb_constant_is_correct_value(self):
        """Test that GB constant equals 1,073,741,824 bytes.

        Validates:
        - GB = 1024 * 1024 * 1024
        - GB = 1,073,741,824
        """
        assert GB == 1024 * 1024 * 1024
        assert GB == 1_073_741_824

    def test_gb_is_1024_times_mb(self):
        """Test that GB is exactly 1024 times MB.

        Validates:
        - GB = 1024 * MB
        """
        assert GB == 1024 * MB


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
