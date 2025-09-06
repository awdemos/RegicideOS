# Copyright 1999-2024 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit bash-completion-r1 flag-o-matic multiprocessing rust-toolchain toolchain-funcs

if [[ ${PV} == *9999 ]]; then
	EGIT_REPO_URI="https://github.com/rust-lang/rust.git"
	inherit git-r3
	S="${WORKDIR}/${P}/src"
else
	SRC_URI="https://forge.rust-lang.org/infra/channel-timeline.html -> rust-${PV}.tar.xz"
	KEYWORDS="amd64 ~arm ~arm64 ~ppc64 ~riscv ~x86"
fi

DESCRIPTION="Systems programming language from Mozilla (RegicideOS Enhanced)"
HOMEPAGE="https://www.rust-lang.org/"

LICENSE="|| ( MIT Apache-2.0 ) BSD-1 BSD-2 BSD-4 UoI-NCSA"
SLOT="stable"
IUSE="clippy cpu_flags_x86_sse2 debug doc miri nightly parallel-compiler profiler rust-analyzer rust-src rustfmt system-bootstrap system-llvm test wasm +embedded_targets"

# RegicideOS: Add embedded targets support
IUSE="${IUSE} embedded_targets"

RESTRICT="!test? ( test )"

BDEPEND="
	>=virtual/rust-1.71.0
	app-eselect/eselect-rust
	|| (
		>=sys-devel/gcc-4.7
		>=sys-devel/clang-3.5
	)
	system-bootstrap? ( !>=dev-lang/rust-1.75.0 )
"

DEPEND="
	>=app-eselect/eselect-rust-20190311
	|| (
		>=sys-devel/gcc-4.7
		>=sys-devel/clang-3.5
	)
	system-llvm? ( 
		>=sys-devel/llvm-15:=
		<sys-devel/llvm-18:=
	)
	>=dev-libs/libffi-3.0.2:=
	>=dev-libs/openssl-1.0.1:=
	virtual/libintl
"

RDEPEND="
	${DEPEND}
	app-eselect/eselect-rust
	rust-analyzer? ( ~dev-util/rust-analyzer-${PV} )
"

PATCHES=(
	"${FILESDIR}"/rust-embedded-targets.patch
)

QA_FLAGS_IGNORED="
	usr/lib/${PN}/${PV}/bin/.*
	usr/lib/${PN}/${PV}/lib/.*
	usr/lib/${PN}/${PV}/libexec/.*
"

# RegicideOS: Enhanced embedded targets
RUST_EMBEDDED_TARGETS=(
	"thumbv6m-none-eabi"
	"thumbv7m-none-eabi"
	"thumbv7em-none-eabi"
	"thumbv7em-none-eabihf"
	"thumbv8m.base-none-eabi"
	"thumbv8m.main-none-eabi"
	"thumbv8m.main-none-eabihf"
	"riscv32i-unknown-none-elf"
	"riscv32imc-unknown-none-elf"
	"riscv32imac-unknown-none-elf"
	"riscv64gc-unknown-none-elf"
	"riscv64imac-unknown-none-elf"
	"armv7-unknown-linux-musleabihf"
	"aarch64-unknown-linux-musl"
	"x86_64-unknown-linux-musl"
	"wasm32-unknown-unknown"
	"wasm32-wasi"
)

src_prepare() {
	if use system-bootstrap; then
		local rustc_version=$(rustc --version | cut -d' ' -f2)
		local rust_toolchain="beta-${rustc_version}"
		local rustdate=$(rustc --version --verbose | grep -oP 'commit-date: \K[0-9-]+')
		einfo "Using system rust ${rustc_version} from ${rustdate}"
	fi

	default
}

src_configure() {
	# RegicideOS: Enhanced configuration for embedded development
	local myeconfargs=(
		--build=$(rust_abi)
		--host=$(rust_abi) 
		--target=$(rust_abi)
		--python="${PYTHON}"
		--set="rust.debug-assertions=$(usex debug true false)"
		--set="rust.optimize=$(usex debug false true)"
		--set="build.rustc-static-libstdcpp=$(usex system-bootstrap false true)"
		--set="target.$(rust_abi).cc=$(tc-getBUILD_CC)"
		--set="target.$(rust_abi).cxx=$(tc-getBUILD_CXX)" 
		--set="target.$(rust_abi).ar=$(tc-getBUILD_AR)"
		--set="target.$(rust_abi).ranlib=$(tc-getBUILD_RANLIB)"
		--set="target.$(rust_abi).linker=$(tc-getBUILD_CC)"
	)

	# RegicideOS: Add embedded targets if requested
	if use embedded_targets; then
		local target
		for target in "${RUST_EMBEDDED_TARGETS[@]}"; do
			myeconfargs+=( --target="${target}" )
		done
		einfo "Enabled embedded targets: ${RUST_EMBEDDED_TARGETS[*]}"
	fi

	# Additional RegicideOS customizations
	myeconfargs+=(
		--enable-vendor
		--enable-verbose-tests
		--disable-docs
		--enable-compiler-docs
		--disable-optimize-tests
		--enable-option-checking
		--disable-profiler
		$(usex clippy --enable-clippy --disable-clippy)
		$(usex doc --enable-docs --disable-docs)
		$(usex miri --enable-miri --disable-miri)
		$(usex profiler --enable-profiler --disable-profiler)
		$(usex rustfmt --enable-rustfmt --disable-rustfmt)
		$(usex rust-analyzer --enable-rust-analyzer --disable-rust-analyzer)
		$(usex rust-src --enable-rust-src --disable-rust-src)
		$(usex system-llvm --llvm-root="${EPREFIX}/usr" "")
		$(usex wasm --target=wasm32-unknown-unknown "")
	)

	econf "${myeconfargs[@]}"
}

src_compile() {
	# RegicideOS: Optimized build process
	local jobs=$(makeopts_jobs)
	local loadavg=$(makeopts_loadavg)
	
	# Use parallel compilation if available
	if use parallel-compiler; then
		export RUSTFLAGS="${RUSTFLAGS} -C codegen-units=${jobs}"
	fi
	
	./x.py build --jobs=${jobs} $(usex doc --stage=2 --stage=1) \
		library/std \
		$(usex clippy clippy "") \
		$(usex miri miri "") \
		$(usex rustfmt rustfmt "") \
		$(usex rust-analyzer rust-analyzer "") \
		|| die "compile failed"
}

src_test() {
	# RegicideOS: Focused testing for stability
	if use test; then
		./x.py test --jobs=$(makeopts_jobs) --no-fail-fast \
			library/std \
			$(usex clippy clippy "") \
			$(usex rustfmt rustfmt "")
	fi
}

src_install() {
	# RegicideOS: Enhanced installation
	./x.py install --jobs=$(makeopts_jobs) || die "install failed"
	
	# Install embedded targets if enabled
	if use embedded_targets; then
		local target
		for target in "${RUST_EMBEDDED_TARGETS[@]}"; do
			if [[ -d "${D}/usr/lib/rustlib/${target}" ]]; then
				einfo "Installed embedded target: ${target}"
			fi
		done
	fi

	# RegicideOS: Install additional development tools
	if use embedded_targets; then
		# Install cross-compilation helpers
		dobin "${FILESDIR}/regicide-cross-compile"
		
		# Install target documentation
		if use doc; then
			docinto embedded-targets
			dodoc "${FILESDIR}/embedded-targets.md"
		fi
	fi
	
	# Install bash completions
	if use bash-completion; then
		dobashcomp src/etc/bash_completion.d/cargo
	fi
}

pkg_postinst() {
	eselect rust update --if-unset

	# RegicideOS: Post-install configuration
	elog "RegicideOS Enhanced Rust ${PV} installed successfully!"
	elog ""
	
	if use embedded_targets; then
		elog "Embedded targets are now available:"
		local target
		for target in "${RUST_EMBEDDED_TARGETS[@]}"; do
			elog "  - ${target}"
		done
		elog ""
		elog "Use 'cargo build --target <target>' to cross-compile"
		elog "Example: cargo build --target thumbv6m-none-eabi"
	fi
	
	if use rust-analyzer; then
		elog "Rust-analyzer is installed and ready for LSP-compatible editors"
	fi
	
	elog "For RegicideOS development:"
	elog "  - AI agents: Use 'cargo new --bin my-agent'"
	elog "  - System tools: Use 'cargo new --bin my-tool'" 
	elog "  - Libraries: Use 'cargo new --lib my-lib'"
	elog ""
	elog "Documentation: https://github.com/awdemos/RegicideOS/blob/main/overlays/regicide-rust/README.md"
}

pkg_postrm() {
	eselect rust update --if-unset
}
