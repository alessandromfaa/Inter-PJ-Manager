#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
exec "${ENGINE:-docker}" compose run --rm api migrate
