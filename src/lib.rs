use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::{Arc, RwLock};

struct Shared {
    lock: RwLock<()>,
    atomic: AtomicUsize,
}

pub struct DoSomething(Arc<Shared>);

impl DoSomething {
    pub fn do_something(&self) {
        self.0.atomic.load(Ordering::Relaxed);
        drop(self.0.lock.try_read().unwrap());
    }
}

#[derive(Clone)]
pub struct Droppable(Arc<Shared>);

impl Drop for Droppable {
    fn drop(&mut self) {
        let _guard = self.0.lock.write().unwrap();
        self.0.atomic.fetch_sub(1, Ordering::Relaxed);
    }
}

pub fn new() -> (DoSomething, Droppable) {
    let shared = Shared {
        lock: RwLock::new(()),
        atomic: AtomicUsize::new(1),
    };
    let shared = Arc::new(shared);

    (DoSomething(shared.clone()), Droppable(shared))
}

#[test]
fn reproduce() {
    loom::model(|| {
        let (tx, guard1) = new();
        let _guard2 = guard1.clone();

        let th1 = loom::thread::spawn(move || {
            drop(guard1);
        });

        tx.do_something();
        assert!(th1.join().is_ok());
    });
}
