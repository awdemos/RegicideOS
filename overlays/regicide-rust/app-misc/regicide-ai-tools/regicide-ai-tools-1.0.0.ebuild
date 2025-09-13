# Copyright 2024 RegicideOS
# Distributed under the terms of the GNU General Public License v3

EAPI=8

CRATES="
"

inherit cargo systemd

DESCRIPTION="AI-powered system management tools for RegicideOS"
HOMEPAGE="https://regicideos.com"
SRC_URI="https://github.com/awdemos/RegicideOS/archive/v${PV}.tar.gz -> ${P}.tar.gz"

LICENSE="GPL-3"
SLOT="0"
KEYWORDS="~amd64"
IUSE="+btrmind +portcl +systemd"

DEPEND="
	btrmind? (
		>=dev-lang/rust-1.75
		sys-fs/btrfs-progs
		systemd? ( sys-apps/systemd )
	)
	portcl? (
		>=dev-lang/rust-1.75
		sys-apps/portage
		systemd? ( sys-apps/systemd )
	)
"
RDEPEND="${DEPEND}"

QA_FLAGS_IGNORED=".*"

src_unpack() {
	default
	cargo_src_unpack
}

src_compile() {
	if use btrmind; then
		cd "${S}/ai-agents/btrmind" || die
		cargo_src_compile
	fi

	if use portcl; then
		# PortCL will be implemented in future
		:
	fi
}

src_install() {
	if use btrmind; then
		cd "${S}/ai-agents/btrmind" || die

		# Install binary
		dobin target/release/btrmind

		# Install configuration
		insinto /etc/btrmind
		doins config/btrmind.toml.example

		# Install systemd service
		use systemd && systemd_dounit systemd/btrmind.service

		# Install man page
		doman man/btrmind.1
	fi

	# Install common files
	dodoc README.md
	dodoc CONTRIBUTING.md
}

pkg_postinst() {
	if use btrmind; then
		elog "BtrMind AI agent has been installed."
		elog
		elog "To enable and start the service:"
		elog "  systemctl enable btrmind"
		elog "  systemctl start btrmind"
		elog
		elog "Configuration file: /etc/btrmind/config.toml"
		elog "Example configuration available at: /etc/btrmind/btrmind.toml.example"
		elog
		elog "Usage:"
		elog "  btrmind analyze    - Check current storage state"
		elog "  btrmind cleanup    - Run cleanup actions"
		elog "  btrmind stats      - Show learning statistics"
	fi

	if use portcl; then
		elog "PortCL AI agent is not yet implemented."
		elog "This USE flag is reserved for future development."
	fi
}