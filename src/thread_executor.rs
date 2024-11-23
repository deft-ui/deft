use std::sync::{Arc, Barrier, mpsc, Mutex};
use std::sync::mpsc::Sender;
use std::thread;

pub struct SingleThreadExecutor<I> {
    sender: Sender<Msg<I>>,
}

impl<I> Clone for SingleThreadExecutor<I> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

struct Msg<I> {
    executor: Box<dyn FnOnce(&mut I)>,
}

unsafe impl<I> Send for Msg<I>{}

impl<I: 'static> SingleThreadExecutor<I> {

    pub fn new<F>(creator: F) -> Self where F: Send + FnOnce() -> I + 'static {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut inst = creator();
            loop {
                let msg: Msg<I> = match receiver.recv() {
                    Err(err) => {
                        println!("error:{}", err);
                        break;
                    }
                    Ok(f) => f,
                };
                (msg.executor)(&mut inst);
            }

        });
        Self {
            sender,
        }
    }

    pub fn run<R, F>(&self, task: F) -> TaskHandle<R>
    where R: Send + 'static,F: Send + FnOnce(&mut I) -> R + 'static
    {
        let result_arc = Arc::new(Mutex::new(None));
        let barrier = Arc::new(Barrier::new(2));
        {
            let result_arc = result_arc.clone();
            let barrier = barrier.clone();
            self.sender.send(Msg {
                executor: Box::new(move |mut instance| {
                    let result = task(&mut instance);
                    let mut r = result_arc.lock().unwrap();
                    *r = Some(result);
                    barrier.wait();
                }),
            }).unwrap();
        }
        TaskHandle { result_arc, barrier }
    }


}

pub struct TaskHandle<R> {
    result_arc: Arc<Mutex<Option<R>>>,
    barrier: Arc<Barrier>,
}

impl<R> TaskHandle<R> {
    pub fn wait(self) -> R {
        self.barrier.wait();
        let mut lock = self.result_arc.lock().unwrap();
        lock.take().unwrap()
    }
}