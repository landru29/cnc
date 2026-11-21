use anyhow::Result as AnyhowResult;

use std::{
    sync::{
        Arc,
        atomic::{Ordering, AtomicBool},
    },
    thread::JoinHandle,
};

use crate::{application::application::Application, frontend::layout::Frontend};


pub struct ThreadedApp {
    handle: Option<JoinHandle<Frontend>>,
    terminal: Option<Frontend>,
}


impl ThreadedApp {
    pub fn new() -> Self {
        Self { handle: None, terminal: None }
    }

    pub fn start(&mut self, mut app: Application, running: Arc<AtomicBool>,) -> AnyhowResult<()> {
        if self.handle.is_some() {
            anyhow::bail!("Application UI has already been started");
        }
        let running = Arc::clone(&running);

        let mut frontend = Frontend::new()?;

        self.handle = Some(std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if app.run_iteration(&mut frontend).is_err() {
                    break;
                }
            }
            frontend
        }));

        Ok(())
    }

    pub fn wait(&mut self) -> AnyhowResult<()> {
        if let Some(handle) = self.handle.take() {
            self.terminal = Some(handle
                .join()
                .map_err(|_| anyhow::anyhow!("UI thread panicked"))?);
        }

        self.terminal.as_mut().unwrap().leave()?;

        Ok(())
    }
}