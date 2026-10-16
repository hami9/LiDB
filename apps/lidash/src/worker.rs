use lidb_core::Snapshot;
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Maximum userspace grace period for a collector to finish during shutdown.
pub const SHUTDOWN_GRACE: Duration = Duration::from_millis(100);
/// Maximum accepted duration of one interactive collection batch, not its refresh wait.
pub const SAMPLE_TIMEOUT: Duration = Duration::from_secs(5);

static COLLECTOR_ACTIVE: OnceLock<Arc<AtomicBool>> = OnceLock::new();

pub struct Update {
    pub snapshot: Snapshot,
    pub collected_at: Instant,
    pub skipped_updates: u64,
}

#[derive(Default)]
struct Latest {
    update: Option<Update>,
    skipped_updates: u64,
    cancelled: bool,
    collecting_since: Option<Instant>,
    sample_timed_out: bool,
}

// The owner releases admission only after joining the actual worker. A timeout
// latches it closed until process exit, even if the detached worker later returns.
struct Admission {
    active: Arc<AtomicBool>,
    abandoned: bool,
}

impl Admission {
    fn acquire(active: Arc<AtomicBool>) -> io::Result<Self> {
        active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "a collector is already active or abandoned; restart lidash after a timeout",
                )
            })?;
        Ok(Self {
            active,
            abandoned: false,
        })
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        if !self.abandoned {
            self.active.store(false, Ordering::Release);
        }
    }
}

// One replaceable slot bounds retained snapshots, even when rendering is paused.
pub struct Worker {
    latest: Arc<Mutex<Latest>>,
    cancel: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    admission: Option<Admission>,
    shutdown_timed_out: bool,
}

impl Worker {
    /// Start the process's single interactive collector.
    ///
    /// A shutdown timeout permanently closes admission in this process. An
    /// in-flight read cannot safely be killed or replaced with another worker.
    pub fn start(
        sample: impl FnMut() -> Snapshot + Send + 'static,
        interval: Duration,
    ) -> io::Result<Self> {
        let active = COLLECTOR_ACTIVE
            .get_or_init(|| Arc::new(AtomicBool::new(false)))
            .clone();
        Self::start_with_admission(sample, interval, active)
    }

    fn start_with_admission(
        mut sample: impl FnMut() -> Snapshot + Send + 'static,
        interval: Duration,
        active: Arc<AtomicBool>,
    ) -> io::Result<Self> {
        // On spawn failure the local guard releases admission before returning.
        let admission = Admission::acquire(active)?;
        let latest = Arc::new(Mutex::new(Latest::default()));
        let worker_latest = Arc::clone(&latest);
        let (cancel, cancellation) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("lidb-collector".into())
            .spawn(move || loop {
                let began = Instant::now();
                {
                    let Ok(mut slot) = worker_latest.lock() else {
                        break;
                    };
                    if slot.cancelled {
                        break;
                    }
                    slot.collecting_since = Some(began);
                }
                // No mailbox lock is held during the potentially blocking read.
                let snapshot = sample();
                let collected_at = Instant::now();
                let Ok(mut slot) = worker_latest.lock() else {
                    break;
                };
                slot.collecting_since = None;
                // Cancellation and publication share this lock. An admitted
                // in-flight batch may finish its reads but cannot publish later.
                if slot.cancelled {
                    break;
                }
                if collected_at.duration_since(began) >= SAMPLE_TIMEOUT {
                    slot.sample_timed_out = true;
                    slot.cancelled = true;
                    slot.update = None;
                    break;
                }
                if slot.update.is_some() {
                    slot.skipped_updates = slot.skipped_updates.saturating_add(1);
                }
                slot.update = Some(Update {
                    snapshot,
                    collected_at,
                    skipped_updates: slot.skipped_updates,
                });
                drop(slot);
                match cancellation.recv_timeout(interval) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    _ => break,
                }
            })?;
        Ok(Self {
            latest,
            cancel: Some(cancel),
            thread: Some(handle),
            admission: Some(admission),
            shutdown_timed_out: false,
        })
    }

    pub fn take_latest(&self) -> io::Result<Option<Update>> {
        self.latest
            .lock()
            .map(|mut slot| slot.update.take())
            .map_err(|_| io::Error::other("collector mailbox failed"))
    }

    /// Reject an overdue or already discarded batch without inventing source metrics.
    pub fn check_collection(&self) -> io::Result<()> {
        self.check_collection_at(Instant::now())
    }

    fn check_collection_at(&self, now: Instant) -> io::Result<()> {
        let mut slot = self
            .latest
            .lock()
            .map_err(|_| io::Error::other("collector mailbox failed"))?;
        if slot.sample_timed_out
            || slot
                .collecting_since
                .and_then(|began| now.checked_duration_since(began))
                .is_some_and(|elapsed| elapsed >= SAMPLE_TIMEOUT)
        {
            slot.cancelled = true;
            slot.sample_timed_out = true;
            slot.update = None;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "collector sample timed out after 5s; readings unavailable",
            ));
        }
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// Cancel publication, wake the refresh wait and allow a bounded joining grace.
    ///
    /// Only a finished worker is joined. An unfinished worker is detached with
    /// an explicit timeout; repeated shutdown/Drop never repeats the grace wait.
    /// This bounds waiting on a blocked sample, not arbitrary OS thread teardown.
    pub fn shutdown(&mut self) -> io::Result<()> {
        if self.shutdown_timed_out {
            return Err(shutdown_timeout());
        }
        if self.thread.is_none() {
            return Ok(());
        }
        let began = Instant::now();
        {
            // Recover a poisoned mailbox only to stop publication and clean up.
            let mut slot = self
                .latest
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            slot.cancelled = true;
            slot.update = None;
        }
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        loop {
            if self.is_finished() {
                let handle = self.thread.take().expect("finished worker owns a handle");
                let result = handle.join();
                drop(self.admission.take());
                return result.map_err(|_| io::Error::other("collector thread panicked"));
            }
            let remaining = SHUTDOWN_GRACE.saturating_sub(began.elapsed());
            if remaining.is_zero() {
                break;
            }
            thread::sleep(remaining.min(Duration::from_millis(2)));
        }
        self.shutdown_timed_out = true;
        if let Some(admission) = self.admission.as_mut() {
            admission.abandoned = true;
        }
        drop(self.thread.take());
        drop(self.admission.take());
        Err(shutdown_timeout())
    }
}

fn shutdown_timeout() -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        "collector shutdown timed out after 100ms; restart lidash",
    )
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

    fn start(sample: impl FnMut() -> Snapshot + Send + 'static, interval: Duration) -> Worker {
        Worker::start_with_admission(sample, interval, Arc::new(AtomicBool::new(false))).unwrap()
    }

    struct NotifyOnDrop(mpsc::Sender<()>);

    impl Drop for NotifyOnDrop {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    #[test]
    fn blocked_sampler_shutdown_returns_timeout_instead_of_waiting_for_read() {
        let (entered, started) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let (stop_rescue, rescue) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        let finished = NotifyOnDrop(finished);
        let active = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicU64::new(0));
        let sampled = Arc::clone(&calls);
        // Rescue a regressed implementation rather than hanging the test runner.
        let watchdog = thread::spawn(move || {
            let _ = rescue.recv_timeout(Duration::from_secs(2));
            let _ = release.send(());
        });
        let mut worker = Worker::start_with_admission(
            move || {
                let _keep_until_worker_exit = &finished;
                sampled.fetch_add(1, Ordering::SeqCst);
                let _ = entered.send(());
                let _ = blocked.recv();
                Snapshot::new(0, 0, SnapshotMode::Fixture)
            },
            Duration::from_secs(60),
            Arc::clone(&active),
        )
        .unwrap();
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let duplicate = Worker::start_with_admission(
            || Snapshot::new(0, 0, SnapshotMode::Fixture),
            Duration::from_secs(60),
            Arc::clone(&active),
        );
        assert_eq!(duplicate.err().unwrap().kind(), io::ErrorKind::WouldBlock);
        let before = Instant::now();
        let result = worker.shutdown();
        let elapsed = before.elapsed();
        let repeated = Instant::now();
        assert_eq!(
            worker.shutdown().unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert!(repeated.elapsed() < Duration::from_millis(50));
        let _ = stop_rescue.send(());
        watchdog.join().unwrap();
        completion.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(
            elapsed < Duration::from_secs(1),
            "shutdown took {elapsed:?}"
        );
        assert!(worker.take_latest().unwrap().is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Even after the detached thread returns, no replacement may be started.
        let restart = Worker::start_with_admission(
            || Snapshot::new(0, 0, SnapshotMode::Fixture),
            Duration::from_secs(60),
            active,
        );
        assert_eq!(restart.err().unwrap().kind(), io::ErrorKind::WouldBlock);
        let repeated = Instant::now();
        drop(worker);
        assert!(repeated.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn overdue_batch_discards_pending_values_and_blocks_late_publication() {
        let (entered, started) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        let finished = NotifyOnDrop(finished);
        let mut worker = start(
            move || {
                let _keep_until_worker_exit = &finished;
                entered.send(()).unwrap();
                let _ = blocked.recv_timeout(Duration::from_secs(2));
                Snapshot::new(99, 0, SnapshotMode::Fixture)
            },
            Duration::from_secs(60),
        );
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let began = {
            let mut slot = worker.latest.lock().unwrap();
            slot.update = Some(Update {
                snapshot: Snapshot::new(1, 0, SnapshotMode::Fixture),
                collected_at: Instant::now(),
                skipped_updates: 0,
            });
            slot.collecting_since.unwrap()
        };
        worker
            .check_collection_at(began + SAMPLE_TIMEOUT - Duration::from_nanos(1))
            .unwrap();
        assert_eq!(
            worker
                .check_collection_at(began + SAMPLE_TIMEOUT)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert!(worker.take_latest().unwrap().is_none());
        release.send(()).unwrap();
        completion.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(worker.take_latest().unwrap().is_none());
        assert_eq!(
            worker.check_collection().unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        worker.shutdown().unwrap();
    }

    #[test]
    fn unfinished_sampler_destructor_does_not_make_join_unbounded() {
        struct BlockingDrop {
            entered: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Drop for BlockingDrop {
            fn drop(&mut self) {
                let _ = self.entered.send(());
                let _ = self.release.recv_timeout(Duration::from_secs(2));
            }
        }
        let (entered, cleanup) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let guard = BlockingDrop {
            entered,
            release: blocked,
        };
        let (sampled, first) = mpsc::channel();
        let mut worker = start(
            move || {
                let _keep_until_worker_exit = &guard;
                let _ = sampled.send(());
                Snapshot::new(0, 0, SnapshotMode::Fixture)
            },
            Duration::from_secs(60),
        );
        first.recv_timeout(Duration::from_secs(2)).unwrap();
        let began = Instant::now();
        let result = worker.shutdown();
        let elapsed = began.elapsed();
        cleanup.recv_timeout(Duration::from_secs(2)).unwrap();
        let _ = release.send(());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(
            elapsed < Duration::from_secs(1),
            "shutdown took {elapsed:?}"
        );
    }

    #[test]
    fn panic_releases_admission_only_after_join() {
        let active = Arc::new(AtomicBool::new(false));
        let (entered, started) = mpsc::channel();
        let mut worker = Worker::start_with_admission(
            move || {
                entered.send(()).unwrap();
                panic!("injected sampler failure");
            },
            Duration::from_secs(60),
            Arc::clone(&active),
        )
        .unwrap();
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !worker.is_finished() {
            assert!(Instant::now() < deadline, "injected panic did not finish");
            thread::yield_now();
        }
        assert_eq!(worker.shutdown().unwrap_err().kind(), io::ErrorKind::Other);
        let mut replacement = Worker::start_with_admission(
            || Snapshot::new(0, 0, SnapshotMode::Fixture),
            Duration::from_secs(60),
            active,
        )
        .unwrap();
        replacement.shutdown().unwrap();
    }

    #[test]
    fn paused_view_retains_pending_snapshot_for_immediate_resume() {
        let worker = Worker {
            latest: Arc::new(Mutex::new(Latest {
                update: Some(Update {
                    snapshot: Snapshot::new(42, 0, SnapshotMode::Fixture),
                    collected_at: Instant::now(),
                    skipped_updates: 2,
                }),
                skipped_updates: 2,
                ..Latest::default()
            })),
            cancel: None,
            thread: None,
            admission: None,
            shutdown_timed_out: false,
        };
        let mut view = crate::tui::View {
            paused: true,
            ..Default::default()
        };
        crate::tui::update_from_worker(&mut view, &worker).unwrap();
        assert!(view.update.is_none());
        assert!(worker.latest.lock().unwrap().update.is_some());
        view.paused = false;
        crate::tui::update_from_worker(&mut view, &worker).unwrap();
        let update = view.update.unwrap();
        assert_eq!(update.snapshot.monotonic_ns(), 42);
        assert_eq!(update.skipped_updates, 2);
        assert!(worker.latest.lock().unwrap().update.is_none());
    }

    #[test]
    fn newest_snapshot_replaces_pending_and_shutdown_wakes_long_wait() {
        let (observed, ticks) = mpsc::channel();
        let counter = Arc::new(AtomicU64::new(0));
        let collect_counter = Arc::clone(&counter);
        let mut worker = start(
            move || {
                let time = collect_counter.fetch_add(1, Ordering::SeqCst);
                let _ = observed.send(time);
                Snapshot::new(time, 0, SnapshotMode::Fixture)
            },
            Duration::from_millis(1),
        );
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

        let (started, first_sample) = mpsc::channel();
        let mut worker = start(
            move || {
                let _ = started.send(());
                Snapshot::new(0, 0, SnapshotMode::Fixture)
            },
            Duration::from_secs(60),
        );
        first_sample.recv_timeout(Duration::from_secs(2)).unwrap();
        let before = Instant::now();
        worker.shutdown().unwrap();
        assert!(before.elapsed() < Duration::from_secs(2));
    }
}
