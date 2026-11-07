#!/bin/bash

cd /FluidNC-main
xtensa-esp32-elf-gdb /firmware/firmware.elf -ex "target remote :1234"


# Launch picocom from the devcontainer
#    /connect.sh
#    picocom /tmp/ttyESP32
#
# Add a breabkpoint
#     hbreak Stepper::pulse_func
#     continue
#
# From the picocom
#     $X
#     G91
#     G0 X100
