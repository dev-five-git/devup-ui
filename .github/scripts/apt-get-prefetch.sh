#!/usr/bin/env bash
# Stands in for apt-get on PATH while `playwright install-deps` runs (see
# install-playwright.sh). For `apt-get install` it first downloads the
# packages apt would fetch, several at a time and from fallback Ubuntu
# mirrors when the runner's mirror is slow, into apt's archive directory;
# then it runs the real apt-get, which finds them already downloaded.
# Anything the prefetch misses, the real apt-get still downloads itself.
set -uo pipefail

real=/usr/bin/apt-get

if [ "${1:-}" = install ]; then
  archives=/var/cache/apt/archives
  mirrors=(${PREFETCH_MIRRORS:-http://archive.ubuntu.com/ubuntu http://azure.archive.ubuntu.com/ubuntu})

  fetch() {
    local url="${1//\'/}" file="$2" hash="$3"
    local algo="${hash%%:*}" sum="${hash#*:}" path tmp candidate
    [ -s "$archives/$file" ] && return 0
    path="${url#*/ubuntu/}"
    tmp="$(mktemp -p "$archives/partial")"
    for candidate in "$url" "${mirrors[@]/%//$path}"; do
      # Give up on a connection slower than 50 kB/s for 15 s and move on to
      # the next mirror.
      if curl -fsSL --connect-timeout 10 --speed-limit 51200 --speed-time 15 \
        --max-time 300 -o "$tmp" "$candidate" &&
        echo "$sum  $tmp" | "$(echo "$algo" | tr '[:upper:]' '[:lower:]')sum" --status -c; then
        mv "$tmp" "$archives/$file"
        return 0
      fi
    done
    rm -f "$tmp"
    echo "prefetch: could not download $file; apt-get will retry it" >&2
  }
  export -f fetch
  export archives
  export PREFETCH_MIRROR_LIST="${mirrors[*]}"

  started=$SECONDS
  uris="$("$real" "$@" --print-uris -qq 2> /dev/null | grep "^'" || true)"
  count="$(printf '%s' "$uris" | grep -c . || true)"
  if [ "$count" -gt 0 ]; then
    echo "prefetch: downloading $count package(s), 8 at a time"
    printf '%s\n' "$uris" |
      xargs -P 8 -L 1 bash -c 'mirrors=($PREFETCH_MIRROR_LIST); fetch "$0" "$1" "$3"'
    echo "prefetch: done in $((SECONDS - started))s"
  fi
fi

exec "$real" "$@"
