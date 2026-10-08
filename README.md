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
cargo run --release -- /dev/ttyUSB0 path/to/program.gcode
```

The default serial port is `/dev/ttyUSB0` if you do not specify one:

```bash
cargo run --release -- path/to/program.gcode
```

## Running a complete program

The application loads the entire G-code program and lets you process it from the terminal interface. Use the `p` command to advance to the next G-code command. The current command is highlighted while the program is being followed.

> Warning: This application is intended for use with a real CNC only after carefully verifying the machine configuration, connections, limits, tool setup, and program contents. Always ensure that the machine is correctly homed and that the work area is clear before sending commands.

## Keyboard controls

- `p` — process the next G-code command
- `Backspace` — delete the last character in the command line
- `Enter` — send the entered command
- `Ctrl+D` — exit the application

## Development

```bash
cargo test
cargo run --release -- /dev/ttyUSB0 testdata/example.gcode
```

## License

This project is distributed under the terms of the license specified in the repository.
