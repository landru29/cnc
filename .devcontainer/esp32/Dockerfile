FROM espressif/idf:v5.4.1
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgcrypt20 libglib2.0-0 libpixman-1-0 libsdl2-2.0-0 libslirp0 \
 && rm -rf /var/lib/apt/lists/*
RUN python3 $IDF_PATH/tools/idf_tools.py install qemu-xtensa
RUN bash -c ". $IDF_PATH/export.sh && pip install littlefs-python"
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]