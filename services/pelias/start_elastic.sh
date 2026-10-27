#!/bin/bash
# Serves the read-only index image, with elasticsearch's writes going to scratch.

set -xe

INDEX_IMAGE=/index/elasticsearch.erofs
DATA_DIR=/usr/share/elasticsearch/data
# The read-only index image is the base layer
LOWER=/mnt/index

# Elasticsearch's writes land in this layer, on top of the base
UPPER=/scratch/upper

# Overlayfs's own scratch space, not a layer. It must be on the same filesystem as UPPER.
WORK=/scratch/work

mkdir -p "$LOWER" "$UPPER" "$WORK"
# The overlay's root takes its owner from upper, and elasticsearch creates files there.
chown elasticsearch:elasticsearch "$UPPER"

# A loop device the host creates after the container starts has no node in
# this container's /dev.
loop=$(losetup --find)
[ -e "$loop" ] || mknod "$loop" b 7 "${loop#/dev/loop}"
mount -t erofs -o loop,ro "$INDEX_IMAGE" "$LOWER"
mount -t overlay overlay -o "lowerdir=${LOWER},upperdir=${UPPER},workdir=${WORK}" "$DATA_DIR"

# Drop root for good: no capabilities, and no way to regain them.
exec setpriv --reuid=elasticsearch --regid=elasticsearch --init-groups --no-new-privs \
    /usr/local/bin/docker-entrypoint.sh "$@"
