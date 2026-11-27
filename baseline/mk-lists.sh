#!/bin/sh
# Usage: sh baseline/mk-lists.sh <mounted stock rootfs dir>
# Writes 74.00.324A.png-sizes ("<path> <width> <height>") and 74.00.324A.modes
# ("<octal mode> <uid>:<gid> <path>") next to this script. Only regular files that
# appear in 74.00.324A.md5 are listed. Output holds metadata only, no file content.
set -e
root=${1:?usage: mk-lists.sh <rootfs dir>}
here=$(cd "$(dirname "$0")" && pwd)
md5=$here/74.00.324A.md5
export LC_ALL=C
sed 's/^[0-9a-f]*  //' "$md5" | sort | while IFS= read -r p; do
  f=$root$p
  [ -f "$f" ] && [ ! -L "$f" ] || continue
  stat -c '%a %u:%g' "$f" | { read -r m o; printf '%04d %s %s\n' "$m" "$o" "$p"; }
done > "$here/74.00.324A.modes"
sed 's/^[0-9a-f]*  //' "$md5" | grep '\.png$' | sort | while IFS= read -r p; do
  f=$root$p
  [ -f "$f" ] && [ ! -L "$f" ] || continue
  set -- $(od -An -tu1 -j16 -N8 "$f")
  printf '%s %s %s\n' "$p" $(( $1*16777216 + $2*65536 + $3*256 + $4 )) $(( $5*16777216 + $6*65536 + $7*256 + $8 ))
done > "$here/74.00.324A.png-sizes"
