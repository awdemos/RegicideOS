# Copyright 2024 RegicideOS Team
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit cargo systemd

DESCRIPTION="RegicideOS AI-powered system management tools"
HOMEPAGE="https://github.com/awdemos/RegicideOS"

if [[ ${PV} == *9999 ]]; then
	EGIT_REPO_URI="https://github.com/awdemos/RegicideOS.git"
	inherit git-r3
	S="${WORKDIR}/${P}/ai-agents"
else
	SRC_URI="https://github.com/awdemos/RegicideOS/archive/v${PV}.tar.gz -> ${P}.tar.gz"
	S="${WORKDIR}/RegicideOS-${PV}/ai-agents"
	KEYWORDS="amd64 ~arm64"
fi

LICENSE="GPL-3"
SLOT="0"
IUSE="btrmind portcl systemd"
REQUIRED_USE="|| ( btrmind portcl )"

# Runtime dependencies
RDEPEND="
	>=virtual/rust-1.70.0
	systemd? ( sys-apps/systemd )
	btrmind? ( 
		sys-fs/btrfs-progs
		sys-process/lsof
	)
	portcl? (
		sys-apps/portage
		app-portage/gentoolkit
	)
"

BDEPEND="
	>=virtual/rust-1.70.0
	>=virtual/pkgconfig-2
"

DEPEND="${RDEPEND}"

src_unpack() {
	if [[ ${PV} == *9999 ]]; then
		git-r3_src_unpack
	else
		default
	fi
}

src_compile() {
	# Build each AI agent separately
	if use btrmind; then
		einfo "Building BtrMind storage AI agent..."
		cd "${S}/btrmind" || die
		cargo_src_compile
		cd "${S}" || die
	fi
	
	if use portcl; then
		einfo "Building PortCL package management AI agent..."
		# TODO: Implement when PortCL is ready
		ewarn "PortCL is not yet implemented"
	fi
}

src_test() {
	if use btrmind; then
		einfo "Testing BtrMind..."
		cd "${S}/btrmind" || die
		cargo_src_test
		cd "${S}" || die
	fi
}

src_install() {
	# Install BtrMind
	if use btrmind; then
		einfo "Installing BtrMind..."
		cd "${S}/btrmind" || die
		
		# Install binary
		dobin target/release/btrmind
		
		# Install configuration
		insinto /etc/btrmind
		doins config/btrmind.toml
		
		# Install systemd service
		if use systemd; then
			systemd_dounit systemd/btrmind.service
			systemd_install_serviced btrmind.service
		fi
		
		# Create directories
		diropts -m0755 -o btrmind -g btrmind
		keepdir /var/lib/btrmind
		keepdir /var/log/btrmind
		
		# Install documentation
		dodoc README.md
		
		cd "${S}" || die
	fi
	
	# Install common AI framework files
	insinto /usr/share/regicide
	doins -r "${FILESDIR}/examples" 2>/dev/null || true
}

pkg_preinst() {
	# Create btrmind user if it doesn't exist
	if use btrmind; then
		enewgroup btrmind
		enewuser btrmind -1 -1 /var/lib/btrmind btrmind
	fi
}

pkg_postinst() {
	elog "RegicideOS AI Tools ${PV} installed successfully!"
	elog ""
	
	if use btrmind; then
		elog "BtrMind AI Storage Agent:"
		elog "  Config: /etc/btrmind/config.toml"
		elog "  Enable: systemctl enable btrmind"
		elog "  Start:  systemctl start btrmind"
		elog "  Test:   btrmind analyze"
		elog ""
	fi
	
	if use portcl; then
		elog "PortCL Package Management AI (Future):"
		elog "  Will provide intelligent package optimization"
		elog ""
	fi
	
	elog "Documentation:"
	elog "  Handbook: /usr/share/doc/${PF}/README.md"
	elog "  Online: https://github.com/awdemos/RegicideOS/blob/main/Handbook.md"
	elog ""
	elog "The AI agents will learn and improve system performance over time."
	elog "Monitor progress with 'journalctl -u btrmind -f' and 'btrmind stats'"
}

pkg_prerm() {
	# Stop services before removal
	if use systemd; then
		if use btrmind && systemctl is-active btrmind >/dev/null 2>&1; then
			einfo "Stopping BtrMind service..."
			systemctl stop btrmind
		fi
	fi
}
