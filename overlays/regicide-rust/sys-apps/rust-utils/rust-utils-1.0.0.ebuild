# Copyright 2024 RegicideOS
# Distributed under the terms of the GNU General Public License v3

EAPI=8

CRATES="
"

inherit cargo

DESCRIPTION="Core system utilities rewritten in Rust for RegicideOS"
HOMEPAGE="https://regicideos.com"
SRC_URI="https://github.com/awdemos/RegicideOS/archive/v${PV}.tar.gz -> ${P}.tar.gz"

LICENSE="GPL-3"
SLOT="0"
KEYWORDS="~amd64"
IUSE="+exa +bat +ripgrep +fd +dust +procs +bottom +hyperfine +tokei +hyperfine +zoxide"

DEPEND="
	>=dev-lang/rust-1.75
	virtual/libiconv
	sys-libs/ncurses
"
RDEPEND="${DEPEND}"

QA_FLAGS_IGNORED=".*"

src_unpack() {
	default
	cargo_src_unpack
}

src_compile() {
	cd "${S}/rust-utils" || die

	# Build selected utilities
	local utils=()
	use exa && utils+=(exa)
	use bat && utils+=(bat)
	use ripgrep && utils+=(ripgrep)
	use fd && utils+=(fd-find)
	use dust && utils+=(dust)
	use procs && utils+=(procs)
	use bottom && utils+=(bottom)
	use hyperfine && utils+=(hyperfine)
	use tokei && utils+=(tokei)
	use zoxide && utils+=(zoxide)

	if [[ ${#utils[@]} -eq 0 ]]; then
		die "At least one utility must be enabled via USE flags"
	fi

	# Build utilities
	for util in "${utils[@]}"; do
		cd "${S}/rust-utils/${util}" || die
		cargo_src_compile
	done
}

src_install() {
	cd "${S}/rust-utils" || die

	# Install selected utilities
	if use exa; then
		cd "${S}/rust-utils/exa" || die
		dobin target/release/exa
		dosym exa /usr/bin/ls
	fi

	if use bat; then
		cd "${S}/rust-utils/bat" || die
		dobin target/release/bat
		dosym bat /usr/bin/cat
	fi

	if use ripgrep; then
		cd "${S}/rust-utils/ripgrep" || die
		dobin target/release/rg
		dosym rg /usr/bin/grep
	fi

	if use fd; then
		cd "${S}/rust-utils/fd" || die
		dobin target/release/fd
		dosym fd /usr/bin/find
	fi

	if use dust; then
		cd "${S}/rust-utils/dust" || die
		dobin target/release/dust
		dosym dust /usr/bin/du
	fi

	if use procs; then
		cd "${S}/rust-utils/procs" || die
		dobin target/release/procs
		dosym procs /usr/bin/ps
		dosym procs /usr/bin/top
	fi

	if use bottom; then
		cd "${S}/rust-utils/bottom" || die
		dobin target/release/btm
		dosym btm /usr/bin/htop
	fi

	if use hyperfine; then
		cd "${S}/rust-utils/hyperfine" || die
		dobin target/release/hyperfine
	fi

	if use tokei; then
		cd "${S}/rust-utils/tokei" || die
		dobin target/release/tokei
	fi

	if use zoxide; then
		cd "${S}/rust-utils/zoxide" || die
		dobin target/release/zoxide
	fi

	# Install documentation
	dodoc README.md
	dodoc CONTRIBUTING.md

	# Install shell completions
	insinto /usr/share/bash-completion/completions
	if use exa; then
		newins "${S}/rust-utils/exa/completions/exa.bash" exa
	fi
	if use bat; then
		newins "${S}/rust-utils/bat/completions/bat.bash" bat
	fi
	if use fd; then
		newins "${S}/rust-utils/fd/completions/fd.bash" fd
	fi
	if use ripgrep; then
		newins "${S}/rust-utils/ripgrep/complete/rg.bash" rg
	fi
	if use zoxide; then
		newins "${S}/rust-utils/zoxide/completions/zoxide.bash" zoxide
	fi

	# Install man pages
	if use exa; then
		doman "${S}/rust-utils/exa/man/exa.1"
	fi
	if use bat; then
		doman "${S}/rust-utils/bat/man/bat.1"
	fi
	if use fd; then
		doman "${S}/rust-utils/fd/man/fd.1"
	fi
	if use ripgrep; then
		doman "${S}/rust-utils/ripgrep/doc/rg.1"
	fi
}

pkg_postinst() {
	elog "RegicideOS Rust utilities have been installed."
	elog
	elog "The following utilities are available:"
	elog
	if use exa; then
		elog "  exa    - Modern ls replacement"
		elog "  ls     -> exa"
	fi
	if use bat; then
		elog "  bat    - Modern cat replacement with syntax highlighting"
		elog "  cat    -> bat"
	fi
	if use ripgrep; then
		elog "  rg     - Fast grep replacement"
		elog "  grep   -> rg"
	fi
	if use fd; then
		elog "  fd     - Fast find replacement"
		elog "  find   -> fd"
	fi
	if use dust; then
		elog "  dust   - Modern du replacement with tree view"
		elog "  du     -> dust"
	fi
	if use procs; then
		elog "  procs  - Modern ps replacement"
		elog "  ps     -> procs"
		elog "  top    -> procs"
	fi
	if use bottom; then
		elog "  btm    - Modern system monitor"
		elog "  htop   -> btm"
	fi
	if use hyperfine; then
		elog "  hyperfine - Command-line benchmarking tool"
	fi
	if use tokei; then
		elog "  tokei  - Code statistics tool"
	fi
	if use zoxide; then
		elog "  zoxide - Smarter cd command"
	fi
	elog
	elog "Note: Some utilities are symlinked to traditional command names."
	elog "To use the original commands, use the full path (e.g., /bin/cat)."
}