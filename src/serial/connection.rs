use anyhow::Result;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use super::feedback::Feedback;

use log::{info, error};


pub struct Client {
    rx: Option<Receiver<Feedback>>,
    pub tx: Sender<String>,
    pub port: Option<Box<dyn serialport::SerialPort>>,
    running: Arc<AtomicBool>,
    pub handle: Option<JoinHandle<()>>,
    command_rx: Option<Receiver<String>>,
    serial_tx: Sender<Feedback>,
    path: String,
    baudrate: u32,
    timeout: Duration,
}

impl Client {
    pub fn new(path: &str, baudrate: u32, timeout: Duration, running: Arc<AtomicBool>) -> Result<Self> {
        let (serial_tx, rx) = channel::<Feedback>();
        let (tx, command_rx) = channel::<String>();

        let port = serialport::new(path, baudrate)
            .timeout(timeout)
            .open()?;

        Ok(Self {
            rx: Some(rx),
            tx,
            port: Some(port),
            running,
            handle: None,
            command_rx: Some(command_rx),
            serial_tx,
            path: path.to_string(),
            baudrate,
            timeout,
        })
    }

    pub fn take_receiver(&mut self) -> Result<Receiver<Feedback>> {
        self.rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("serial receiver has already been taken"))
    }

    pub fn start(&mut self) -> Result<()> {
        let command_rx = self.command_rx.take().expect("serial command receiver should exist");
        let serial_tx = self.serial_tx.clone();

        let Some(port) = self.port.as_ref() else {
            anyhow::bail!("serial port is disconnected");
        };
        let mut port = port.try_clone()?;
        let running_thread = Arc::clone(&self.running);
        let path = self.path.clone();
        let baudrate = self.baudrate;
        let timeout = self.timeout;

        let handle = std::thread::spawn(move || {
            let mut read_buffer: Vec<u8> = vec![0; 1024];
            let mut line_buffer = BufferedData::new();

            // Outer loop: attempts to (re)connect to the serial port.
            while running_thread.load(Ordering::Relaxed) {
                // Inner loop: read from the currently open port until the
                // connection is lost, then break out to attempt a reconnect.
                let mut connected = true;
                while connected && running_thread.load(Ordering::Relaxed) {
                    while let Ok(command) = command_rx.try_recv() {
                        let mut data_to_send = command.clone();
                        if !data_to_send.ends_with('\n') {
                            data_to_send.push('\n');
                        }

                        info!(">>> Sending command to serial port: {}", data_to_send.trim_end());

                        if let Err(e) = port.write_all(data_to_send.as_bytes()) {
                            error!("Error writing to serial port: {}", e);
                            connected = false;
                            break;
                        }
                    }

                    if !connected {
                        break;
                    }

                    match port.read(read_buffer.as_mut_slice()) {
                        Ok(bytes_read) => {
                            if bytes_read > 0 {
                                line_buffer.push(&read_buffer[..bytes_read]);

                                for line in line_buffer.by_ref() {
                                    if line.is_empty() {
                                        continue
                                    }

                                    match line.parse::<Feedback>() {
                                        Ok(feedback) => {
                                            info!("<<< Received data from serial port: {}", feedback);

                                            if let Err(e) = serial_tx.send(feedback) {
                                                eprintln!("Error sending data: {}", e);
                                            }
                                        },
                                        Err(e) => {
                                            error!("Error parsing feedback from serial port: {}", e);
                                        }
                                    };
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                        Err(e) => {
                            error!("Connection to serial port lost: {}", e);
                            connected = false;
                        }
                    }
                }

                if !running_thread.load(Ordering::Relaxed) {
                    break;
                }

                // Attempt to reconnect.
                info!("Attempting to reconnect to serial port...");
                loop {
                    if !running_thread.load(Ordering::Relaxed) {
                        break;
                    }

                    match serialport::new(&path, baudrate).timeout(timeout).open() {
                        Ok(new_port) => {
                            info!("Reconnected to serial port");
                            port = new_port;
                            line_buffer = BufferedData::new();
                            break;
                        }
                        Err(e) => {
                            error!("Reconnect failed: {}", e);
                            std::thread::sleep(Duration::from_secs(1));
                        }
                    }
                }
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    pub fn restore(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("Failed to join thread");
        }
        self.port.take();
    }
}


struct BufferedData{
    remaining: Vec<u8>
}

impl BufferedData {
    fn new() -> Self {
        Self {
            remaining: vec![],
        }
    }

    fn push(&mut self, data: &[u8]) {
        self.remaining.extend_from_slice(data);
    }

}

impl Iterator for BufferedData {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.remaining.iter().position(|&b| b == b'\n')?;

        let line = self.remaining.drain(..=pos).collect::<Vec<u8>>();

        Some(String::from_utf8_lossy(&line[..line.len() - 1]).into_owned())
    }
}