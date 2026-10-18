use anyhow::Result;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub struct Client {
    pub rx: Receiver<String>,
    pub tx: Sender<String>,
    pub port: Option<Box<dyn serialport::SerialPort>>,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    command_rx: Option<Receiver<String>>,
    serial_tx: Sender<String>,
}

impl Client {
    pub fn new(path: &str, baudrate: u32) -> Result<Self> {
        let (serial_tx, rx) = channel::<String>();
        let (tx, command_rx) = channel::<String>();

        let port = serialport::new(path, baudrate)
            .timeout(Duration::from_millis(100))
            .open()?;

        let running = Arc::new(AtomicBool::new(true));

        Ok(Self {
            rx,
            tx,
            port: Some(port),
            running,
            handle: None,
            command_rx: Some(command_rx),
            serial_tx,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        let command_rx = self.command_rx.take().expect("serial command receiver should exist");
        let serial_tx = self.serial_tx.clone();

        let Some(port) = self.port.as_ref() else {
            anyhow::bail!("serial port is disconnected");
        };
        let mut port = port.try_clone()?;
        let running_thread = Arc::clone(&self.running);

        let handle = std::thread::spawn(move || {
            let mut buffer: Vec<u8> = vec![0; 1024];

            while running_thread.load(Ordering::Relaxed) {
                while let Ok(command) = command_rx.try_recv() {
                        if let Err(e) = port.write_all(command.as_bytes()) {
                            eprintln!("Error writing to serial port: {}", e);
                        }
                        if let Err(e) = port.write_all(b"\n") {
                            eprintln!("Error writing newline to serial port: {}", e);
                        }
                }
                
                match port.read(buffer.as_mut_slice()) {
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                            for line in data.lines().map(str::to_owned) {
                                if let Err(e) = serial_tx.send(line) {
                                    eprintln!("Error sending data: {}", e);
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                    Err(e) => {
                        eprintln!("Error reading from serial port: {}", e);
                        break;
                    }
                }


            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("Failed to join thread");
        }
        self.port.take();
    }
}