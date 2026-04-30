# Copyright 2024 RegicideOS
# Distributed under the terms of the GNU General Public License v3

EAPI=8

CRATES="
	anyhow-1.0.75
	candle-core-0.6.0
	candle-nn-0.6.0
	candle-transformers-0.6.0
	serde-1.0.192
	serde_json-1.0.108
	tensor-0.8.0
	tch-0.15.0
	ndarray-0.15.6
	polars-0.38.0
"

inherit cargo

DESCRIPTION="Rust machine learning inference engine optimized for RegicideOS"
HOMEPAGE="https://github.com/huggingface/candle"
SRC_URI="https://github.com/huggingface/candle/archive/refs/tags/${PV}.tar.gz -> ${P}.tar.gz"

LICENSE="|| ( MIT Apache-2.0 )"
SLOT="0"
KEYWORDS="~amd64"
IUSE="+cuda +metal +opencl +vulkan"

DEPEND="
	>=dev-lang/rust-1.75
	cuda? ( dev-util/nvidia-cuda-toolkit )
	metal? ( virtual/apple-sdk-framework-metal )
	opencl? ( virtual/opencl )
	vulkan? ( media-libs/vulkan-loader )
"
RDEPEND="${DEPEND}"

QA_FLAGS_IGNORED=".*"

src_unpack() {
	default
	cargo_src_unpack
}

src_compile() {
	cd "${S}/candle-core" || die

	local features=()
	use cuda && features+=(cuda)
	use metal && features+=(metal)
	use opencl && features+=(opencl)
	use vulkan && features+=(vulkan)

	if [[ ${#features[@]} -eq 0 ]]; then
		features=(cpu)
	fi

	CARGO_FEATURES="${features[*]}" cargo_src_compile
}

src_install() {
	cd "${S}/candle-core" || die

	# Install candle libraries
	dolib.so target/release/deps/libcandle_core.so
	dolib.so target/release/deps/libcandle_nn.so
	dolib.so target/release/deps/libcandle_transformers.so

	# Install headers
	insinto /usr/include/candle
	doins candle-core/include/*.h

	# Install utilities
	dobin target/release/candle-examples

	# Install Python bindings if available
	if [[ -d "${S}/python" ]]; then
		cd "${S}/python" || die
		python_domodule candle
	fi

	# Install documentation
	dodoc README.md
	dodoc CONTRIBUTING.md

	# Install examples
	insinto /usr/share/candle/examples
	doins examples/*.rs
}

pkg_postinst() {
	elog "Candle ML inference engine has been installed."
	elog
	elog "This provides fast, memory-efficient ML inference in Rust."
	elog
	elog "Features enabled:"
	elog "  CPU inference: always available"
	if use cuda; then
		elog "  CUDA support: enabled"
	fi
	if use metal; then
		elog "  Metal support: enabled"
	fi
	if use opencl; then
		elog "  OpenCL support: enabled"
	fi
	if use vulkan; then
		elog "  Vulkan support: enabled"
	fi
	elog
	elog "Example usage:"
	elog "  candle-examples --model path/to/model.onnx"
	elog
	elog "For more information, see:"
	elog "  https://github.com/huggingface/candle"
	elog "  /usr/share/candle/examples/"
}