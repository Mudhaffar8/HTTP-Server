use std::{
    thread,
    sync::{Arc, Mutex, mpsc}
};

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>
}

type Job = Box<dyn FnOnce() + Send + 'static>;

// Note: Use thread::Builder for Error Checking
impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let msg = receiver.lock().unwrap().recv();
                
                match msg {
                    Ok(job) => {
                        println!("Worker {id} got a job; Executing...");

                        job();
                    },
                    Err(_) => {
                        println!("Worker {id} shutting!");
                        break;
                    }
                }
            }
        });
        
        Worker { id, thread }
    }
}

pub struct ThreadPool {
    threads: Vec<Worker>,
    sender: mpsc::Sender<Job>
}

impl ThreadPool {
    pub fn new(n: usize) -> ThreadPool {
        if n == 0 { panic!(); }

        let mut workers: Vec<Worker> = Vec::with_capacity(n);

        let (sender, receiver) = mpsc::channel();
        
        let receiver = Arc::new(Mutex::new(receiver));

        for i in 0..n { 
            workers.push(Worker::new(i, Arc::clone(&receiver))); 
        }

        ThreadPool { threads: workers, sender }
    }

    pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
    
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in self.threads.drain(..) {
            println!("Worker {} is shutting down!", worker.id);

            worker.thread.join().unwrap();
        }
    }
}
