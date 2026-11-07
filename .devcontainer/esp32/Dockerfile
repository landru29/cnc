FROM espressif/idf:v5.4.1
RUN apt-get update && apt-get install -y --no-install-recommends\
    libgcrypt20 libglib2.0-0 libpixman-1-0 libsdl2-2.0-0 libslirp0 curl unzip \
 && rm -rf /var/lib/apt/lists/*
RUN python3 $IDF_PATH/tools/idf_tools.py install qemu-xtensa
RUN bash -c ". $IDF_PATH/export.sh && pip install littlefs-python"
RUN curl -L -o /tmp/fluidnc.zip https://github.com/bdring/FluidNC/archive/refs/heads/main.zip && cd / && unzip -x /tmp/fluidnc.zip
COPY entrypoint.sh /entrypoint.sh
COPY debug.sh /debug.sh
RUN chmod +x /entrypoint.sh
ENV PATH="$PATH:/opt/esp/tools/xtensa-esp-elf-gdb/14.2_20240403/xtensa-esp-elf-gdb/bin"
ENTRYPOINT ["/entrypoint.sh"]