//! Owns connection and protocol tasks until runtime drain completes.
use futures_util::future::{BoxFuture, FutureExt, Shared};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
#[derive(Clone)]
struct Task {
    abort: tokio::task::AbortHandle,
    completed: Shared<BoxFuture<'static, ()>>,
}
#[derive(Default)]
struct State {
    closed: bool,
    tasks: Vec<Task>,
}
/// Supervises runtime-owned connection and protocol futures across transport backends.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct RuntimeTasks(Arc<Mutex<State>>);
impl RuntimeTasks {
    /// Starts an empty task scope.
    pub fn new() -> Self {
        Self::default()
    }
    /// Retains ownership of a spawned future until it completes or shutdown cancels it.
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let mut state = self.0.lock().expect("runtime task scope poisoned");
        if state.closed {
            drop(state);
            drop(future);
            return;
        }
        state.tasks.retain(|task| !task.abort.is_finished());
        let handle = tokio::spawn(future);
        let abort = handle.abort_handle();
        let completed = async move {
            let _ = handle.await;
        }
        .boxed()
        .shared();
        state.tasks.push(Task { abort, completed });
    }
    /// Prevents new tasks from being admitted and snapshots the pending task handles.
    fn close(&self) -> Vec<Task> {
        let mut state = self.0.lock().expect("runtime task scope poisoned");
        state.closed = true;
        state.tasks.clone()
    }
    /// Closes the task scope and awaits natural completion without surrendering ownership.
    pub async fn wait(&self) {
        for task in self.close() {
            task.completed.await;
        }
    }
    /// Closes admission to the task scope, cancels owned futures, and joins them all.
    pub async fn abort_and_wait(&self) {
        let tasks = self.close();
        for task in &tasks {
            task.abort.abort();
        }
        for task in tasks {
            task.completed.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeTasks;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Probe(Arc<AtomicUsize>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[tokio::test]
    async fn cancellation_joins_owned_tasks_before_returning() {
        let tasks = RuntimeTasks::new();
        let drops = Arc::new(AtomicUsize::new(0));
        let probe = Probe(drops.clone());
        let (entered, started) = tokio::sync::oneshot::channel();
        tasks.spawn(async move {
            let _probe = probe;
            entered.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        started.await.unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        tasks.abort_and_wait().await;
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    #[tokio::test(start_paused = true)]
    async fn interrupted_drain_keeps_tasks_owned_for_forced_cancellation() {
        let tasks = RuntimeTasks::new();
        let drops = Arc::new(AtomicUsize::new(0));
        let probe = Probe(drops.clone());
        let (entered, started) = tokio::sync::oneshot::channel();
        tasks.spawn(async move {
            let _probe = probe;
            entered.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        started.await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(1), tasks.wait())
                .await
                .is_err()
        );
        tasks.abort_and_wait().await;
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn closed_supervisor_drops_late_tasks_without_running_them() {
        let tasks = RuntimeTasks::new();
        tasks.abort_and_wait().await;
        let drops = Arc::new(AtomicUsize::new(0));
        let runs = Arc::new(AtomicUsize::new(0));
        let probe = Probe(drops.clone());
        let ran = runs.clone();
        tasks.spawn(async move {
            let _probe = probe;
            ran.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(runs.load(Ordering::SeqCst), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}

/// Hyper executor retaining protocol futures in the shared runtime task scope.
#[doc(hidden)]
#[derive(Clone)]
pub struct RuntimeExecutor(pub RuntimeTasks);
#[cfg(feature = "http")]
impl<F> hyper::rt::Executor<F> for RuntimeExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    /// Runs a Hyper protocol future inside the shared runtime task scope.
    fn execute(&self, future: F) {
        self.0.spawn(async move {
            let _ = future.await;
        });
    }
}
