#!/bin/bash

cd /fluidnc \
    && pio run -e noradio \
    && cp .pio/build/noradio/bootloader.bin /out/. \
    && cp .pio/build/noradio/firmware.bin /out/. \
    && cp .pio/build/noradio/partitions.bin /out/. \
    && cp /root/.platformio/packages/framework-arduinoespressif32/tools/partitions/boot_app0.bin /out/. \
    && chmod a+rw /out/*.bin
