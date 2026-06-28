#!/usr/bin/env sh
set -e
CC="${CC:-cc}"
INC="${VULKAN_INCLUDE:-/usr/include}"
OUT="libVkLayer_vortstrap_present_mode.so"
if [ ! -f "$INC/vulkan/vk_layer.h" ]; then
  echo "error: vulkan/vk_layer.h not found under $INC" >&2
  exit 1
fi
"$CC" -I"$INC" -shared -fPIC -O2 -fvisibility=hidden -Wall -Wextra \
  -o "$OUT" present_mode_layer.c
echo "built $OUT"
