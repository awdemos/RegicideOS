#!/bin/bash
# Stage 7b: generate an SBOM from the Portage package database and enforce
# architecture gates (package count, COSMIC presence).
set -euo pipefail

source "$(dirname "$0")/common.sh"
STAGE_NAME="stage7-sbom"

TARBALL="${OUTPUT_DIR}/stage4-amd64-systemd-cosmic.tar.xz"
ROOTS_DIR="$(mktemp -d -t regicide-sbom-XXXXXX)"
LEGACY_SBOM_FILE="${OUTPUT_DIR}/sbom.json"
SPDX_FILE="${OUTPUT_DIR}/sbom.spdx.json"

trap 'rm -rf "${ROOTS_DIR}"' EXIT

log_status "start" "generating SBOM and running package gates"
echo "Stage 7b: SBOM and package gates..."

if [[ ! -f "${TARBALL}" ]]; then
    echo "ERROR: stage4 tarball missing: ${TARBALL}"
    exit 1
fi

echo "Extracting Portage database from tarball..."
tar -C "${ROOTS_DIR}" -xpJf "${TARBALL}" ./var/db/pkg 2>/dev/null || true

if [[ ! -d "${ROOTS_DIR}/var/db/pkg" ]]; then
    echo "ERROR: /var/db/pkg not found in tarball"
    exit 1
fi

PKG_COUNT=0
COSMIC_PKGS=0
PKG_JSON_ENTRIES=()

# Read package database once; data is reused for both SBOM formats.
while IFS= read -r -d '' pkg_dir; do
    category="$(basename "$(dirname "${pkg_dir}")")"
    name_version="$(basename "${pkg_dir}")"
    PKG_COUNT=$((PKG_COUNT + 1))
    if [[ "${category}" == cosmic-* || "${name_version}" == cosmic-* ]]; then
        COSMIC_PKGS=$((COSMIC_PKGS + 1))
    fi
    PKG_JSON_ENTRIES+=("{ \"category\": \"${category}\", \"package\": \"${name_version}\" }")
done < <(find "${ROOTS_DIR}/var/db/pkg" -mindepth 2 -maxdepth 2 -type d -print0 | sort -z)

# Legacy simple JSON SBOM (kept for backwards compatibility).
{
    echo "{"
    echo "  \"generator\": \"RegicideOS stage7-sbom\","
    echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"packages\": ["
    FIRST=true
    for entry in "${PKG_JSON_ENTRIES[@]}"; do
        if [[ "${FIRST}" == true ]]; then
            FIRST=false
        else
            echo ","
        fi
        printf '    %s' "${entry}"
    done
    echo ""
    echo "  ],"
    echo "  \"package_count\": ${PKG_COUNT},"
    echo "  \"cosmic_package_count\": ${COSMIC_PKGS}"
    echo "}"
} > "${LEGACY_SBOM_FILE}"

echo "Legacy SBOM written to ${LEGACY_SBOM_FILE}"

# SPDX-2.3 JSON SBOM generated from the same Portage package data.
SPDX_ID_NS="SPDXRef-Package"
DOCUMENT_NAMESPACE="https://regicideos.dev/spbom/stage4-amd64-systemd-cosmic-$(date -u +%Y%m%d%H%M%S)"
{
    echo "{"
    echo "  \"spdxVersion\": \"SPDX-2.3\","
    echo "  \"dataLicense\": \"CC0-1.0\","
    echo "  \"SPDXID\": \"SPDXRef-DOCUMENT\","
    echo "  \"name\": \"RegicideOS-stage4-amd64-systemd-cosmic\","
    echo "  \"documentNamespace\": \"${DOCUMENT_NAMESPACE}\","
    echo "  \"creationInfo\": {"
    echo "    \"created\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "    \"creators\": [\"Tool: RegicideOS stage7-sbom\"]"
    echo "  },"
    echo "  \"packages\": ["
    FIRST=true
    for entry in "${PKG_JSON_ENTRIES[@]}"; do
        if [[ "${FIRST}" == true ]]; then
            FIRST=false
        else
            echo ","
        fi
        category="$(echo "${entry}" | sed -n 's/.*\"category\": \"\([^\"]*\)\".*/\1/p')"
        name_version="$(echo "${entry}" | sed -n 's/.*\"package\": \"\([^\"]*\)\".*/\1/p')"
        # Portage directory names are "name-version[-revision]"; the version
        # starts at the last hyphen followed by a digit.
        name="${name_version}"
        version=""
        if [[ "${name_version}" =~ ^(.*)-([0-9].*)$ ]]; then
            name="${BASH_REMATCH[1]}"
            version="${BASH_REMATCH[2]}"
        fi
        spdx_id="${SPDX_ID_NS}-${category}-${name_version}"
        printf '    {\n'
        printf '      \"SPDXID\": \"%s\",\n' "${spdx_id}"
        printf '      \"name\": \"%s\",\n' "${name}"
        printf '      \"versionInfo\": \"%s\",\n' "${version}"
        printf '      \"downloadLocation\": \"NOASSERTION\",\n'
        printf '      \"supplier\": \"NOASSERTION\",\n'
        printf '      \"licenseConcluded\": \"NOASSERTION\",\n'
        printf '      \"licenseDeclared\": \"NOASSERTION\",\n'
        printf '      \"copyrightText\": \"NOASSERTION\",\n'
        printf '      \"externalRefs\": [\n'
        printf '        {\n'
        printf '          \"referenceCategory\": \"PACKAGE-MANAGER\",\n'
        printf '          \"referenceType\": \"purl\",\n'
        printf '          \"referenceLocator\": \"pkg:ebuild/%s/%s@%s\"\n' "${category}" "${name}" "${version}"
        printf '        }\n'
        printf '      ]\n'
        printf '    }'
    done
    echo ""
    echo "  ],"
    echo "  \"relationships\": ["
    echo "    { \"spdxElementId\": \"SPDXRef-DOCUMENT\", \"relatedSpdxElement\": \"SPDXRef-DOCUMENT\", \"relationshipType\": \"DESCRIBES\" }"
    echo "  ]"
    echo "}"
} > "${SPDX_FILE}"

echo "SPDX SBOM written to ${SPDX_FILE}"
echo "Total packages: ${PKG_COUNT}"
echo "COSMIC packages: ${COSMIC_PKGS}"

ERRORS=0

# Gate: COSMIC greeter must be installed.
if find "${ROOTS_DIR}/var/db/pkg" -mindepth 2 -maxdepth 2 -type d \( -name 'cosmic-greeter' -o -name 'cosmic-greeter-*' \) | grep -q .; then
    echo "PASS: cosmic-greeter installed"
else
    echo "FAIL: cosmic-greeter not found in package database"
    ERRORS=$((ERRORS + 1))
fi

# Gate: package count sanity. The white paper claims "slightly over 300 packages".
# Allow a generous upper bound for now, but warn if it grows much larger.
if [[ ${PKG_COUNT} -gt 1200 ]]; then
    echo "FAIL: package count ${PKG_COUNT} exceeds 1200; investigate bloat"
    ERRORS=$((ERRORS + 1))
else
    echo "PASS: package count ${PKG_COUNT} within acceptable range"
fi

if [[ ${ERRORS} -gt 0 ]]; then
    echo ""
    echo "Stage 7b SBOM gates FAILED with ${ERRORS} error(s)."
    log_status "failed" "${ERRORS} SBOM gate errors"
    exit 1
fi

echo ""
echo "Stage 7b SBOM gates PASSED."
log_status "complete" "SBOM generated and gates passed"
