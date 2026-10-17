mod grbl;
mod display;
mod serial;

use clap::Parser;
use anyhow::Result;
use display::layout::display;
use grbl::program::Program;

#[derive(Parser)]
#[command(name = "workspace")]
struct Cli {
    #[arg(short = 'p', long, default_value_t = "/dev/ttyUSB0".to_string())]
    port: String,

    #[arg(short = 'b', long, default_value_t = 115200)]
    baudrate: u32,

    #[arg(trailing_var_arg = true)]
    extras: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("opening port {}", cli.port);
    println!("baudrate: {}", cli.baudrate);

    if cli.extras.is_empty() {
        return Err(anyhow::anyhow!("Missing filename"));
    }

    let filename = cli.extras[0].clone();
    
    println!("Filename : {}", filename);

    let file_content = std::fs::read_to_string(filename)?;

    let gcode: Vec<String> = file_content.lines().map(str::to_owned).collect();

    let mut serial_client = serial::connection::Client::new(&cli.port, cli.baudrate)?;
    serial_client.start()?;

    display(&serial_client.rx, &serial_client.tx, Program::new(gcode))?;

    serial_client.stop();
    Ok(())
}
