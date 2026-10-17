use anyhow::Result;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub struct Client {
    pub tx: Sender<Vec<String>>,
    pub rx: Receiver<Vec<String>>,
    pub port: Option<Box<dyn serialport::SerialPort>>,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Client {
    pub fn new(path: &str, baudrate: u32) -> Result<Self> {
        let (tx, rx) = channel::<Vec<String>>();

        let port = serialport::new(path, baudrate)
            .timeout(Duration::from_millis(100))
            .open()?;

        let running = Arc::new(AtomicBool::new(true));

        Ok(Self { tx, rx, port: Some(port), running, handle: None })
    }

    pub fn start(&mut self) -> Result<()> {
        let tx = self.tx.clone();
        let Some(port) = self.port.as_ref() else {
            anyhow::bail!("serial port is disconnected");
        };
        let mut port = port.try_clone()?;
        let running_thread = Arc::clone(&self.running);

        let handle = std::thread::spawn(move || {
            let mut buffer: Vec<u8> = vec![0; 1024];

            while running_thread.load(Ordering::Relaxed) {
                match port.read(buffer.as_mut_slice()) {
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                            let lines: Vec<String> = data.lines().map(str::to_owned).collect();
                            if let Err(e) = tx.send(lines) {
                                eprintln!("Error sending data: {}", e);
                                break;
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