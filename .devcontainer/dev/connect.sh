#!/bin/bash

nohup socat pty,link=/tmp/ttyESP32,raw,echo=0 tcp:esp32:5555,forever,interval=1 >/tmp/socat.log 2>&1 &