mod grbl;
mod display;
mod serial;
mod application;

use clap::Parser;
use anyhow::Result;
use display::layout::App;
use grbl::program::Program;
use application::routing::Channels;

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
    let program = Program::open(filename)?;


    // Open serial port.
    let mut serial_client = serial::connection::Client::new(
        &cli.port, 
        cli.baudrate, 
        std::time::Duration::from_millis(100),
    )?;
    serial_client.start()?;

    // Report positions in machine coordinates, with buffer status
    vec!["$10=3", "G21", "?", "$G"].iter().for_each(|cmd| {
        serial_client.tx.send(format!("{}\n", cmd).to_string()).unwrap();
    });

    // Create UI
    // let serial_rx = serial_client.take_receiver()?;
    let mut application = App::new(program, cli.port);
    let res = application.start();

    Channels::new(
        serial_client.take_receiver()?, 
        serial_client.tx, 
        application.rx, 
        application.tx).run_bridge();

    // TODO wait for application.handle and serial_client.handle

    // Restore states
    ratatui::restore();
    serial_client.restore();

    Ok(res?)
}
