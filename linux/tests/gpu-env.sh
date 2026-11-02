#!/bin/sh
# gpu-env.sh — test the GPU selection inside srr-play without real hardware.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# the host may export PRIME/Vulkan variables; the function must be tested clean
unset VK_ICD_FILENAMES DRI_PRIME __NV_PRIME_RENDER_OFFLOAD

# 1. A hybrid host with an NVIDIA ICD and another vendor pins NVIDIA, so
#    DXVK cannot land on the integrated GPU.
mkdir -p "$tmp/hybrid"
echo '{}' > "$tmp/hybrid/nvidia_icd.json"
echo '{}' > "$tmp/hybrid/radeon_icd.json"
out=$(SRR_ICD_DIR="$tmp/hybrid" gpu_env)
case "$out" in
  VK_ICD_FILENAMES=*nvidia_icd.json) ;;
  *) echo "FAIL: hybrid host did not pin the NVIDIA ICD: $out"; exit 1 ;;
esac

# 2. A single-vendor host leaves the environment alone (DXVK default).
out=$(SRR_ICD_DIR="$tmp/hybrid" VK_ICD_FILENAMES=already gpu_env)
[ -z "$out" ] || { echo "FAIL: preset env should win: $out"; exit 1; }
rmdir "$tmp/single" 2>/dev/null || true
mkdir -p "$tmp/single" "$tmp/empty"
echo '{}' > "$tmp/single/intel_icd.json"
out=$(SRR_ICD_DIR="$tmp/single" SRR_SYSFS="$tmp/empty" gpu_env)
[ -z "$out" ] || { echo "FAIL: single ICD should not pin: $out"; exit 1; }

# 3. Intel/AMD hybrid without an NVIDIA ICD falls back to the sysfs PRIME
#    heuristic: an iGPU boot card plus a bigger-VRAM discrete card.
drm=$tmp/prime/class/drm
mkdir -p "$drm/card0/device" "$drm/card1/device" "$drm/card0-DP-1/device"
echo 0x8086 > "$drm/card0/device/vendor"
echo 1 > "$drm/card0/device/boot_vga"
echo 512 > "$drm/card0/device/mem_info_vram_total"
echo 0x1002 > "$drm/card1/device/vendor"
echo 0 > "$drm/card1/device/boot_vga"
echo 16384 > "$drm/card1/device/mem_info_vram_total"
mkdir -p "$tmp/prime-icd"
echo '{}' > "$tmp/prime-icd/radeon_icd.json"
out=$(SRR_ICD_DIR="$tmp/prime-icd" SRR_SYSFS="$tmp/prime" gpu_env)
case "$out" in
  DRI_PRIME=1) ;;
  *) echo "FAIL: iGPU boot + dGPU vram should pick DRI_PRIME=1: $out"; exit 1 ;;
esac

echo "gpu-env: ok"
