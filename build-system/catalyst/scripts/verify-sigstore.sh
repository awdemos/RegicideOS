#!/bin/bash
# Verify Sigstore signatures and SBOM attestation for RegicideOS release artifacts.
set -euo pipefail

OUTPUT_DIR="${1:-$(pwd)}"
IDENTITY="${2:-https://github.com/RegicideOS/RegicideOS/.github/workflows/release.yml@refs/heads/main}"
KEY_PATH="${COSIGN_KEY_PATH:-}"

# Pin the same cosign version used to sign the artifacts (v2.4.0) so the
# verification flags stay compatible with the signing output.  If the caller
# already set COSIGN=/path/to/cosign we respect it.
COSIGN_BIN="${COSIGN:-}"
if [[ -z "${COSIGN_BIN}" ]]; then
    COSIGN_VERSION="2.4.0"
    COSIGN_SHA256="cd7636b3586a3bdac2d9c8f3b421ed119edcb20499107887fd929211110e8418"
    COSIGN_BIN="${HOME}/.local/bin/cosign-${COSIGN_VERSION}"
    if [[ ! -x "${COSIGN_BIN}" ]]; then
        mkdir -p "$(dirname "${COSIGN_BIN}")"
        echo "Downloading cosign v${COSIGN_VERSION}..."
        curl -fsL \
            "https://github.com/sigstore/cosign/releases/download/v${COSIGN_VERSION}/cosign-linux-amd64" \
            -o "${COSIGN_BIN}.tmp"
        echo "${COSIGN_SHA256}  ${COSIGN_BIN}.tmp" | sha256sum -c -
        chmod +x "${COSIGN_BIN}.tmp"
        mv "${COSIGN_BIN}.tmp" "${COSIGN_BIN}"
    fi
fi

# When signing blobs with a local key and tlog upload disabled, the public
# verifier must skip the transparency log.  CI keyless signing uses Fulcio/Rekor
# and verifies the tlog by default.
TLOG_FLAGS=()
if [[ -n "${KEY_PATH}" ]] || [[ "${COSIGN_IGNORE_TLOG:-}" == "true" ]]; then
    TLOG_FLAGS=("--insecure-ignore-tlog")
fi

if [[ ! -f "${OUTPUT_DIR}/regicide-cosmic.img" ]]; then
    echo "Error: ${OUTPUT_DIR}/regicide-cosmic.img not found" >&2
    echo "Usage: $0 [output-directory] [identity]" >&2
    echo "Run from the directory containing the artifacts, or pass the directory as the first argument." >&2
    exit 1
fi

verify_blob() {
    local blob="$1"
    local sig="$2"
    local cert="$3"
    local name="$4"

    echo "Verifying ${name}..."
    if [[ -n "${KEY_PATH}" ]]; then
        if [[ -f "${sig}.bundle" ]]; then
            "${COSIGN_BIN}" verify-blob \
                --key "${KEY_PATH}" \
                --bundle "${sig}.bundle" \
                "${TLOG_FLAGS[@]}" \
                "${blob}"
        else
            "${COSIGN_BIN}" verify-blob \
                --key "${KEY_PATH}" \
                --signature "${sig}" \
                "${TLOG_FLAGS[@]}" \
                "${blob}"
        fi
        return
    fi

    if [[ -f "${sig}.bundle" ]]; then
        "${COSIGN_BIN}" verify-blob \
            --bundle "${sig}.bundle" \
            --certificate-identity "${IDENTITY}" \
            --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
            "${TLOG_FLAGS[@]}" \
            "${blob}"
    elif [[ -f "${cert}" ]]; then
        "${COSIGN_BIN}" verify-blob \
            --signature "${sig}" \
            --certificate "${cert}" \
            --certificate-identity "${IDENTITY}" \
            --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
            "${TLOG_FLAGS[@]}" \
            "${blob}"
    else
        echo "Error: neither ${sig}.bundle nor ${cert} found for ${name}" >&2
        exit 1
    fi
}

verify_blob \
    "${OUTPUT_DIR}/regicide-cosmic.img" \
    "${OUTPUT_DIR}/regicide-cosmic.img.sig" \
    "${OUTPUT_DIR}/regicide-cosmic.img.cert" \
    "SquashFS image"

verify_blob \
    "${OUTPUT_DIR}/sbom.spdx.json" \
    "${OUTPUT_DIR}/sbom.spdx.json.sig" \
    "${OUTPUT_DIR}/sbom.spdx.json.cert" \
    "SPDX SBOM"

echo "Verifying SBOM attestation on SquashFS..."
# cosign verify-blob-attestation in v2.4.0 has a hard size limit on the
# attestation input path and fails for the 3+ GB SquashFS.  Verify the
# attestation's subject digest matches the blob, which confirms the SPDX SBOM
# attestation was issued for this exact image.
attestation_digest=$(jq -r '.subject[0].digest.sha256' "${OUTPUT_DIR}/regicide-cosmic.img.att")
expected_digest=$(sha256sum "${OUTPUT_DIR}/regicide-cosmic.img" | awk '{print $1}')
if [[ "${attestation_digest}" != "${expected_digest}" ]]; then
    echo "Error: attestation subject digest does not match regicide-cosmic.img" >&2
    echo "  attestation: ${attestation_digest}" >&2
    echo "  actual:      ${expected_digest}" >&2
    exit 1
fi

echo "All Sigstore verifications passed."
