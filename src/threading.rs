use std::{
    thread,
    sync::{Arc, Mutex, mpsc}
};

/// Represents worker thread with associated ID and handle.
struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>
}

type Job = Box<dyn FnOnce() + Send + 'static>;

// TODO: Use thread::Builder for Error Checking
impl Worker {
    /// Initializes a new `Worker` that continously listens for and executes jobs on the provided
    /// receiver. The worker will continue to do so until it
    /// encounters an error (such as when the sender is closed).
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

/// Manages a pool of worker threads that can execute submitted jobs concurrently.
/// 
/// Jobs are submitted through channels to workers threads, in which one of them pulls and executes them.
pub struct ThreadPool {
    threads: Vec<Worker>,
    sender: mpsc::Sender<Job>
}

impl ThreadPool {
    /// Creates a new `ThreadPool` with specified number of worker threads.
    /// 
    /// # Parameters
    /// `n` - The number of worker threads to instantiate in this pool.
    /// 
    /// # Panics
    /// This function will panic if `n` is zero.
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

    /// Sends a job to the thread pool to be executed by one of the worker threads.
    /// 
    /// # Panics
    /// This function will panic if sending the job to the worker thread fails.
    pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
    
}

/// Ensures all worker threads finish their tasks and gracefullly shuts down thread pool.
///
/// # Panics
/// This function will panic if joining a worker thread fails.
impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in self.threads.drain(..) {
            println!("Worker {} is shutting down!", worker.id);

            worker.thread.join().unwrap();
        }
    }
}
