#!/usr/bin/env sh
set -e
CC="${CC:-cc}"
OUT="${1:-vortex-optim}"
"$CC" -O2 -std=c11 -Wall -Wextra -o "$OUT" optimizer.c
echo "built $OUT"
