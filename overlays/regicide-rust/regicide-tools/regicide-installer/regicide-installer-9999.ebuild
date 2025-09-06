# Copyright 2024 RegicideOS Team
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit cargo git-r3

DESCRIPTION="RegicideOS system installer with AI integration"
HOMEPAGE="https://github.com/awdemos/RegicideOS"

EGIT_REPO_URI="https://github.com/awdemos/RegicideOS.git"
S="${WORKDIR}/${P}/installer"

LICENSE="GPL-3"
SLOT="0"
KEYWORDS=""
IUSE=""

# Runtime dependencies for installation
RDEPEND="
	>=virtual/rust-1.70.0
	sys-fs/btrfs-progs
	sys-boot/grub:2
	sys-fs/dosfstools
	sys-fs/e2fsprogs
	sys-block/parted
	sys-apps/util-linux
	net-misc/curl
	app-arch/squashfs-tools
"

BDEPEND="
	>=virtual/rust-1.70.0
"

DEPEND="${RDEPEND}"

src_unpack() {
	git-r3_src_unpack
}

src_compile() {
	cd "${S}" || die "Failed to enter installer directory"
	cargo_src_compile
}

src_test() {
	cd "${S}" || die "Failed to enter installer directory"
	cargo_src_test
}

src_install() {
	cd "${S}" || die "Failed to enter installer directory"
	
	# Install installer binary
	newbin target/release/installer regicide-installer
	
	# Install documentation
	dodoc "${WORKDIR}/${P}/README.md"
	dodoc "${WORKDIR}/${P}/Handbook.md"
	
	# Install example configurations
	insinto /usr/share/regicide/installer
	doins "${FILESDIR}/regicide-config-examples/"* 2>/dev/null || true
}

pkg_postinst() {
	elog "RegicideOS Installer ${PV} installed successfully!"
	elog ""
	elog "Usage:"
	elog "  regicide-installer                    # Interactive installation"
	elog "  regicide-installer -c config.toml    # Automated installation"
	elog ""
	elog "WARNING: This installer will completely erase the target drive!"
	elog "Only run this when installing RegicideOS on a new system."
	elog ""
	elog "For installation instructions, see:"
	elog "  /usr/share/doc/${PF}/Handbook.md"
	elog "  https://github.com/awdemos/RegicideOS/blob/main/Handbook.md"
	elog ""
	elog "Example configs: /usr/share/regicide/installer/"
}
