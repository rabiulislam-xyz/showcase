#!/usr/bin/env bash
# One-time system build dependencies for Tauri v2 on Ubuntu 22.04+.
set -euo pipefail
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  pkg-config
echo "Done. cargo/build-essential/libssl-dev are assumed already present."
