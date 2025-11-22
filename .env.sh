#!/bin/bash
# Wrapper to export variables from .env for shell scripts
# Dagger can use .env directly, scripts should source this file instead

set -a
source "$(dirname "${BASH_SOURCE[0]}")/.env"
set +a
