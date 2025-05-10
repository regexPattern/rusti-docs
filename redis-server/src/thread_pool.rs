mod error;

use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
};

pub use error::Error;

#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        if let Some(sender) = &self.sender {
            sender
                .send(job)
                .map_err(|err| Error::SendError(format!("fallo al enviar tarea: {err}")))
        } else {
            Err(Error::NoSender)
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[derive(Debug)]
struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = match receiver.lock() {
                    Ok(receiver_guard) => receiver_guard.recv(),
                    Err(e) => {
                        print!(
                            "{}",
                            log::error!("worker {id} fallo en adquirir lock del receiver: {e}")
                        );
                        break;
                    }
                };

                match job {
                    Ok(job) => {
                        print!("{}", log::debug!("worker {id} ejecutando tarea"));
                        job();
                    }
                    Err(_) => {
                        print!("{}", log::debug!("worker {id} desconectado"));
                        break;
                    }
                }
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}
