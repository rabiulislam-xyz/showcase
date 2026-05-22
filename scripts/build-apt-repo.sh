#!/usr/bin/env bash
# Build a signed, single-distribution (stable/main, amd64) apt repository
# from a directory of .deb files.
#
# Usage: build-apt-repo.sh <deb_dir> <out_dir>
#
# Assumes the signing secret key is already imported into the gpg keyring of
# the calling environment; this script only signs with the default key.
#
# Produced layout (rooted at <out_dir>):
#   pool/main/<*.deb>
#   dists/stable/main/binary-amd64/Packages
#   dists/stable/main/binary-amd64/Packages.gz
#   dists/stable/Release
#   dists/stable/Release.gpg        (detached signature of Release)
#   dists/stable/InRelease          (inline-signed Release)
#   showcase-archive-keyring.gpg    (dearmored public key for /etc/apt/keyrings)
set -euo pipefail

ORIGIN="Showcase"

usage() {
  echo "Usage: $0 <deb_dir> <out_dir>" >&2
  exit 2
}

[ "$#" -eq 2 ] || usage

DEB_DIR="$1"
OUT_DIR="$2"

[ -d "$DEB_DIR" ] || { echo "error: deb_dir '$DEB_DIR' is not a directory" >&2; exit 1; }

# Require the tools we depend on.
for tool in apt-ftparchive gpg; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "error: required tool '$tool' not found (install apt-utils and gnupg)" >&2
    exit 1
  }
done

# Resolve to absolute paths before we start changing directories.
DEB_DIR="$(cd "$DEB_DIR" && pwd)"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

# Idempotent: rebuild the generated tree from scratch each run.
rm -rf "$OUT_DIR/pool" "$OUT_DIR/dists"
mkdir -p "$OUT_DIR/pool/main"
mkdir -p "$OUT_DIR/dists/stable/main/binary-amd64"

# Copy the .deb files into the pool.
deb_count=0
for deb in "$DEB_DIR"/*.deb; do
  [ -e "$deb" ] || { echo "error: no .deb files found in '$DEB_DIR'" >&2; exit 1; }
  cp -f "$deb" "$OUT_DIR/pool/main/"
  deb_count=$((deb_count + 1))
done

# apt-ftparchive expects to be run from the repo root so that the File: paths
# recorded in Packages are relative (pool/main/...).
cd "$OUT_DIR"

# Generate Packages + Packages.gz for the binary-amd64 component.
apt-ftparchive packages pool/main > dists/stable/main/binary-amd64/Packages
gzip -9 -c dists/stable/main/binary-amd64/Packages > dists/stable/main/binary-amd64/Packages.gz

# Generate the Release index for the suite.
apt-ftparchive release dists/stable \
  -o "APT::FTPArchive::Release::Origin=${ORIGIN}" \
  -o "APT::FTPArchive::Release::Label=${ORIGIN}" \
  -o "APT::FTPArchive::Release::Suite=stable" \
  -o "APT::FTPArchive::Release::Codename=stable" \
  -o "APT::FTPArchive::Release::Components=main" \
  -o "APT::FTPArchive::Release::Architectures=amd64" \
  > dists/stable/Release

# Sign the Release file with the default secret key.
# InRelease is the inline-signed variant; Release.gpg is the detached signature.
rm -f dists/stable/InRelease dists/stable/Release.gpg
gpg --batch --yes --clearsign --output dists/stable/InRelease dists/stable/Release
gpg --batch --yes -abs --output dists/stable/Release.gpg dists/stable/Release

# Export the public key (dearmored binary) for clients to drop in
# /etc/apt/keyrings.
gpg --export > "$OUT_DIR/showcase-archive-keyring.gpg"

echo "apt repo built at: $OUT_DIR"
echo "  packages:        $deb_count .deb in pool/main/"
echo "  release:         dists/stable/Release (+ InRelease, Release.gpg)"
echo "  public key:      showcase-archive-keyring.gpg"
