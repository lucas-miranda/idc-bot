#!/bin/sh

set -o allexport
source ./.env
set +o allexport
cargo run
