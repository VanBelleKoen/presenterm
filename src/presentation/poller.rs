use crate::render::operation::{Pollable, PollableState};
use std::{
    panic::{self, AssertUnwindSafe},
    sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel},
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(25);
const POLLABLE_TIMEOUT: Duration = Duration::from_secs(300);

pub(crate) struct Poller {
    sender: Sender<PollerCommand>,
    receiver: Receiver<PollableEffect>,
}

impl Poller {
    pub(crate) fn launch() -> Self {
        let (command_sender, command_receiver) = channel();
        let (effect_sender, effect_receiver) = channel();
        let worker = PollerWorker::new(command_receiver, effect_sender);
        thread::spawn(move || {
            worker.run();
        });
        Self { sender: command_sender, receiver: effect_receiver }
    }

    pub(crate) fn send(&self, command: PollerCommand) {
        let _ = self.sender.send(command);
    }

    pub(crate) fn next_effect(&mut self) -> Option<PollableEffect> {
        self.receiver.try_recv().ok()
    }
}

/// An effect caused by a pollable.
#[derive(Clone)]
pub(crate) enum PollableEffect {
    /// Refresh the given slide.
    RefreshSlide(usize),

    /// Display an error for the given slide.
    DisplayError { slide: usize, error: String },
}

/// A poller command.
pub(crate) enum PollerCommand {
    /// Start polling a pollable that's positioned in the given slide.
    Poll { pollable: Box<dyn Pollable>, slide: usize },

    /// Reset all pollables.
    Reset,
}

struct PollerWorker {
    receiver: Receiver<PollerCommand>,
    sender: Sender<PollableEffect>,
    pollables: Vec<(Box<dyn Pollable>, usize, Instant)>,
    pollable_timeout: Duration,
}

impl PollerWorker {
    fn new(receiver: Receiver<PollerCommand>, sender: Sender<PollableEffect>) -> Self {
        Self::with_timeout(receiver, sender, POLLABLE_TIMEOUT)
    }

    fn with_timeout(receiver: Receiver<PollerCommand>, sender: Sender<PollableEffect>, timeout: Duration) -> Self {
        Self { receiver, sender, pollables: Default::default(), pollable_timeout: timeout }
    }

    fn run(mut self) {
        loop {
            match self.receiver.recv_timeout(POLL_INTERVAL) {
                Ok(command) => self.process_command(command),
                // TODO don't loop forever.
                Err(RecvTimeoutError::Timeout) => self.poll(),
                Err(RecvTimeoutError::Disconnected) => break,
            };
        }
    }

    fn process_command(&mut self, command: PollerCommand) {
        match command {
            PollerCommand::Poll { mut pollable, slide } => {
                // Poll and only insert if it's still running.
                let started_at = Instant::now();
                match poll_safely(pollable.as_mut()) {
                    PollableState::Unmodified | PollableState::Modified => {
                        self.pollables.push((pollable, slide, started_at));
                    }
                    PollableState::Done => {
                        let _ = self.sender.send(PollableEffect::RefreshSlide(slide));
                    }
                    PollableState::Failed { error } => {
                        let _ = self.sender.send(PollableEffect::DisplayError { slide, error });
                    }
                };
            }
            PollerCommand::Reset => self.pollables.clear(),
        }
    }

    fn poll(&mut self) {
        let mut removables = Vec::new();
        for (index, (pollable, slide, started_at)) in self.pollables.iter_mut().enumerate() {
            let slide = *slide;
            let (effect, remove) = if started_at.elapsed() >= self.pollable_timeout {
                (
                    Some(PollableEffect::DisplayError {
                        slide,
                        error: format!("pollable timed out after {:?}", self.pollable_timeout),
                    }),
                    true,
                )
            } else {
                match poll_safely(pollable.as_mut()) {
                    PollableState::Unmodified => (None, false),
                    PollableState::Modified => (Some(PollableEffect::RefreshSlide(slide)), false),
                    PollableState::Done => (Some(PollableEffect::RefreshSlide(slide)), true),
                    PollableState::Failed { error } => (Some(PollableEffect::DisplayError { slide, error }), true),
                }
            };
            if let Some(effect) = effect {
                let _ = self.sender.send(effect);
            }
            if remove {
                removables.push(index);
            }
        }
        // Walk back and swap remove to avoid invalidating indexes.
        for index in removables.iter().rev() {
            self.pollables.swap_remove(*index);
        }
    }
}

fn poll_safely(pollable: &mut dyn Pollable) -> PollableState {
    match panic::catch_unwind(AssertUnwindSafe(|| pollable.poll())) {
        Ok(state) => state,
        Err(_) => PollableState::Failed { error: "pollable panicked while polling".into() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PanicPollable;

    impl Pollable for PanicPollable {
        fn poll(&mut self) -> PollableState {
            panic!("test panic")
        }
    }

    #[test]
    fn poll_safely_converts_panics_to_failures() {
        let state = poll_safely(&mut PanicPollable);

        assert_eq!(state, PollableState::Failed { error: "pollable panicked while polling".into() });
    }

    #[test]
    fn initial_panicking_pollable_emits_an_error() {
        let (_command_sender, command_receiver) = channel();
        let (effect_sender, effect_receiver) = channel();
        let mut worker = PollerWorker::new(command_receiver, effect_sender);

        worker.process_command(PollerCommand::Poll { pollable: Box::new(PanicPollable), slide: 3 });

        assert!(matches!(
            effect_receiver.try_recv().unwrap(),
            PollableEffect::DisplayError { slide: 3, error } if error == "pollable panicked while polling"
        ));
    }

    #[test]
    fn ongoing_panicking_pollable_emits_an_error_and_is_removed() {
        let (_command_sender, command_receiver) = channel();
        let (effect_sender, effect_receiver) = channel();
        let mut worker = PollerWorker::new(command_receiver, effect_sender);
        worker.pollables.push((Box::new(PanicPollable), 4, Instant::now()));

        worker.poll();

        assert!(matches!(
            effect_receiver.try_recv().unwrap(),
            PollableEffect::DisplayError { slide: 4, error } if error == "pollable panicked while polling"
        ));
        assert!(worker.pollables.is_empty());
    }

    #[test]
    fn ongoing_pollable_times_out_and_is_removed() {
        let (_command_sender, command_receiver) = channel();
        let (effect_sender, effect_receiver) = channel();
        let mut worker = PollerWorker::with_timeout(command_receiver, effect_sender, Duration::ZERO);
        worker.pollables.push((Box::new(PanicPollable), 5, Instant::now()));

        worker.poll();

        assert!(matches!(
            effect_receiver.try_recv().unwrap(),
            PollableEffect::DisplayError { slide: 5, error } if error == "pollable timed out after 0ns"
        ));
        assert!(worker.pollables.is_empty());
    }
}
