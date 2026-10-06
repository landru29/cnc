mod grbl;
mod display;

use clap::Parser;
use anyhow::Result;
use display::layout::display;

#[derive(Parser)]
#[command(name = "workspace")]
struct Cli {
    #[arg(long, default_value_t = "/dev/ttyUSB0".to_string())]
    port: String,

    #[arg(trailing_var_arg = true)]
    extras: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("opening port {}", cli.port);

    if cli.extras.is_empty() {
        return Err(anyhow::anyhow!("Missing filename"));
    }

    let filename = cli.extras[0].clone();
    
    println!("Filename : {}", filename);

    let file_content = std::fs::read_to_string(filename)?;

    let gcode: Vec<String> = file_content.lines().map(str::to_owned).collect();

    display(gcode)?;

    Ok(())
}
