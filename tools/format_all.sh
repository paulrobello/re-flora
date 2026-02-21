#!/usr/bin/env bash

# ============================================================================
# Clang-Format Bash Script
#
# This script recursively finds and formats files with specific extensions
# in the current directory and all subdirectories. It uses clang-format
# and respects the settings in your .clang-format file.
#
# Prerequisites:
# 1. clang-format must be installed and accessible in your system's PATH.
# 2. A .clang-format file should be present in the root directory where you
#    run this script, or in any parent directory.
#
# Configuration:
# - Edit the FILE_EXTENSIONS array below to change or add file types.
# ============================================================================

echo
echo "Starting formatting rust code"
cargo fmt

# --- CONFIGURATION ---
FILE_EXTENSIONS=("*.vert" "*.frag" "*.comp" "*.glsl")

# --- SCRIPT LOGIC ---
echo
echo "Starting formatting shaders"
echo "Searching for files with extensions: ${FILE_EXTENSIONS[*]}"
echo

if ! command -v clang-format >/dev/null 2>&1; then
    echo "ERROR: clang-format not found in your system's PATH."
    echo "Please install LLVM and ensure clang-format is in your PATH."
    exit 1
fi

files=()
if command -v rg >/dev/null 2>&1; then
    mapfile -t files < <(rg --files -g "${FILE_EXTENSIONS[0]}" -g "${FILE_EXTENSIONS[1]}" \
        -g "${FILE_EXTENSIONS[2]}" -g "${FILE_EXTENSIONS[3]}" || true)
else
    mapfile -t files < <(find . -type f \( -name "${FILE_EXTENSIONS[0]}" \
        -o -name "${FILE_EXTENSIONS[1]}" -o -name "${FILE_EXTENSIONS[2]}" \
        -o -name "${FILE_EXTENSIONS[3]}" \))
fi

total_files=${#files[@]}
echo "Found ${total_files} files."

current_file=0
for file in "${files[@]}"; do
    current_file=$((current_file + 1))
    echo "${current_file}/${total_files} done"
    clang-format -i --style=file "$file"
done

echo
echo "================================="
echo "Formatting process complete."
echo "================================="
echo
