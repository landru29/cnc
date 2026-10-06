#!/bin/bash

rm -f /tmp/ttyV0 /tmp/ttyV1

echo "Creating virtual serial ports /tmp/ttyV0 and /tmp/ttyV1 ..."
socat \
  pty,raw,echo=0,link=/tmp/ttyV0 \
  pty,raw,echo=0,link=/tmp/ttyV1  &

for i in {1..20}; do
    [ -e /tmp/ttyV0 ] && [ -e /tmp/ttyV1 ] && break
    sleep 0.1
done

echo "done."