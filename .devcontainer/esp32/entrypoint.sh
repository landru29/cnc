#!/bin/bash
set -e
. "$IDF_PATH/export.sh" >/dev/null 2>&1

littlefs-python create /fs /tmp/littlefs.bin \
  --fs-size=$((LFS_SIZE)) --block-size=4096

esptool.py --chip esp32 merge_bin -o /tmp/flash.bin --fill-flash-size 4MB \
  0x1000  /firmware/bootloader.bin \
  0x8000  /firmware/partitions.bin \
  0xe000  /firmware/boot_app0.bin \
  0x10000 /firmware/firmware.bin \
  $LFS_OFFSET /tmp/littlefs.bin

exec qemu-system-xtensa -nographic -machine esp32 \
  -drive file=/tmp/flash.bin,if=mtd,format=raw \
  -serial tcp:0.0.0.0:5555,server,nowait \
  -global driver=timer.esp32.timg,property=wdt_disable,value=true