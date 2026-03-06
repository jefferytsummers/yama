#!/bin/bash
# Verify design tokens are synchronized between platforms
# Run from repo root: ./scripts/verify-design-tokens.sh

set -e

TOKENS_FILE="docs/brand/design-tokens.json"
CSS_FILE="web/src/app.css"
RUST_FILE="shared/yama-theme/src/lib.rs"

errors=0

echo "Verifying design token synchronization..."
echo ""

check_color() {
    local name="$1"
    local hex="$2"
    local rgb_r="$3"
    local rgb_g="$4"
    local rgb_b="$5"
    local name_lower=$(echo "$name" | tr '[:upper:]' '[:lower:]')

    # Check CSS
    if grep -q "${hex}" "$CSS_FILE"; then
        printf "  \033[0;32m✓\033[0m CSS: --color-%s = %s\n" "$name_lower" "$hex"
    else
        printf "  \033[0;31m✗\033[0m CSS: Missing --color-%s with value %s\n" "$name_lower" "$hex"
        errors=$((errors + 1))
    fi

    # Check Rust
    if grep -q "from_rgb(${rgb_r}, ${rgb_g}, ${rgb_b})" "$RUST_FILE"; then
        printf "  \033[0;32m✓\033[0m Rust: %s = Color32::from_rgb(%s, %s, %s)\n" "$name" "$rgb_r" "$rgb_g" "$rgb_b"
    else
        printf "  \033[0;31m✗\033[0m Rust: Missing %s with RGB(%s, %s, %s)\n" "$name" "$rgb_r" "$rgb_g" "$rgb_b"
        errors=$((errors + 1))
    fi
}

echo "Core Colors:"
check_color "OBSIDIAN" "#0D0D0F" 13 13 15
check_color "BASALT" "#16161A" 22 22 26
check_color "SLATE" "#1E1E24" 30 30 36
check_color "GRAPHITE" "#2A2A32" 42 42 50
check_color "STONE" "#3D3D47" 61 61 71

echo ""
echo "Accent Colors:"
check_color "AMBER" "#F59E0B" 245 158 11
check_color "EMBER" "#EF4444" 239 68 68
check_color "JADE" "#10B981" 16 185 129
check_color "AZURE" "#3B82F6" 59 130 246
check_color "VIOLET" "#8B5CF6" 139 92 246

echo ""
echo "Text Colors:"
check_color "CHALK" "#FAFAFA" 250 250 250
check_color "SILVER" "#A1A1AA" 161 161 170
check_color "ASH" "#71717A" 113 113 122

echo ""
if [ $errors -eq 0 ]; then
    printf "\033[0;32mAll color tokens are synchronized!\033[0m\n"
    exit 0
else
    printf "\033[0;31mFound %d synchronization issues.\033[0m\n" "$errors"
    echo "Update the mismatched files to match docs/brand/design-tokens.json"
    exit 1
fi
