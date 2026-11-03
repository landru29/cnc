mod application;
mod display;
mod grbl;
mod logging;
mod serial;
mod history;

use anyhow::Result;
use application::routing::Channels;
use clap::Parser;
use display::layout::{Frontend, ThreadedFrontend};
use grbl::program::Program;
use logging::ErrorLogger;
use sys_locale::get_locale;

use crate::grbl::helper::{Helper, Lang};

#[derive(Parser)]
#[command(name = "workspace")]
struct Cli {
    #[arg(short = 'p', long, default_value_t = "/dev/ttyUSB0".to_string())]
    port: String,

    #[arg(short = 'b', long, default_value_t = 115200)]
    baudrate: u32,

    #[arg(short = 'l', long)]
    log_file: Option<String>,

    #[arg(trailing_var_arg = true)]
    extras: Vec<String>,

    #[arg(long, default_value_t = "".to_string())]
    lang: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let logger = cli.log_file.as_deref().map(ErrorLogger::new).transpose()?;

    let lang: Lang = if cli.lang != "" {
        Lang::from(cli.lang)
    } else {
        let locale = get_locale().unwrap_or_else(|| String::from("en-US"));
        Lang::from(locale.split(['-', '_']).next().unwrap_or("en").to_string())
    };
    let command_helper = Helper::new(lang)?;

    println!("opening port {}", cli.port);
    println!("baudrate: {}", cli.baudrate);

    let program = if cli.extras.is_empty() {
        None
    } else {
        let filename = cli.extras[0].clone();
        println!("Filename : {}", filename);
        Some(Program::open(filename)?)
    };

    // Open serial port.
    let mut serial_client = serial::connection::Client::new(
        &cli.port,
        cli.baudrate,
        std::time::Duration::from_millis(100),
    )?;
    serial_client.start()?;

    // Report positions in machine coordinates, with buffer status
    vec!["$10=3", "G21", "?", "$G"].iter().for_each(|cmd| {
        serial_client
            .tx
            .send(format!("{}\n", cmd).to_string())
            .unwrap();
    });

    // Create UI
    let mut ui = ThreadedFrontend::new();
    let mut frontend = Frontend::new(program, cli.port, logger, command_helper);
    let frontend_command_rx = frontend.take_command_receiver()?;
    let frontend_tx = frontend.tx.clone();
    ui.start(frontend)?;

    Channels::new(
        serial_client.take_receiver()?,
        serial_client.tx.clone(),
        frontend_command_rx,
        frontend_tx,
    )
    .run_bridge();

    ui.wait()?;

    // Restore states
    ratatui::restore();
    serial_client.restore();

    Ok(())
}
