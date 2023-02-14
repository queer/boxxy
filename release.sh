#!/usr/bin/env bash

version=$(cat Cargo.toml | grep version | head -n1 | awk '{print $3}' | sed -e 's/"//g')

cargo release "${version}" --execute --no-publish

./install.sh
