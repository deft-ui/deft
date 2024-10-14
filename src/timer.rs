use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};
use skia_safe::wrapper::NativeTransmutableWrapper;
use crate::app::AppEvent;
use crate::event_loop::send_event;

thread_local! {
    pub static TIMER: RefCell<Timer> = RefCell::new(Timer::new());
}


enum Task {
    Timeout(Box<dyn FnOnce()>),
    Interval(
        u64, // interval
        Box<dyn Fn()>,
    ),
}

struct TimeTask {
    id: u64,
    next_execute_time: Instant,
    task: Task,
}

unsafe impl Sync for TimeTask {}

unsafe impl Send for TimeTask {}

impl Eq for TimeTask {}

impl PartialEq<Self> for TimeTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd<Self> for TimeTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.next_execute_time == other.next_execute_time {
            self.id.partial_cmp(&other.id)
        } else {
            self.next_execute_time.partial_cmp(&other.next_execute_time)
        }
    }
}

impl Ord for TimeTask {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.next_execute_time == other.next_execute_time {
            self.id.cmp(&other.id)
        } else {
            self.next_execute_time.cmp(&other.next_execute_time)
        }
    }
}

struct Timer {
    next_task_id: u64,
    tasks: Arc<Mutex<BTreeSet<TimeTask>>>,
    sender: Sender<()>,
}

const DEFAULT_SLEEP_TIME: u64 = 10000000000;

pub struct TimerHandle {
    id: u64,
}

impl TimerHandle {
    fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

}

impl Drop for TimerHandle {
    fn drop(&mut self) {
        remove_time_task(self.id);
    }
}


impl Timer {
    fn new() -> Self {
        let (sender, receiver) = channel();
        let tasks = Arc::new(Mutex::new(BTreeSet::<TimeTask>::new()));
        let tasks_arc = tasks.clone();
        thread::spawn(move || {
            let mut sleep_time = Duration::from_millis(DEFAULT_SLEEP_TIME);
            loop {
                if let Ok(()) = receiver.recv_timeout(sleep_time) {
                    let new_sleep_time = match tasks_arc.lock().unwrap().first() {
                        None => Duration::from_millis(DEFAULT_SLEEP_TIME),
                        Some(t) => t.next_execute_time.duration_since(get_now_time()),
                    };
                    if !new_sleep_time.is_zero() {
                        sleep_time = new_sleep_time;
                    } else {
                        sleep_time = Duration::from_millis(DEFAULT_SLEEP_TIME);
                        send_event(AppEvent::CheckTimer).unwrap();
                    }
                } else {
                    if let Err(_) = send_event(AppEvent::CheckTimer) {
                        break;
                    }
                }
            }
        });
        Self {
            next_task_id: 1,
            tasks,
            sender,
        }
    }
}

pub fn set_timeout<F: FnOnce() + 'static>(callback: F, millis: u64) -> TimerHandle {
    set_timeout_nanos(callback, millis * 1000000)
}

pub fn set_timeout_nanos<F: FnOnce() + 'static>(callback: F, nanos: u64) -> TimerHandle {
    let id = get_next_id();
    let execute_time = get_now_time().add(Duration::from_nanos(nanos));
    add_time_task(TimeTask {
        id,
        next_execute_time: execute_time,
        task: Task::Timeout(Box::new(callback)),
    });
    TimerHandle::new(id)
}

pub fn set_interval<F: Fn() + 'static>(callback: F, interval: u64) -> TimerHandle {
    let id = get_next_id();
    let next_execute_time = get_now_time().add(Duration::from_millis(interval));
    add_time_task(TimeTask {
        id,
        next_execute_time,
        task: Task::Interval(interval, Box::new(callback)),
    });
    TimerHandle::new(id)
}

fn get_next_id() -> u64 {
    TIMER.with_borrow_mut(|t| {
        let id = t.next_task_id;
        t.next_task_id += 1;
        id
    })
}

fn add_time_task(time_task: TimeTask) {
    TIMER.with_borrow_mut(move |t| {
        let mut tasks = t.tasks.lock().unwrap();
        assert_eq!(tasks.insert(time_task), true);
    });
    wakeup_sleep();
}

fn remove_time_task(id: u64) {
    TIMER.with_borrow_mut(move |t| {
        let mut tasks = t.tasks.lock().unwrap();
        tasks.retain(|task| task.id != id);
    });
    wakeup_sleep();
}

pub fn check_task() {
    let task = TIMER.with_borrow_mut(move |t| {
        let now = get_now_time();
        let mut tasks = t.tasks.lock().unwrap();
        let execute = match tasks.first() {
            None => false,
            Some(task) => task.next_execute_time <= now,
        };
        if execute {
            tasks.pop_first()
        } else {
            None
        }
    });
    if let Some(task) = task {
        match task.task {
            Task::Timeout(callback) => {
                callback();
            }
            Task::Interval(interval, callback) => {
                (&callback)();
                let next_execute_time = get_now_time().add(Duration::from_millis(interval));
                add_time_task(TimeTask {
                    id: task.id,
                    next_execute_time,
                    task: Task::Interval(interval, callback),
                });
            }
        }
    }
    wakeup_sleep();
}

fn wakeup_sleep() {
    TIMER.with_borrow(|e| {
        e.sender.send(()).unwrap();
    })
}

fn get_now_time() -> Instant {
    Instant::now()
}