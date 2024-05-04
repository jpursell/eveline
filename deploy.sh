#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly TARGET_HOST=jpursell@iprj
readonly TARGET_PATH=/home/jpursell/eveline
readonly TARGET_ARCH=arm-unknown-linux-gnueabihf
readonly SOURCE_PATH=./target/${TARGET_ARCH}/release/eveline
readonly TARGET_GCODE_PATH=/home/jpursell/eveline.gcode

cargo build --release --target=${TARGET_ARCH}
rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
rsync ${1} ${TARGET_HOST}:${TARGET_GCODE_PATH}
ssh -t ${TARGET_HOST} RUST_LOG=info ${TARGET_PATH} --gcode-path ${TARGET_GCODE_PATH}