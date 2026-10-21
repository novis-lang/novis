#!/bin/sh
# Writes the deploy rows' program into `/app`, from inside the backend container.
#
# The deploy rows serve `/app` from a Docker volume, and this script writes every file in it. Docker
# Desktop's file share from a Windows or macOS host reports thousands of changed files late, so a copy
# written from the host would measure the share and not the server. A Linux server has no such share.
# `tools/nv/cmd/bench-compare.ts` runs `init` in a one-off container before the stack comes up, and
# `copy` and `switch` in the running backend while the row's own requests check every reply.
#
#   deploy.sh init   EXT COUNT LAYOUT                 version 1 of every part, and the program files from /src
#   deploy.sh copy   EXT COUNT LAYOUT VERSION PAUSE   every part at VERSION, PAUSE seconds between two files
#   deploy.sh switch EXT COUNT LAYOUT VERSION         points /app/current at releases/VERSION in one rename
#
# EXT is the parts' language: `nvs`, `php` or `ts`. COUNT is the number of parts. LAYOUT `flat` serves
# `/app` with its parts in `/app/parts/`. LAYOUT `release` serves the link `/app/current`, which points
# at `/app/releases/N/`, each a whole program. Go compiles its parts into its image and never runs this.
# `APP` and `SRC` replace `/app` and `/src`, so the driver's test can run it in a scratch folder.
#
# POSIX `sh` only: it runs under dash in the Debian images and must run under busybox too.
set -eu

APP=${APP:-/app}
SRC=${SRC:-/src}

die() {
  echo "deploy.sh: $*" >&2
  exit 2
}

[ $# -ge 4 ] || die "usage: deploy.sh init|copy|switch EXT COUNT LAYOUT [VERSION [PAUSE]]"
cmd=$1
ext=$2
count=$3
layout=$4
shift 4

case $ext in nvs | php | ts) ;; *) die "EXT is nvs, php or ts, not '$ext'" ;; esac
case $layout in flat | release) ;; *) die "LAYOUT is flat or release, not '$layout'" ;; esac
case $count in '' | *[!0-9]*) die "COUNT is a number, not '$count'" ;; esac

# A part's file name is its number padded to the digits of the highest one: `p000` for 200 parts and
# `p0000` for 2000. `pad` is 10 to the power of those digits, and `${n#1}` drops its leading 1, so
# the name is made without starting a process per file.
last=$((count - 1))
pad=1
d=0
while [ "$d" -lt "${#last}" ]; do
  pad=$((pad * 10))
  d=$((d + 1))
done

# The folder a version's program is served from.
home() {
  if [ "$layout" = release ]; then echo "$APP/releases/$1"; else echo "$APP"; fi
}

part_text() {
  case $ext in
    nvs) printf '<?nvs\necho "%s";\n' "$1" ;;
    php) printf '<?php echo "%s";\n' "$1" ;;
    ts) printf 'export const version = "%s";\n' "$1" ;;
  esac
}

pause() {
  [ "$1" = 0 ] || sleep "$1"
}

# Every part at version $1 in folder $2, with a pause of $3 seconds after each file.
write_parts() {
  mkdir -p "$2/parts"
  i=0
  while [ "$i" -lt "$count" ]; do
    n=$((pad + i))
    part_text "$1" >"$2/parts/p${n#1}.$ext"
    pause "$3"
    i=$((i + 1))
  done
}

# Novis's entry in folder $1. `require` takes a path written as literal text, so it names each part
# on its own line.
write_entry() {
  {
    printf '<?nvs\n// Prints the version of each of the %s parts. deploy.sh writes this file and the parts.\n' "$count"
    i=0
    while [ "$i" -lt "$count" ]; do
      n=$((pad + i))
      printf "require 'parts/p%s.nvs';\n" "${n#1}"
      i=$((i + 1))
    done
    printf 'echo "\\n";\n'
  } >"$1/main.nvs"
}

case $cmd in
  init)
    # A volume left by an earlier run is emptied first, hidden files too.
    rm -rf "$APP"/* "$APP"/.[!.]* "$APP"/..?*
    dir=$(home 1)
    mkdir -p "$dir"
    cp "$SRC"/* "$dir"/
    write_parts 1 "$dir" 0
    if [ "$ext" = nvs ]; then write_entry "$dir"; fi
    if [ "$layout" = release ]; then ln -s releases/1 "$APP/current"; fi
    ;;
  copy)
    [ $# -eq 2 ] || die "copy needs VERSION and PAUSE"
    dir=$(home "$1")
    if [ "$layout" = release ]; then
      # A new release is a whole program: release 1's program files first, then the parts.
      mkdir -p "$dir"
      for f in "$APP/releases/1"/*; do
        if [ -f "$f" ]; then
          cp "$f" "$dir"/
          pause "$2"
        fi
      done
    fi
    write_parts "$1" "$dir" "$2"
    ;;
  switch)
    [ $# -eq 1 ] || die "switch needs VERSION"
    [ "$layout" = release ] || die "switch needs LAYOUT release"
    ln -s "releases/$1" "$APP/next"
    mv -T "$APP/next" "$APP/current"
    ;;
  *) die "the command is init, copy or switch, not '$cmd'" ;;
esac
