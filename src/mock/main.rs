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

    #[arg(short = 'b', long, default_value_t = 115200)]
    baudrate: u32,
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
    then: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_text = std::fs::read_to_string(&cli.config)?;
    let config: Config = serde_yaml::from_str(&config_text)?;

    for spec in &config.specs {
        println!("catching: {}", spec.when);
    }

    println!("using config {}", cli.config);
    println!("using port {}", cli.port);
    println!("baudrate: {}", cli.baudrate);
    println!("press Ctrl+C or Ctrl+D to exit");

    let mut uart = serialport::new(&cli.port, cli.baudrate)
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

    let mut iteration: u64 = 0;

    while !shutdown.load(Ordering::Relaxed) {
        match uart.read(&mut bytes) {
            Ok(0) => continue,
            Ok(count) => {
                for byte in &bytes[..count] {
                    if *byte == b'\r' || *byte == b'\n' {
                        if !input.is_empty() {
                            println!("iteration {}", iteration);
                            respond(&mut uart, &config, &input, iteration)?;
                            iteration += 1;
                            input.clear();
                        }
                    } else {
                        input.push(*byte);
                        if input.len() == 1 && has_match(&config, &input) {
                            println!("iteration {}", iteration);
                            respond(&mut uart, &config, &input, iteration)?;
                            iteration += 1;
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
    config.specs.iter().any(|spec| {
        match_when(spec.when.as_bytes(), request)
    })
}

fn match_when(from: &[u8], to: &[u8]) -> bool {
    if let Some(hex) = from.strip_prefix(b"0x") {
        if to.len() != 1 {
            return false;
        }

        if let Some(value) = std::str::from_utf8(hex)
            .ok()
            .and_then(|hex|{
                u8::from_str_radix(hex, 16).ok()
            }) {
                return value == to[0];
            } else {
                return false;
            }
    } else {
        from.eq_ignore_ascii_case(to)
    }
}

fn respond<W: Write>(uart: &mut W, config: &Config, request: &[u8], iteration: u64) -> Result<()> {
    let request = String::from_utf8_lossy(request);
    println!(" < request: {}", request);

    let response = config
        .specs
        .iter()
        .find(|spec| match_when(spec.when.as_bytes(), request.as_bytes()))
        .map_or(&config.default, |spec| {
            if spec.then.is_empty() {
                &config.default
            } else {
                let idx: usize = (iteration as usize) % spec.then.len();
                &spec.then[idx]
            }
        });

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
    fn match_when_handles_text_and_hex_bytes_without_panicking() {
        assert!(match_when(b"$G", b"$g"));
        assert!(match_when(b"0x08", b"\x08"));
        assert!(!match_when(b"0xgg", b"\x08"));
        assert!(!match_when(b"0x08", b"\x08\x09"));
    }

    #[test]
    fn sample_config_replies_to_matching_and_unknown_requests() {
        let config: Config = serde_yaml::from_str(include_str!("../../testdata/fluidnc.yaml")).unwrap();
        let mut response = Vec::new();

        respond(&mut response, &config, b"?", 0).unwrap();
        assert_eq!(response, b"<Idle|MPos:10.250,20.500,-1.000|WPos:10.250,20.500,-1.000|Bf:15,127|FS:500,0|Pn:Q>\n");

        response.clear();
        respond(&mut response, &config, b"$G", 0).unwrap();
        assert_eq!(response, b"[GC:G0 G54 G17 G21 G91 G94 M5 M9 T0 F0 S0]\nok\n");

        response.clear();
        respond(&mut response, &config, b"unknown", 0).unwrap();
        assert_eq!(response, b"ok\n");
    }
}