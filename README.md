# CNC Controller

A Rust application for controlling a CNC machine using G-code commands.

The program reads a G-code file, displays its contents in a terminal interface, and lets you follow and send the commands of a complete machining program.

## Features

- Load a G-code program from a file
- Display the program and its current execution cursor
- Show the commands sent to the machine
- Display live GRBL status information
- Browse the program and execute its commands step by step through the terminal interface
- Ignore comments and empty lines when processing G-code
- Built with Rust and Ratatui

## Requirements

- Rust 2024 edition
- A CNC controller compatible with GRBL or a similar serial protocol
- A serial port available on the system, such as `/dev/ttyUSB0`
- A G-code file containing the machining program

## Installation

```bash
cargo build --release
```

## Usage

Run the application with the path to your G-code file:

```bash
cargo run --release --bin cnc -- /dev/ttyUSB0 path/to/program.gcode
```

The default serial port is `/dev/ttyUSB0` if you do not specify one:

```bash
cargo run --release --bin cnc -- path/to/program.gcode
```

To log frontend and GRBL errors to a file, pass `--log-file` (or `-l`):

```bash
cargo run --release --bin cnc -- --log-file cnc-errors.log path/to/program.gcode
```

The file is created if needed and appended to. Without this option, no log file is
created. Log entries include an RFC 3339 UTC timestamp.

## Developpement

1. Launch the devcontainer.
2. Inside the devcontainer, launch `/connect.sh`. It creates a node `/tmp/ttyESP` which is an UART.
3. Launch the application `cargo run --release --bin cnc -- -p /tmp/ttyESP`


## Testing

```bash
cargo test
```

## License

This project is distributed under the terms of the license specified in the repository.
