#!/usr/bin/env bash

cargo build --release
cp -v target/release/boxxy ~/bin
