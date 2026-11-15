FROM espressif/idf:v6.1.1

ARG FLUIDNC_REF=v4.1.1


RUN apt-get update && apt-get install -y --no-install-recommends\
    libgcrypt20 \
    libglib2.0-0 \
    libpixman-1-0 \
    libsdl2-2.0-0 \
    libslirp0 \
    git \
    curl \
    unzip \
   && rm -rf /var/lib/apt/lists/*
   
RUN python3 $IDF_PATH/tools/idf_tools.py install qemu-xtensa
RUN bash -c ". $IDF_PATH/export.sh && pip install littlefs-python"

RUN git clone --depth 1 --branch ${FLUIDNC_REF} \
      https://github.com/bdring/FluidNC /fluidnc

COPY entrypoint.sh /entrypoint.sh
COPY debug.sh /debug.sh
RUN chmod +x /entrypoint.sh &&  chmod +x /debug.sh

ENV PATH="$PATH:/opt/esp/tools/xtensa-esp-elf-gdb/17.1_20260402/xtensa-esp-elf-gdb/bin"

ENTRYPOINT ["/entrypoint.sh"]