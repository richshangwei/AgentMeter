use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub struct CollectorLifecycle {
    state: Mutex<(bool, bool)>, // stopping, running
    finished: Condvar,
    pub cancelled: AtomicBool,
}
pub struct Permit(Arc<CollectorLifecycle>);
impl CollectorLifecycle {
    pub fn is_running(&self) -> bool {
        self.state.lock().map(|state| state.1).unwrap_or(false)
    }
    pub fn start(self: &Arc<Self>) -> Result<Permit, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "collector lifecycle unavailable")?;
        if state.0 || state.1 {
            return Err("collector stopped or busy".into());
        }
        state.1 = true;
        Ok(Permit(Arc::clone(self)))
    }
    pub fn stop_and_wait(&self) {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.cancelled.store(true, Ordering::Release);
        while state.1 {
            state = self.finished.wait(state).unwrap();
        }
    }
}
impl Permit {
    pub fn cancelled(&self) -> &AtomicBool {
        &self.0.cancelled
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.state.lock().unwrap().1 = false;
        self.0.finished.notify_all();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_cancels_running_work_waits_for_release_and_rejects_new_work() {
        let lifecycle = Arc::new(CollectorLifecycle::default());
        let permit = lifecycle.start().unwrap();
        assert!(lifecycle.start().is_err());
        let stopped = Arc::new(AtomicBool::new(false));
        let worker = {
            let lifecycle = Arc::clone(&lifecycle);
            let stopped = Arc::clone(&stopped);
            std::thread::spawn(move || {
                lifecycle.stop_and_wait();
                stopped.store(true, Ordering::Release);
            })
        };
        while !permit.cancelled().load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        assert!(!stopped.load(Ordering::Acquire));
        drop(permit);
        worker.join().unwrap();
        assert!(stopped.load(Ordering::Acquire));
        assert!(lifecycle.start().is_err());
    }
}
