#!/bin/sh
set -e

# Fix permissions on data directory if running as root
if [ "$(id -u)" = '0' ]; then
    # Ensure data directory exists and has correct ownership
    mkdir -p /data
    chown -R hmanlab:hmanlab /data

    # Drop to hmanlab user and execute command
    exec gosu hmanlab "$@"
fi

exec "$@"
