#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly TARGET_HOST=jpursell@iprj
readonly TARGET_PATH=/home/jpursell/eveline
readonly TARGET_ARCH=arm-unknown-linux-gnueabihf
readonly SOURCE_PATH=./target/${TARGET_ARCH}/release/eveline

ssh -t ${TARGET_HOST} sudo shutdown -h now