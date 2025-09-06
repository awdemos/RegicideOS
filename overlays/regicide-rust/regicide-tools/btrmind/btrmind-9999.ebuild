# Copyright 2024 RegicideOS Team
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit cargo systemd git-r3

DESCRIPTION="AI-powered BTRFS storage monitoring and optimization for RegicideOS"
HOMEPAGE="https://github.com/awdemos/RegicideOS"

EGIT_REPO_URI="https://github.com/awdemos/RegicideOS.git"
S="${WORKDIR}/${P}/ai-agents/btrmind"

LICENSE="GPL-3"
SLOT="0"
KEYWORDS=""
IUSE="systemd"

# Runtime dependencies
RDEPEND="
	>=virtual/rust-1.70.0
	systemd? ( sys-apps/systemd )
	sys-fs/btrfs-progs
	sys-process/lsof
	app-admin/sudo
"

BDEPEND="
	>=virtual/rust-1.70.0
	>=virtual/pkgconfig-2
"

DEPEND="${RDEPEND}"

src_unpack() {
	git-r3_src_unpack
}

src_compile() {
	cd "${S}" || die "Failed to enter btrmind directory"
	cargo_src_compile
}

src_test() {
	cd "${S}" || die "Failed to enter btrmind directory"  
	cargo_src_test
}

src_install() {
	cd "${S}" || die "Failed to enter btrmind directory"
	
	# Install binary
	dobin target/release/btrmind
	
	# Install configuration
	insinto /etc/btrmind
	doins config/btrmind.toml
	
	# Install systemd service
	if use systemd; then
		systemd_dounit systemd/btrmind.service
	fi
	
	# Create directories with proper ownership
	diropts -m0755
	dodir /var/lib/btrmind
	dodir /var/log/btrmind
	
	# Install documentation
	dodoc README.md
	
	# Install test script
	dobin test_btrmind.sh
	newbin test_btrmind.sh btrmind-test
}

pkg_preinst() {
	# Create btrmind user and group
	enewgroup btrmind
	enewuser btrmind -1 -1 /var/lib/btrmind btrmind
}

pkg_postinst() {
	# Fix ownership of data directories
	chown -R btrmind:btrmind /var/lib/btrmind /var/log/btrmind

	elog "BtrMind AI Storage Agent ${PV} installed successfully!"
	elog ""
	elog "Configuration: /etc/btrmind/config.toml"
	elog ""
	elog "To start BtrMind:"
	if use systemd; then
		elog "  systemctl enable btrmind"
		elog "  systemctl start btrmind"
	else
		elog "  OpenRC support not yet implemented"
	fi
	elog ""
	elog "Manual operations:"
	elog "  btrmind analyze               # Analyze storage state"
	elog "  btrmind cleanup --aggressive  # Manual cleanup"
	elog "  btrmind stats                # Learning progress"
	elog "  btrmind-test                 # Run test suite"
	elog ""
	elog "Monitor with:"
	elog "  journalctl -u btrmind -f     # Service logs"
	elog "  btrmind stats                # AI learning stats"
	elog ""
	elog "The AI will learn and optimize your storage over 7-14 days."
	elog "Initial actions use rule-based heuristics until learning converges."
}

pkg_prerm() {
	# Stop service before removal
	if use systemd && systemctl is-active btrmind >/dev/null 2>&1; then
		einfo "Stopping BtrMind service..."
		systemctl stop btrmind
	fi
}
