# Copyright 2024 RegicideOS
# Distributed under the terms of the GNU General Public License v3

EAPI=8

PYTHON_COMPAT=( python3_{10..12} )

inherit check-reqs estack flag-o-matic multiprocessing multilib \
	prefix python-any-r1 rust-toolchain toolchain-funcs

MY_P="rustc-${PV}-src"
SRC_URI="
	https://static.rust-lang.org/dist/${MY_P}.tar.xz
	https://dev.gentoo.org/~polynomial-c/${CATEGORY}/${PN}/${PN}-nightly-snapshot-2023-09-01.tar.xz
"

DESCRIPTION="Systems programming language from Mozilla, enhanced for RegicideOS"
HOMEPAGE="https://www.rust-lang.org/"

LICENSE="|| ( MIT Apache-2.0 ) BSD-1 BSD-2 BSD-4 UoI-NCSA"
SLOT="stable"
KEYWORDS="~amd64 ~arm64"
IUSE="clippy cpu_flags_x86_sse2 debug doc embedded_targets +nightly parallel-compiler rls rustfmt system-bootstrap system-llvm test"

# These are the targets included in the default installation
ALL_TARGETS=(
	"aarch64-unknown-linux-gnu"
	"arm-unknown-linux-gnueabihf"
	"riscv64gc-unknown-linux-gnu"
	"thumbv7m-none-eabi"
	"thumbv7em-none-eabihf"
	"wasm32-wasi"
	"x86_64-unknown-linux-gnu"
)

# Additional embedded targets for RegicideOS
EMBEDDED_TARGETS=(
	"thumbv6m-none-eabi"
	"thumbv8m.base-none-eabi"
	"thumbv8m.main-none-eabi"
	"riscv32i-unknown-none-elf"
	"riscv32imc-unknown-none-elf"
	"riscv64gc-unknown-none-elf"
)

# Additional AI/ML targets
AI_TARGETS=(
	"x86_64-pc-windows-gnu"
	"wasm32-unknown-unknown"
)

DEPEND="
	app-arch/xz-utils
	app-eselect/eselect-rust
	sys-devel/llvm:17
	>=sys-devel/clang-17
"

RDEPEND="
	${DEPEND}
	!sys-devel/rust:stable
"

pkg_setup() {
	rust_pkg_setup

	if use system-bootstrap; then
		local rustc
		for rustc in rust{c,-bin,-std}; do
			if has_version -b "dev-lang/${rustc}[system-llvm(+)]"; then
				ewarn "${rustc} is installed with system-llvm use flag. This may cause issues."
			fi
		done
	fi
}

src_prepare() {
	default

	# Apply RegicideOS patches
	eapply "${FILESDIR}"/${PN}-1.80.0-regicide.patch

	# Enable embedded targets if requested
	if use embedded_targets; then
		einfo "Enabling embedded targets for RegicideOS"
		echo 'target.'${EMBEDDED_TARGETS[@]// /.target.'}' > config.toml
	fi

	# Enable AI/ML targets
	echo 'target.'${AI_TARGETS[@]// /.target.'}' >> config.toml

	# Set default build targets
	cat >> config.toml <<- EOT
		[build]
		extended = true
		tools = ["cargo", "clippy", "rustfmt", "rust-analyzer"]
		python = "${EPYTHON}"
	EOT
}

src_configure() {
	local tools="cargo,clippy,rustfmt"
	use doc && tools+=",rust-docs"
	use rls && tools+=",rls"
	use rust-analyzer && tools+=",rust-analyzer"

	cat >> config.toml <<- EOT
		[rust]
		parallel-compiler = $(usex parallel-compiler true false)
		deny-warnings = false

		[install]
		prefix = "${EPREFIX}/usr"
		libdir = "$(get_libdir)"
		docdir = "share/doc/${PF}"
		mandir = "share/man"

		[target.x86_64-unknown-linux-gnu]
		llvm-config = "$(get_llvm_prefix "${LLVM_MAX_SLOT}")/bin/llvm-config"
		cflags = "${CFLAGS}"
		cxxflags = "${CXXFLAGS}"
		ldflags = "${LDFLAGS}"

		[target.aarch64-unknown-linux-gnu]
		cc = "aarch64-linux-gnu-gcc"
		cxx = "aarch64-linux-gnu-g++"
		ar = "aarch64-linux-gnu-ar"

		[target.arm-unknown-linux-gnueabihf]
		cc = "arm-linux-gnueabihf-gcc"
		cxx = "arm-linux-gnueabihf-g++"
		ar = "arm-linux-gnueabihf-ar"

		[target.riscv64gc-unknown-linux-gnu]
		cc = "riscv64-linux-gnu-gcc"
		cxx = "riscv64-linux-gnu-g++"
		ar = "riscv64-linux-gnu-ar"
	EOT

	# Configure embedded targets
	local target
	for target in "${EMBEDDED_TARGETS[@]}"; do
		cat >> config.toml <<- EOT
			[target.${target}]
			cc = "${target%%-*}-gcc"
			cxx = "${target%%-*}-g++"
			ar = "${target%%=*}-ar"
		EOT
	done
}

src_compile() {
	# Use system LLVM for better performance
	export LIBCLANG_PATH="$(get_llvm_prefix "${LLVM_MAX_SLOT}")/$(get_libdir)"

	# Build with parallel compilation
	local jobs=$(makeopts_jobs)
	export RUSTFLAGS="-C debuginfo=2 -C target-cpu=native"
	export CARGO_BUILD_JOBS=${jobs}

	default
}

src_install() {
	local DESTDIR="${D}"
	local std_libdir="${D}/usr/$(get_libdir)"

	default

	# Install additional toolchain components
	exeinto /usr/bin
	doexe target/release/cargo

	# Install rust-analyzer if built
	if use rust-analyzer; then
		doexe target/release/rust-analyzer
	fi

	# Install toolchain configuration
	insinto /usr/share/${PN}
	doins "${FILESDIR}"/toolchain.toml

	# Install RegicideOS-specific tools
	dobin "${FILESDIR}"/regicide-cross-compile

	# Install documentation for RegicideOS-specific features
	dodoc "${FILESDIR}"/RegicideOS-Rust-Guide.md

	# Set up eselect integration
	newbashcomp "${FILESDIR}"/rust.bash-completion rust
	newzshcomp "${FILESDIR}"/rust.zsh-completion _rust
}

pkg_postinst() {
	elog "RegicideOS enhanced Rust toolchain has been installed."
	elog
	elog "This build includes:"
	elog "  - Standard Rust toolchain (cargo, clippy, rustfmt)"
	elog "  - Embedded targets: $(IFS=','; echo "${EMBEDDED_TARGETS[*]}")"
	elog "  - AI/ML targets: $(IFS=','; echo "${AI_TARGETS[*]}")"
	elog
	elog "Cross-compilation helper:"
	elog "  regicide-cross-compile --list-targets"
	elog "  regicide-cross-compile --target thumbv7em-none-eabihf"
	elog
	elog "Toolchain configuration: /usr/share/rust/toolchain.toml"

	rust_pkg_postinst
}