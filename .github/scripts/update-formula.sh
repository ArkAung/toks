#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# Usage:  ./update-formula.sh <version>
#   <version> is the tag without the leading "v", e.g. 0.2.0
# ------------------------------------------------------------
VERSION="${1}"
REPO="${GITHUB_REPOSITORY:-arkaung/toks}"   # fallback for local testing

# 1️⃣ Download the source tarball (needed for its SHA256)
TARBALL="toks-${VERSION}.tar.gz"
curl -LfsS -o "${TARBALL}" \
  "https://github.com/${REPO}/archive/refs/tags/v${VERSION}.tar.gz"
SOURCE_SHA256=$(shasum -a 256 "${TARBALL}" | awk '{print $1}')
rm -f "${TARBALL}"

# 2️⃣ Define the three bottle URLs we will upload later
#    (the workflow uploads them as assets with these exact names)
BOTTLE_X86_64="toks-${VERSION}.x86_64.tar.gz"
BOTTLE_ARM64="toks-${VERSION}.arm64.tar.gz"
BOTTLE_LINUX="toks-${VERSION}.x86_64_linux.tar.gz"

# 3️⃣ Compute SHA256 of the bottles that will be present as assets
#    (we compute them from the files that will be in the workspace)
if [[ -f "${BOTTLE_X86_64}" ]]; then
  INTEL_SHA256=$(shasum -a 256 "${BOTTLE_X86_64}" | awk '{print $1}')
else
  INTEL_SHA256="PLACEHOLDER_INTEL_SHA256"
fi

if [[ -f "${BOTTLE_ARM64}" ]]; then
  ARM64_SHA256=$(shasum -a 256 "${BOTTLE_ARM64}" | awk '{print $1}')
else
  ARM64_SHA256="PLACEHOLDER_ARM64_SHA256"
fi

if [[ -f "${BOTTLE_LINUX}" ]]; then
  LINUX_SHA256=$(shasum -a 256 "${BOTTLE_LINUX}" | awk '{print $1}')
else
  LINUX_SHA256="PLACEHOLDER_LINUX_SHA256"
fi

# 4️⃣ Replace the four lines in the formula
#    We use sed with a temporary file to stay portable (macOS/ Linux)
FORMULA_PATH="Formula/toks.rb"

# Replace source URL & SHA256
sed -i.bak -e "s|url \".*\"|url \"https://github.com/${REPO}/archive/refs/tags/v${VERSION}.tar.gz\"|" \
           -e "s|sha256 \".*\"|sha256 \"${SOURCE_SHA256}\"|" \
           "${FORMULA_PATH}"

# Replace bottle SHA256s
sed -i.bak -e "s|sha256 cellar: :any_skip_relocation, arm64_ventura:.*|sha256 cellar: :any_skip_relocation, arm64_ventura:   \"${ARM64_SHA256}\"|" \
           -e "s|sha256 cellar: :any_skip_relocation, x86_64_ventura:.*|sha256 cellar: :any_skip_relocation, x86_64_ventura:   \"${INTEL_SHA256}\"|" \
           -e "s|sha256 cellar: :any_skip_relocation, x86_64_linux:.*|sha256 cellar: :any_skip_relocation, x86_64_linux:    \"${LINUX_SHA256}\"|" \
           "${FORMULA_PATH}"

# Remove backup files created by sed on macOS
rm -f "${FORMULA_PATH}.bak"

echo "✅ Formula updated for version ${VERSION}"