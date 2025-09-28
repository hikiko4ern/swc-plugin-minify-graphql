#!/usr/bin/env bash

set -xeo pipefail

if [ -z "$npm_package_version" ]; then
	echo "npm_package_version env is not set"
	exit 1
fi

cargo bin cargo-set-version --exclude graphql-minify "$npm_package_version"
