use std::sync::mpsc::Sender;
use std::sync::mpsc;
use std::thread;

pub struct TaskExecutor<I> {
    sender: Sender<Msg<I>>,
}

impl<I> Clone for TaskExecutor<I> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

struct Msg<I> {
    executor: Box<dyn FnOnce(&mut I)>,
}

unsafe impl<I> Send for Msg<I> {}

impl<I: 'static> TaskExecutor<I> {
    pub fn new<F>(creator: F) -> Self
    where
        F: Send + FnOnce() -> I + 'static,
    {
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
        Self { sender }
    }

    pub fn run<F>(&self, task: F)
    where
        F: Send + FnOnce(&mut I) + 'static,
    {
        {
            self.sender
                .send(Msg {
                    executor: Box::new(move |mut instance| {
                        task(&mut instance);
                    }),
                })
                .unwrap();
        }
    }
}
