#!/bin/sh
# gpu-env.sh — test the GPU selection inside srr-play without real hardware.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
SRR_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../srr-play"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# the host may export PRIME/Vulkan variables; the function must be tested clean
unset VK_ICD_FILENAMES VK_DRIVER_FILES DRI_PRIME __NV_PRIME_RENDER_OFFLOAD

# sysfs with an NVIDIA PCI device, one with only the nvidia module loaded,
# and one with neither
mkdir -p "$tmp/nvpci/bus/pci/devices/0000:01:00.0" "$tmp/nvmod/module/nvidia" \
  "$tmp/amdpci/bus/pci/devices/0000:0c:00.0" "$tmp/empty"
echo 0x10de > "$tmp/nvpci/bus/pci/devices/0000:01:00.0/vendor"
echo 0x1002 > "$tmp/amdpci/bus/pci/devices/0000:0c:00.0/vendor"

# 1. A hybrid host with an NVIDIA GPU, its ICD and another vendor's ICD pins
#    NVIDIA under both loader variable names, so DXVK cannot land on the
#    integrated GPU.
mkdir -p "$tmp/hybrid"
echo '{}' > "$tmp/hybrid/nvidia_icd.json"
echo '{}' > "$tmp/hybrid/radeon_icd.json"
want="VK_DRIVER_FILES=$tmp/hybrid/nvidia_icd.json VK_ICD_FILENAMES=$tmp/hybrid/nvidia_icd.json"
out=$(SRR_ICD_DIR="$tmp/hybrid" SRR_SYSFS="$tmp/nvpci" gpu_env)
[ "$out" = "$want" ] || { echo "FAIL: hybrid host did not pin the NVIDIA ICD: $out"; exit 1; }
out=$(SRR_ICD_DIR="$tmp/hybrid" SRR_SYSFS="$tmp/nvmod" gpu_env)
[ "$out" = "$want" ] || { echo "FAIL: loaded nvidia module did not count: $out"; exit 1; }

# 2. A leftover NVIDIA ICD without any NVIDIA device is never pinned.
out=$(SRR_ICD_DIR="$tmp/hybrid" SRR_SYSFS="$tmp/amdpci" gpu_env)
[ -z "$out" ] || { echo "FAIL: pinned NVIDIA without an NVIDIA GPU: $out"; exit 1; }

# 3. ICDs in /etc/vulkan/icd.d count as well as /usr/share (several dirs);
#    every NVIDIA json found is listed.
mkdir -p "$tmp/etc" "$tmp/usr"
echo '{}' > "$tmp/etc/nvidia_icd.json"
echo '{}' > "$tmp/usr/intel_icd.x86_64.json"
out=$(SRR_ICD_DIR="$tmp/etc $tmp/usr" SRR_SYSFS="$tmp/nvpci" gpu_env)
[ "$out" = "VK_DRIVER_FILES=$tmp/etc/nvidia_icd.json VK_ICD_FILENAMES=$tmp/etc/nvidia_icd.json" ] ||
  { echo "FAIL: /etc ICD not considered: $out"; exit 1; }
echo '{}' > "$tmp/usr/nvidia_icd.i686.json"
out=$(SRR_ICD_DIR="$tmp/etc $tmp/usr" SRR_SYSFS="$tmp/nvpci" gpu_env)
case "$out" in
  "VK_DRIVER_FILES=$tmp/etc/nvidia_icd.json:$tmp/usr/nvidia_icd.i686.json "*) ;;
  *) echo "FAIL: every NVIDIA json should be listed: $out"; exit 1 ;;
esac

# 4. A driver choice the caller made wins, under either variable name.
for preset in VK_ICD_FILENAMES VK_DRIVER_FILES DRI_PRIME __NV_PRIME_RENDER_OFFLOAD; do
  out=$(env "$preset=already" SRR_ICD_DIR="$tmp/hybrid" SRR_SYSFS="$tmp/nvpci" \
    sh -c 'SRR_PLAY_FUNCTIONS_ONLY=1 . "$1"; gpu_env' sh "$HERE/../srr-play")
  [ -z "$out" ] || { echo "FAIL: preset $preset should win: $out"; exit 1; }
done

# 5. A single-vendor host leaves the environment alone (DXVK default).
mkdir -p "$tmp/single"
echo '{}' > "$tmp/single/intel_icd.json"
out=$(SRR_ICD_DIR="$tmp/single" SRR_SYSFS="$tmp/empty" gpu_env)
[ -z "$out" ] || { echo "FAIL: single ICD should not pin: $out"; exit 1; }

# 6. Intel/AMD hybrid without an NVIDIA ICD falls back to the sysfs PRIME
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
[ "$out" = DRI_PRIME=1 ] || { echo "FAIL: iGPU boot + dGPU vram should pick DRI_PRIME=1: $out"; exit 1; }

# 7. AMD APU boot card with a bigger AMD dGPU: VRAM decides (mem_info_vram_total).
drm=$tmp/apu/class/drm
mkdir -p "$drm/card0/device" "$drm/card1/device"
echo 0x1002 > "$drm/card0/device/vendor"; echo 1 > "$drm/card0/device/boot_vga"
echo 512 > "$drm/card0/device/mem_info_vram_total"
echo 0x1002 > "$drm/card1/device/vendor"; echo 0 > "$drm/card1/device/boot_vga"
echo 8192 > "$drm/card1/device/mem_info_vram_total"
out=$(SRR_ICD_DIR="$tmp/prime-icd" SRR_SYSFS="$tmp/apu" gpu_env)
[ "$out" = DRI_PRIME=1 ] || { echo "FAIL: APU + bigger dGPU should pick DRI_PRIME=1: $out"; exit 1; }

echo "gpu-env: ok"
