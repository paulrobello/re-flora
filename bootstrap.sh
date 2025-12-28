#!/usr/bin/env bash
set -euo pipefail

if ! command -v cmake >/dev/null 2>&1; then
  echo "Warning: cmake is not available in PATH. Please install it manually before proceeding." >&2
fi

sudo apt update
sudo apt install -y libasound2-dev pkg-config
sudo apt update
sudo apt install -y build-essential clang libclang-dev llvm-dev libc6-dev
wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-1.4.313-noble.list https://packages.lunarg.com/vulkan/1.4.313/lunarg-vulkan-1.4.313-noble.list
sudo apt update
sudo apt install -y vulkan-sdk
cargo clean
