#!/usr/bin/env bash
# Unix / CI wrapper for the portable Node build.
set -euo pipefail
cd "$(dirname "$0")/.."
node web/build.mjs