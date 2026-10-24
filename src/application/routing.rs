use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub struct Channels {
    pub serial_rx: Receiver<String>,
    pub serial_tx: Sender<String>,
    pub ui_rx: Receiver<String>,
    pub ui_tx: Sender<String>,
}

impl Channels {
    pub fn new(serial_rx: Receiver<String>,
                serial_tx: Sender<String>,
                ui_rx: Receiver<String>,
                ui_tx: Sender<String>) -> Self {
        Self{
            serial_rx,
            serial_tx,
            ui_rx,
            ui_tx,
        }
    }

    pub fn run_bridge(self) -> (thread::JoinHandle<()>, thread::JoinHandle<()>) {
        let Channels { serial_rx, serial_tx, ui_rx, ui_tx } = self;

        // serial_rx -> ui_tx
        let h1 = thread::spawn(move || {
            for msg in serial_rx {              // se termine quand tous les Sender de rx1 sont drop
                if ui_tx.send(msg).is_err() {
                    break;                // le récepteur de ui_tx n'existe plus
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

        (h1, h2)
    }
}