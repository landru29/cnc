use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

pub struct Channels {
    pub serial_rx: Receiver<String>,
    pub serial_tx: Sender<String>,
    pub ui_rx: Receiver<String>,
    pub ui_tx: Sender<String>,
}

impl Channels {
    pub fn new(
        serial_rx: Receiver<String>,
        serial_tx: Sender<String>,
        ui_rx: Receiver<String>,
        ui_tx: Sender<String>,
    ) -> Self {
        Self { serial_rx, serial_tx, ui_rx, ui_tx }
    }

    pub fn run_bridge(
        self,
    ) -> (
        thread::JoinHandle<()>,
        thread::JoinHandle<()>,
        thread::JoinHandle<()>,
    ) {
        let Channels { serial_rx, serial_tx, ui_rx, ui_tx } = self;

        // Clone pour le thread de polling, avant que serial_tx soit déplacé dans h2
        let poll_tx = serial_tx.clone();

        // serial_rx -> ui_tx
        let h1 = thread::spawn(move || {
            for msg in serial_rx {
                if ui_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // ui_rx -> serial_tx
        let h2 = thread::spawn(move || {
            for msg in ui_rx {
                if serial_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // Polling toutes les secondes : "?" et "$G"
        let h3 = thread::spawn(move || loop {
            if poll_tx.send("?".to_string()).is_err() {
                break;
            }
            if poll_tx.send("$G".to_string()).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        });

        (h1, h2, h3)
    }
}