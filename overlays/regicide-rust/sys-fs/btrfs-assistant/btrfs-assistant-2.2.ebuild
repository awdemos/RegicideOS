# Copyright 2026 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit cmake xdg

DESCRIPTION="GUI management tool for Btrfs filesystems and snapshots"
HOMEPAGE="https://gitlab.com/btrfs-assistant/btrfs-assistant"
SRC_URI="https://gitlab.com/btrfs-assistant/btrfs-assistant/-/archive/${PV}/btrfs-assistant-${PV}.tar.bz2"

LICENSE="GPL-3"
SLOT="0"
KEYWORDS="~amd64"

# Qt6 Widgets + LinguistTools (translations); qtsvg for icons.
DEPEND="
	dev-qt/qtbase:6[gui,widgets]
	dev-qt/qtsvg:6
"
RDEPEND="
	${DEPEND}
	sys-fs/btrfs-progs
	sys-auth/polkit
"
BDEPEND="
	dev-qt/qttools:6[linguist]
"
