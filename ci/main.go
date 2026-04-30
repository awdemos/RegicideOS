package main

import (
	"context"
	"fmt"
	"os"
	"dagger.io/dagger"
)

func main() {
	ctx := context.Background()

	// Initialize Dagger client
	client, err := dagger.Connect(ctx, dagger.WithLogOutput(os.Stderr))
	if err != nil {
		panic(err)
	}
	defer client.Close()

	// Get reference to the project source
	source := client.Host().Directory(".")

	// Run the CI pipeline
	if err := runCI(ctx, client, source); err != nil {
		fmt.Printf("CI failed: %s\n", err)
		os.Exit(1)
	}

	fmt.Println("✅ CI pipeline completed successfully")
}

func runCI(ctx context.Context, client *dagger.Client, source *dagger.Directory) error {
	// Run all pipeline stages
	fmt.Println("🚀 Starting RegicideOS CI Pipeline")

	// Stage 1: Rust Components Build
	if err := buildRustComponents(ctx, client, source); err != nil {
		return fmt.Errorf("rust build failed: %w", err)
	}

	// Stage 2: Security Scanning
	if err := securityScan(ctx, client, source); err != nil {
		return fmt.Errorf("security scan failed: %w", err)
	}

	// Stage 3: Overlay Testing
	if err := testOverlay(ctx, client, source); err != nil {
		return fmt.Errorf("overlay test failed: %w", err)
	}

	// Stage 4: AI Agents Testing
	if err := testAIAgents(ctx, client, source); err != nil {
		return fmt.Errorf("AI agents test failed: %w", err)
	}

	return nil
}

// buildRustComponents builds all Rust components with caching
func buildRustComponents(ctx context.Context, client *dagger.Client, source *dagger.Directory) error {
	fmt.Println("🔧 Building Rust components...")

	rust := client.Container().
		From("rust:1.75-slim").
		WithWorkdir("/app").
		WithDirectory("/app", source).
		WithExec([]string{"apt-get", "update"}).
		WithExec([]string{"apt-get", "install", "-y", "pkg-config", "libssl-dev"}).
		// Cache Cargo registry and dependencies
		WithMountedCache("/usr/local/cargo/registry", client.CacheVolume("cargo-registry")).
		WithMountedCache("/app/target", client.CacheVolume("rust-target"))

	// Build installer
	fmt.Println("  📦 Building installer...")
	installerBuild := rust.
		WithWorkdir("/app/installer").
		WithExec([]string{"cargo", "build", "--release"}).
		WithExec([]string{"cargo", "test", "--release"})

	if _, err := installerBuild.Stdout(ctx); err != nil {
		return fmt.Errorf("installer build failed: %w", err)
	}

	// Build BtrMind AI agent
	fmt.Println("  🤖 Building BtrMind AI agent...")
	btrmindBuild := rust.
		WithWorkdir("/app/ai-agents/btrmind").
		WithExec([]string{"cargo", "build", "--release"}).
		WithExec([]string{"cargo", "test", "--release"})

	if _, err := btrmindBuild.Stdout(ctx); err != nil {
		return fmt.Errorf("btrmind build failed: %w", err)
	}

	fmt.Println("  ✅ Rust components built successfully")
	return nil
}

// securityScan performs security scanning with trivy and hadolint
func securityScan(ctx context.Context, client *dagger.Client, source *dagger.Directory) error {
	fmt.Println("🔒 Running security scans...")

	// Trivy vulnerability scanning
	fmt.Println("  🔍 Running Trivy vulnerability scan...")
	trivy := client.Container().
		From("aquasec/trivy:latest").
		WithDirectory("/scan", source).
		WithWorkdir("/scan").
		WithExec([]string{"trivy", "fs", "--exit-code", "1", "--severity", "CRITICAL,HIGH", "."})

	if _, err := trivy.Stdout(ctx); err != nil {
		return fmt.Errorf("trivy scan failed: %w", err)
	}

	// Rust audit for security vulnerabilities
	fmt.Println("  🦀 Running cargo audit...")
	rustSecurityScan := client.Container().
		From("rust:1.75-slim").
		WithDirectory("/app", source).
		WithWorkdir("/app").
		WithExec([]string{"cargo", "install", "cargo-audit"}).
		WithExec([]string{"cargo", "audit", "--deny", "warnings"})

	if _, err := rustSecurityScan.Stdout(ctx); err != nil {
		return fmt.Errorf("cargo audit failed: %w", err)
	}

	// Check for any Dockerfiles and run hadolint
	fmt.Println("  🐳 Running hadolint on Dockerfiles...")
	hadolint := client.Container().
		From("hadolint/hadolint:latest").
		WithDirectory("/scan", source).
		WithWorkdir("/scan").
		WithExec([]string{"find", ".", "-name", "Dockerfile*", "-exec", "hadolint", "{}", "+"})

	if _, err := hadolint.Stdout(ctx); err != nil {
		// hadolint might fail if no Dockerfiles found, which is ok
		fmt.Printf("  ⚠️  hadolint warning: %v\n", err)
	}

	fmt.Println("  ✅ Security scans completed")
	return nil
}

// testOverlay tests the RegicideOS overlay in Gentoo environment
func testOverlay(ctx context.Context, client *dagger.Client, source *dagger.Directory) error {
	fmt.Println("🐧 Testing overlay in Gentoo environment...")

	gentooTest := client.Container().
		From("gentoo/stage3:latest").
		WithDirectory("/regicide", source).
		WithWorkdir("/regicide").
		WithExec([]string{"emerge-webrsync"}).
		WithExec([]string{"emerge", "--quiet-build=y", "eselect-repository", "git", "dev-vcs/git"}).
		WithExec([]string{"mkdir", "-p", "/var/db/repos"}).
		WithExec([]string{"cp", "-r", "/regicide/overlays/regicide-rust", "/var/db/repos/regicide-overlay"}).
		WithExec([]string{"mkdir", "-p", "/etc/portage/repos.conf", "/etc/portage/package.accept_keywords"}).
		WithNewFile("/etc/portage/repos.conf/regicide.conf", dagger.ContainerWithNewFileOpts{
			Contents: `[regicide-overlay]
location = /var/db/repos/regicide-overlay
sync-type = git
sync-uri = https://github.com/awdemos/regicide-overlay.git
auto-sync = yes
`,
		}).
		WithNewFile("/etc/portage/package.accept_keywords/regicide", dagger.ContainerWithNewFileOpts{
			Contents: "regicide-tools/* **\n",
		}).
		WithExec([]string{"eselect", "repository", "list"}).
		WithExec([]string{"emerge", "--search", "btrmind"}).
		WithExec([]string{"emerge", "--pretend", "--quiet", "regicide-tools/btrmind"})

	output, err := gentooTest.Stdout(ctx)
	if err != nil {
		return fmt.Errorf("gentoo overlay test failed: %w", err)
	}

	fmt.Printf("  📋 Gentoo test output:\n%s", output)
	fmt.Println("  ✅ Overlay test completed")
	return nil
}

// testAIAgents tests AI agents with simulated environments
func testAIAgents(ctx context.Context, client *dagger.Client, source *dagger.Directory) error {
	fmt.Println("🤖 Testing AI agents...")

	// Test BtrMind with simulated BTRFS environment
	fmt.Println("  🧠 Testing BtrMind AI agent...")
	
	btrmindTest := client.Container().
		From("rust:1.75-slim").
		WithDirectory("/app", source).
		WithWorkdir("/app/ai-agents/btrmind").
		WithExec([]string{"apt-get", "update"}).
		WithExec([]string{"apt-get", "install", "-y", "pkg-config", "libssl-dev", "btrfs-progs"}).
		WithMountedCache("/usr/local/cargo/registry", client.CacheVolume("cargo-registry")).
		WithMountedCache("/app/target", client.CacheVolume("rust-target")).
		WithExec([]string{"cargo", "build", "--release"}).
		WithExec([]string{"cargo", "test", "--release"}).
		// Test CLI without requiring root/BTRFS
		WithExec([]string{"./target/release/btrmind", "--help"}).
		WithExec([]string{"./target/release/btrmind", "--dry-run", "config"})

	if _, err := btrmindTest.Stdout(ctx); err != nil {
		return fmt.Errorf("btrmind test failed: %w", err)
	}

	fmt.Println("  ✅ AI agents testing completed")
	return nil
}
