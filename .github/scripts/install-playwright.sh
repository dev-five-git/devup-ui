#!/usr/bin/env bash
# Installs Playwright browsers and their system packages on a GitHub-hosted
# Ubuntu runner without letting a slow apt mirror hold the job hostage.
#
#   install-playwright.sh <browser>...
#
# Environment:
#   PLAYWRIGHT_CLI       the Playwright CLI to run (default: "bunx playwright")
#   ATTEMPT_TIMEOUT      the time limit of one attempt (default: 4m)
#   ATTEMPTS             how many attempts to make (default: 3)
#   PLAYWRIGHT_APT_CACHE a directory of .deb files kept between runs by
#                        actions/cache; seeded into apt before installing and
#                        refreshed after a successful install (optional)
#
# `playwright install --with-deps` runs `apt-get update` and
# `apt-get install` for the packages the browsers need. Firefox and WebKit
# need about 185 packages (140 MB) the image does not ship, and the Ubuntu
# mirror of the runner sometimes serves them at a few kB/s, so one install
# could run until the job timed out. Each attempt here is time-limited;
# packages downloaded before a timeout stay in /var/cache/apt/archives, so
# the next attempt only fetches the rest.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <browser>..." >&2
  exit 2
fi

read -r -a cli <<< "${PLAYWRIGHT_CLI:-bunx playwright}"
attempt_timeout="${ATTEMPT_TIMEOUT:-4m}"
attempts="${ATTEMPTS:-3}"
apt_cache="${PLAYWRIGHT_APT_CACHE:-}"
apt_cache="${apt_cache/#\~/$HOME}"
archives=/var/cache/apt/archives

# Keep downloaded packages, retry failed requests and drop stalled
# connections instead of waiting on them, and wait for a dpkg lock left by
# an interrupted attempt rather than failing on it.
sudo tee /etc/apt/apt.conf.d/99-playwright-ci > /dev/null << 'EOF'
APT::Keep-Downloaded-Packages "true";
Binary::apt::APT::Keep-Downloaded-Packages "true";
Acquire::Retries "3";
Acquire::http::Timeout "30";
Acquire::https::Timeout "30";
DPkg::Lock::Timeout "120";
EOF

if [ -n "$apt_cache" ] && compgen -G "$apt_cache/*.deb" > /dev/null; then
  echo "Seeding apt with $(find "$apt_cache" -name '*.deb' | wc -l) cached package(s)"
  sudo cp -n "$apt_cache"/*.deb "$archives/"
fi

attempt=1
until timeout --kill-after=20s "$attempt_timeout" "${cli[@]}" install --with-deps "$@"; do
  status=$?
  if [ "$attempt" -ge "$attempts" ]; then
    echo "::error::Playwright install failed after $attempt attempt(s) (last exit status $status)"
    exit "$status"
  fi
  echo "::warning::Playwright install attempt $attempt exited with status $status (124 = timed out after $attempt_timeout); retrying"
  # A killed attempt can leave apt-get behind sudo or dpkg half-configured.
  sudo pkill -KILL -x apt-get || true
  sudo dpkg --configure -a || true
  echo "Packages already downloaded: $(find "$archives" -maxdepth 1 -name '*.deb' | wc -l)"
  attempt=$((attempt + 1))
done
echo "Playwright installed on attempt $attempt"

if [ -n "$apt_cache" ]; then
  # Keep only packages apt can still use, so the cache does not grow with
  # every Ubuntu update.
  sudo apt-get autoclean -qq
  mkdir -p "$apt_cache"
  find "$apt_cache" -name '*.deb' -delete
  if compgen -G "$archives/*.deb" > /dev/null; then
    cp "$archives"/*.deb "$apt_cache/"
  fi
  echo "Kept $(find "$apt_cache" -name '*.deb' | wc -l) package(s) for the apt cache"
fi
