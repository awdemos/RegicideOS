# Copyright 2024 RegicideOS Team
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit cargo git-r3

DESCRIPTION="RegicideOS system installer with AI integration"
HOMEPAGE="https://github.com/awdemos/RegicideOS"

EGIT_REPO_URI="https://github.com/awdemos/RegicideOS.git"
S="${WORKDIR}/${P}"

LICENSE="GPL-3"
SLOT="0"
KEYWORDS=""
IUSE=""

# Runtime dependencies for installation
RDEPEND="
	dev-lang/rust-bin
	sys-fs/btrfs-progs
	sys-boot/grub:2
	sys-fs/dosfstools
	sys-fs/e2fsprogs
	sys-block/parted
	sys-apps/util-linux
	net-misc/curl
	sys-fs/squashfs-tools
"

BDEPEND="
	dev-lang/rust-bin
"

DEPEND="${RDEPEND}"

src_unpack() {
	git-r3_src_unpack
	cargo_live_src_unpack
}

src_compile() {
	cd "${S}/installer" || die "Failed to enter installer directory"
	cargo_gen_config
	cargo_src_compile
}

src_test() {
	cd "${S}/installer" || die "Failed to enter installer directory"
	cargo_gen_config
	cargo_src_test
}

src_install() {
	# The installer crate is a workspace member; Cargo places the release
	# binary in the workspace root's target directory, not in
	# installer/target/release/.
	local installer_bin="${S}/target/release/installer"
	[[ -f "${installer_bin}" ]] || die "installer binary not found at ${installer_bin}"
	newbin "${installer_bin}" regicide-installer

	# Install documentation
	dodoc "${S}/README.md"
	dodoc "${S}/Handbook.md"

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
