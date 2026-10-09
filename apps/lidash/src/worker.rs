use lidb_core::Snapshot;
use std::{
    io,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct Update {
    pub snapshot: Snapshot,
    pub collected_at: Instant,
    pub skipped_updates: u64,
}

#[derive(Default)]
struct Latest {
    update: Option<Update>,
    skipped_updates: u64,
}

// One replaceable slot bounds retained snapshots, even when rendering is paused.
pub struct Worker {
    latest: Arc<Mutex<Latest>>,
    cancel: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(
        mut sample: impl FnMut() -> Snapshot + Send + 'static,
        interval: Duration,
    ) -> io::Result<Self> {
        let latest = Arc::new(Mutex::new(Latest::default()));
        let worker_latest = Arc::clone(&latest);
        let (cancel, cancellation) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("lidb-collector".into())
            .spawn(move || {
                loop {
                    let snapshot = sample();
                    let collected_at = Instant::now();
                    let Ok(mut slot) = worker_latest.lock() else {
                        break;
                    };
                    if slot.update.is_some() {
                        slot.skipped_updates = slot.skipped_updates.saturating_add(1);
                    }
                    slot.update = Some(Update {
                        snapshot,
                        collected_at,
                        skipped_updates: slot.skipped_updates,
                    });
                    drop(slot);
                    // Cancellation wakes this wait immediately; shutdown never waits out an interval.
                    match cancellation.recv_timeout(interval) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        _ => break,
                    }
                }
            })?;
        Ok(Self {
            latest,
            cancel: Some(cancel),
            thread: Some(thread),
        })
    }

    pub fn take_latest(&self) -> io::Result<Option<Update>> {
        self.latest
            .lock()
            .map(|mut slot| slot.update.take())
            .map_err(|_| io::Error::other("collector mailbox failed"))
    }

    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_some_and(JoinHandle::is_finished)
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| io::Error::other("collector thread panicked"))?;
        }
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::SnapshotMode;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn newest_snapshot_replaces_pending_and_shutdown_wakes_long_wait() {
        let (observed, ticks) = mpsc::channel();
        let counter = Arc::new(AtomicU64::new(0));
        let collect_counter = Arc::clone(&counter);
        let mut worker = Worker::start(
            move || {
                let time = collect_counter.fetch_add(1, Ordering::SeqCst);
                let _ = observed.send(time);
                Snapshot::new(time, 0, SnapshotMode::Fixture)
            },
            Duration::from_millis(1),
        )
        .unwrap();
        for _ in 0..3 {
            ticks.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        // Lock acquisition waits for the latest publication, rather than assuming send timing.
        let deadline = Instant::now() + Duration::from_secs(2);
        let update = loop {
            if let Some(update) = worker.take_latest().unwrap() {
                if update.snapshot.monotonic_ns() >= 2 {
                    break update;
                }
            }
            assert!(Instant::now() < deadline);
            thread::yield_now();
        };
        assert!(update.skipped_updates > 0);
        worker.shutdown().unwrap();

        let (started, start) = mpsc::channel();
        let mut worker = Worker::start(
            move || {
                let _ = started.send(());
                Snapshot::new(0, 0, SnapshotMode::Fixture)
            },
            Duration::from_secs(60),
        )
        .unwrap();
        start.recv_timeout(Duration::from_secs(2)).unwrap();
        let before = Instant::now();
        worker.shutdown().unwrap();
        assert!(before.elapsed() < Duration::from_secs(2));
    }
}
