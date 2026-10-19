use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::{Duration, Instant};

pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
}

pub struct EventHandler {
    tick_rate: Duration,
    last_tick: Instant,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
            last_tick: Instant::now(),
        }
    }

    pub fn next_event(&mut self) -> Result<AppEvent, std::io::Error> {
        let elapsed = self.last_tick.elapsed();
        if elapsed >= self.tick_rate {
            self.last_tick = Instant::now();
            return Ok(AppEvent::Tick);
        }

        let timeout = self.tick_rate - elapsed;

        if event::poll(timeout)? {
            if self.last_tick.elapsed() >= self.tick_rate {
                self.last_tick = Instant::now();
                return Ok(AppEvent::Tick);
            }
            match event::read()? {
                CrosstermEvent::Key(key) => Ok(AppEvent::Key(key)),
                CrosstermEvent::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
                _ => Ok(AppEvent::Tick),
            }
        } else {
            self.last_tick = Instant::now();
            Ok(AppEvent::Tick)
        }
    }
}
