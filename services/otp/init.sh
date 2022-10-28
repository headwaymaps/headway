#!/bin/bash

set -xe

if [ -f "${OTP_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${OTP_ARTIFACT_DEST_PATH}"
    exit 0
fi

if [ -f "${OTP_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cp "${OTP_ARTIFACT_SOURCE_PATH}" "${OTP_ARTIFACT_DEST_PATH}"
    exit 0
fi

echo "Downloading artifact"
wget -O "${OTP_ARTIFACT_DEST_PATH}" "${OTP_ARTIFACT_URL}"
