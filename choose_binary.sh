#!/bin/sh
TARGETARCH=$1
BINARY_X86=$2
BINARY_ARM64=$3

if [ "${TARGETARCH}" = "amd64" ]; then
    cp "${BINARY_X86}" /entsoe-logger
elif [ "${TARGETARCH}" = "arm64" ]; then
    cp "${BINARY_ARM64}" /entsoe-logger
fi
