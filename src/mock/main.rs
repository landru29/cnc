use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use std::io::{self, ErrorKind, Read, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "workspace")]
struct Cli {
    #[arg(short = 'c', long, default_value_t = "testdata/mock.yaml".to_string())]
    config: String,

    #[arg(short = 'p', long, default_value_t = "/dev/ttyUSB0".to_string())]
    port: String,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    default: String,
    #[serde(default)]
    specs: Vec<Spec>,
}

#[derive(Deserialize)]
struct Spec {
    when: String,
    then: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_text = std::fs::read_to_string(&cli.config)?;
    let config: Config = serde_yaml::from_str(&config_text)?;

    println!("using config {}", cli.config);
    println!("using port {}", cli.port);
    println!("press Ctrl+C or Ctrl+D to exit");

    let mut uart = serialport::new(&cli.port, 115_200)
        .timeout(Duration::from_millis(100))
        .open()?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let signal_shutdown = Arc::clone(&shutdown);
    ctrlc::set_handler(move || signal_shutdown.store(true, Ordering::Relaxed))?;

    let stdin_shutdown = Arc::clone(&shutdown);
    thread::spawn(move || {
        let mut byte = [0_u8; 1];
        loop {
            match io::stdin().read(&mut byte) {
                Ok(_) => {
                    println!("stdin closed, shutting down");
                    stdin_shutdown.store(true, Ordering::Relaxed);
                    break;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    });

    let mut input = Vec::new();
    let mut bytes = [0_u8; 256];
    while !shutdown.load(Ordering::Relaxed) {
        match uart.read(&mut bytes) {
            Ok(0) => continue,
            Ok(count) => {
                for byte in &bytes[..count] {
                    if *byte == b'\r' || *byte == b'\n' {
                        if !input.is_empty() {
                            respond(&mut uart, &config, &input)?;
                            input.clear();
                        }
                    } else {
                        input.push(*byte);
                        if input.len() == 1 && has_match(&config, &input) {
                            respond(&mut uart, &config, &input)?;
                            input.clear();
                        }
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::TimedOut | ErrorKind::WouldBlock | ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }

    println!("shutting down");

    Ok(())
}

fn has_match(config: &Config, request: &[u8]) -> bool {
    config.specs.iter().any(|spec| spec.when.as_bytes() == request)
}

fn respond<W: Write>(uart: &mut W, config: &Config, request: &[u8]) -> Result<()> {
    let request = String::from_utf8_lossy(request);
    println!(" < request: {}", request);

    let response = config
        .specs
        .iter()
        .find(|spec| spec.when.to_lowercase() == request.to_lowercase())
        .map_or(&config.default, |spec| &spec.then);

    println!(" > response: {}", response);

    uart.write_all(response.as_bytes())?;
    if !response.ends_with('\n') {
        uart.write_all(b"\n")?;
    }
    uart.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_config_replies_to_matching_and_unknown_requests() {
        let config: Config = serde_yaml::from_str(include_str!("../../testdata/mock.yaml")).unwrap();
        let mut response = Vec::new();

        respond(&mut response, &config, b"?").unwrap();
        assert_eq!(response, b"<Run|MPos:10.250,20.500,-1.000|WPos:10.250,20.500,-1.000|FS:500,0>\n");

        response.clear();
        respond(&mut response, &config, b"$G").unwrap();
        assert_eq!(response, b"[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]\nok\n");

        response.clear();
        respond(&mut response, &config, b"unknown").unwrap();
        assert_eq!(response, b"ok\n");
    }
}