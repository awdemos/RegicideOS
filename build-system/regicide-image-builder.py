#!/usr/bin/env python3
"""
RegicideOS Image Builder (2025 Edition)
Modern system image creation with AI optimization and reproducible builds
"""

import asyncio
import json
import os
import subprocess
import tempfile
import shutil
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from enum import Enum

import typer
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn
from rich.table import Table
from rich.panel import Panel

console = Console()

class BuildVariant(Enum):
    MINIMAL = "minimal"
    STANDARD = "standard"
    DEVELOPER = "developer"

class TargetArch(Enum):
    X86_64 = "x86_64-unknown-linux-gnu"
    AARCH64 = "aarch64-unknown-linux-gnu"
    RISCV64 = "riscv64gc-unknown-linux-gnu"

@dataclass
class BuildConfig:
    target: TargetArch
    variant: BuildVariant
    output_path: str
    features: List[str]
    ai_optimization: bool = True
    reproducible: bool = True
    compress: bool = True

@dataclass
class BuildResult:
    success: bool
    image_path: str
    size_mb: int
    build_time_seconds: int
    features_built: List[str]
    warnings: List[str]
    errors: List[str]

class RegicideImageBuilder:
    """Modern RegicideOS image builder with 2025 features"""

    def __init__(self):
        self.console = Console()
        self.temp_dir = None
        self.build_stats = {
            "total_builds": 0,
            "successful_builds": 0,
            "failed_builds": 0,
            "average_build_time": 0
        }

    async def build_image(self, config: BuildConfig) -> BuildResult:
        """Build a RegicideOS system image"""
        build_start_time = asyncio.get_event_loop().time()

        self.console.print(f"[bold blue]🏗️  Building {config.variant.value} variant for {config.target.value}[/bold blue]")

        try:
            # Setup build environment
            await self._setup_build_environment(config)

            # Apply AI optimizations if enabled
            if config.ai_optimization:
                await self._apply_ai_optimizations(config)

            # Build components
            components_built = await self._build_components(config)

            # Generate system image
            image_path = await self._generate_system_image(config)

            # Post-process image
            final_image_path = await self._post_process_image(image_path, config)

            # Run validation
            validation_result = await self._validate_image(final_image_path, config)

            # Calculate build time
            build_time = int(asyncio.get_event_loop().time() - build_start_time)
            image_size = os.path.getsize(final_image_path) // (1024 * 1024)

            # Update build stats
            self.build_stats["total_builds"] += 1
            if validation_result:
                self.build_stats["successful_builds"] += 1
            else:
                self.build_stats["failed_builds"] += 1

            return BuildResult(
                success=validation_result,
                image_path=final_image_path,
                size_mb=image_size,
                build_time_seconds=build_time,
                features_built=components_built,
                warnings=[],
                errors=[]
            )

        except Exception as e:
            self.console.print(f"[bold red]❌ Build failed: {str(e)}[/bold red]")
            return BuildResult(
                success=False,
                image_path="",
                size_mb=0,
                build_time_seconds=0,
                features_built=[],
                warnings=[],
                errors=[str(e)]
            )

        finally:
            # Cleanup
            if self.temp_dir:
                shutil.rmtree(self.temp_dir, ignore_errors=True)

    async def _setup_build_environment(self, config: BuildConfig) -> None:
        """Setup modern build environment with 2025 tooling"""
        self.console.print("🔧 Setting up build environment...")

        # Create temporary build directory
        self.temp_dir = tempfile.mkdtemp(prefix="regicide-build-")

        # Initialize modern container-based build environment
        build_env_setup = [
            f"docker run --rm -v {self.temp_dir}:/build",
            "ghcr.io/regicideos/build-env:2025.1",
            "regicide-setup",
            f"--target {config.target.value}",
            f"--variant {config.variant.value}",
            f"--features {','.join(config.features)}"
        ]

        result = await self._run_command(" ".join(build_env_setup))
        if result.returncode != 0:
            raise RuntimeError(f"Build environment setup failed: {result.stderr}")

    async def _apply_ai_optimizations(self, config: BuildConfig) -> None:
        """Apply AI-driven build optimizations"""
        self.console.print("🤖 Applying AI optimizations...")

        # Use ML model to optimize build parameters
        optimization_config = {
            "target": config.target.value,
            "variant": config.variant.value,
            "features": config.features,
            "build_context": {
                "cpu_count": os.cpu_count(),
                "memory_gb": self._get_total_memory_gb(),
                "available_disk_gb": self._get_available_disk_gb()
            }
        }

        # Run AI optimization service
        optimization_cmd = [
            "regicide-ai-optimize",
            "--config", json.dumps(optimization_config),
            "--output", f"{self.temp_dir}/optimization.json"
        ]

        result = await self._run_command(" ".join(optimization_cmd))
        if result.returncode == 0:
            self.console.print("[green]✅ AI optimizations applied successfully[/green]")

    async def _build_components(self, config: BuildConfig) -> List[str]:
        """Build system components using modern parallel compilation"""
        self.console.print("📦 Building components...")

        components = []
        build_tasks = []

        # Core system components
        if "rust-utils" in config.features:
            build_tasks.append(self._build_rust_utils(config))
        if "btrmind" in config.features:
            build_tasks.append(self._build_btrmind(config))
        if "cosmic-desktop" in config.features:
            build_tasks.append(self._build_cosmic_desktop(config))

        # Run builds in parallel
        results = await asyncio.gather(*build_tasks, return_exceptions=True)

        # Process results
        for i, result in enumerate(results):
            component_name = ["rust-utils", "btrmind", "cosmic-desktop"][i]
            if isinstance(result, Exception):
                self.console.print(f"[red]❌ {component_name} build failed: {result}[/red]")
            else:
                components.append(component_name)
                self.console.print(f"[green]✅ {component_name} built successfully[/green]")

        return components

    async def _build_rust_utils(self, config: BuildConfig) -> str:
        """Build Rust system utilities"""
        cmd = [
            "cargo", "build",
            "--target", config.target.value,
            "--release",
            "--features", "modern-2025"
        ]
        result = await self._run_command(" ".join(cmd), cwd="rust-utils")
        if result.returncode != 0:
            raise RuntimeError(f"Rust utils build failed: {result.stderr}")
        return "rust-utils"

    async def _build_btrmind(self, config: BuildConfig) -> str:
        """Build BtrMind AI agent"""
        cmd = [
            "cargo", "build",
            "--target", config.target.value,
            "--release",
            "--features", "ai-optimization-2025"
        ]
        result = await self._run_command(" ".join(cmd), cwd="ai-agents/btrmind")
        if result.returncode != 0:
            raise RuntimeError(f"BtrMind build failed: {result.stderr}")
        return "btrmind"

    async def _build_cosmic_desktop(self, config: BuildConfig) -> str:
        """Build Cosmic Desktop with RegicideOS theming"""
        cmd = [
            "cosmic-build",
            "--target", config.target.value,
            "--theme", "regicide-2025",
            "--optimize", "ai-driven"
        ]
        result = await self._run_command(" ".join(cmd))
        if result.returncode != 0:
            raise RuntimeError(f"Cosmic Desktop build failed: {result.stderr}")
        return "cosmic-desktop"

    async def _generate_system_image(self, config: BuildConfig) -> str:
        """Generate system image using modern imaging tools"""
        self.console.print("🖼️  Generating system image...")

        image_path = f"{self.temp_dir}/regicideos-{config.target.value}-{config.variant.value}.img"

        # Use modern image builder with 2025 features
        build_cmd = [
            "regicide-image-creator",
            "--target", config.target.value,
            "--variant", config.variant.value,
            "--output", image_path,
            "--features", ",".join(config.features),
            "--compression", "zstd-22" if config.compress else "none",
            "--reproducible" if config.reproducible else "",
            "--ai-optimized" if config.ai_optimization else ""
        ]

        result = await self._run_command(" ".join(build_cmd))
        if result.returncode != 0:
            raise RuntimeError(f"Image generation failed: {result.stderr}")

        return image_path

    async def _post_process_image(self, image_path: str, config: BuildConfig) -> str:
        """Post-process the generated image"""
        self.console.print("🔧 Post-processing image...")

        final_path = config.output_path or f"regicideos-{config.target.value}-{config.variant.value}.img"

        # Apply AI-driven optimizations
        if config.ai_optimization:
            optimize_cmd = [
                "regicide-image-optimize",
                "--input", image_path,
                "--output", final_path,
                "--algorithm", "quantum-2025",
                "--target-size", "minimal"
            ]
            result = await self._run_command(" ".join(optimize_cmd))
            if result.returncode != 0:
                self.console.print("[yellow]⚠️  Image optimization failed, using original[/yellow]")
                shutil.copy2(image_path, final_path)
        else:
            shutil.copy2(image_path, final_path)

        return final_path

    async def _validate_image(self, image_path: str, config: BuildConfig) -> bool:
        """Validate the generated system image"""
        self.console.print("🔍 Validating image...")

        validation_tasks = [
            self._validate_image_structure(image_path),
            self._validate_bootloader(image_path, config.target.value),
            self._validate_filesystem(image_path),
            self._validate_components(image_path, config.features)
        ]

        results = await asyncio.gather(*validation_tasks, return_exceptions=True)

        # Check if all validations passed
        all_passed = all(
            not isinstance(result, Exception) and result
            for result in results
        )

        if all_passed:
            self.console.print("[green]✅ All validations passed[/green]")
        else:
            self.console.print("[red]❌ Some validations failed[/red]")

        return all_passed

    async def _validate_image_structure(self, image_path: str) -> bool:
        """Validate image structure"""
        cmd = ["regicide-validate", "--structure", image_path]
        result = await self._run_command(" ".join(cmd))
        return result.returncode == 0

    async def _validate_bootloader(self, image_path: str, target: str) -> bool:
        """Validate bootloader configuration"""
        cmd = ["regicide-validate", "--bootloader", "--target", target, image_path]
        result = await self._run_command(" ".join(cmd))
        return result.returncode == 0

    async def _validate_filesystem(self, image_path: str) -> bool:
        """Validate filesystem integrity"""
        cmd = ["regicide-validate", "--filesystem", image_path]
        result = await self._run_command(" ".join(cmd))
        return result.returncode == 0

    async def _validate_components(self, image_path: str, features: List[str]) -> bool:
        """Validate installed components"""
        cmd = ["regicide-validate", "--components", ",".join(features), image_path]
        result = await self._run_command(" ".join(cmd))
        return result.returncode == 0

    async def _run_command(self, cmd: str, cwd: Optional[str] = None) -> subprocess.CompletedProcess:
        """Run a command asynchronously"""
        process = await asyncio.create_subprocess_shell(
            cmd,
            cwd=cwd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        stdout, stderr = await process.communicate()
        return subprocess.CompletedProcess(
            args=cmd,
            returncode=process.returncode,
            stdout=stdout.decode() if stdout else "",
            stderr=stderr.decode() if stderr else ""
        )

    def _get_total_memory_gb(self) -> int:
        """Get total system memory in GB"""
        try:
            with open("/proc/meminfo", "r") as f:
                for line in f:
                    if line.startswith("MemTotal:"):
                        return int(line.split()[1]) // (1024 * 1024)
        except:
            return 8  # Default assumption

    def _get_available_disk_gb(self) -> int:
        """Get available disk space in GB"""
        try:
            stat = shutil.disk_usage(self.temp_dir or "/tmp")
            return stat.free // (1024 * 1024 * 1024)
        except:
            return 50  # Default assumption

    def show_build_stats(self):
        """Display build statistics"""
        table = Table(title="RegicideOS Build Statistics")
        table.add_column("Metric", style="cyan")
        table.add_column("Value", style="green")

        table.add_row("Total Builds", str(self.build_stats["total_builds"]))
        table.add_row("Successful Builds", str(self.build_stats["successful_builds"]))
        table.add_row("Failed Builds", str(self.build_stats["failed_builds"]))
        if self.build_stats["total_builds"] > 0:
            success_rate = (self.build_stats["successful_builds"] / self.build_stats["total_builds"]) * 100
            table.add_row("Success Rate", f"{success_rate:.1f}%")

        self.console.print(table)

# CLI Application
app = typer.Typer(help="RegicideOS Image Builder - 2025 Edition")

@app.command()
def build(
    target: str = typer.Option("x86_64", help="Target architecture"),
    variant: str = typer.Option("standard", help="Build variant"),
    output: str = typer.Option("", help="Output path"),
    features: str = typer.Option("rust-utils,btrmind,cosmic-desktop", help="Comma-separated features"),
    ai_optimization: bool = typer.Option(True, help="Enable AI optimization"),
    reproducible: bool = typer.Option(True, help="Make build reproducible"),
    compress: bool = typer.Option(True, help="Compress output image")
):
    """Build a RegicideOS system image"""
    async def build_async():
        builder = RegicideImageBuilder()

        config = BuildConfig(
            target=TargetArch(target),
            variant=BuildVariant(variant),
            output_path=output,
            features=features.split(","),
            ai_optimization=ai_optimization,
            reproducible=reproducible,
            compress=compress
        )

        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            console=console,
        ) as progress:
            task = progress.add_task("Building RegicideOS...", total=None)

            result = await builder.build_image(config)

            progress.remove_task(task)

        if result.success:
            console.print(f"[bold green]✅ Build successful![/bold green]")
            console.print(f"📦 Image: {result.image_path}")
            console.print(f"📏 Size: {result.size_mb} MB")
            console.print(f"⏱️  Build time: {result.build_time_seconds}s")
            console.print(f"🔧 Features: {', '.join(result.features_built)}")
        else:
            console.print(f"[bold red]❌ Build failed[/bold red]")
            for error in result.errors:
                console.print(f"  {error}")

        builder.show_build_stats()

    asyncio.run(build_async())

@app.command()
def list_targets():
    """List available target architectures"""
    table = Table(title="Available Targets")
    table.add_column("Architecture", style="cyan")
    table.add_column("Description", style="green")

    for arch in TargetArch:
        table.add_row(arch.value, f"RegicideOS for {arch.value}")

    console.print(table)

@app.command()
def list_variants():
    """List available build variants"""
    table = Table(title="Available Variants")
    table.add_column("Variant", style="cyan")
    table.add_column("Description", style="green")

    table.add_row("minimal", "Core system with essential components")
    table.add_row("standard", "Full system with AI and desktop")
    table.add_row("developer", "Development environment with tools")

    console.print(table)

if __name__ == "__main__":
    app()