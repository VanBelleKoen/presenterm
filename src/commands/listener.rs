use super::{
    dual::DualClient,
    keyboard::{CommandKeyBindings, KeyBindingsValidationError, KeyboardListener},
    speaker_notes::SpeakerNotesEvent,
};
use crate::{config::KeyBindingsConfig, presenter::PresentationError};
use serde::Deserialize;
use std::time::Duration;
use strum::EnumDiscriminants;

/// A command listener that allows polling all command sources in a single place.
pub struct CommandListener {
    keyboard: KeyboardListener,
    dual_client: Option<DualClient>,
}

impl CommandListener {
    /// Create a new command source.
    pub fn new(config: KeyBindingsConfig) -> Result<Self, KeyBindingsValidationError> {
        let bindings = CommandKeyBindings::try_from(config)?;
        Ok(Self { keyboard: KeyboardListener::new(bindings), dual_client: None })
    }

    /// Follow a master presentation.
    ///
    /// The keyboard is restricted to the commands that don't change what is being shown.
    pub fn with_dual_client(mut self, client: DualClient) -> Self {
        self.dual_client = Some(client);
        self
    }

    /// Try to get the next command.
    ///
    /// This attempts to get a command and returns `Ok(None)` on timeout.
    pub(crate) fn try_next_command(&mut self) -> Result<Option<Command>, PresentationError> {
        if let Some(client) = &mut self.dual_client {
            if let Some(msg) = client.try_recv()? {
                return Ok(Some(Self::event_to_command(msg)));
            }
        }
        let command = self.keyboard.poll_next_command(Duration::from_millis(20))?;
        if self.dual_client.is_some() {
            // The master is in charge of navigation.
            return Ok(command.filter(|command| {
                matches!(command, Command::Exit | Command::Redraw | Command::Suspend | Command::CloseModal)
            }));
        }
        Ok(command)
    }

    fn event_to_command(event: SpeakerNotesEvent) -> Command {
        match event {
            SpeakerNotesEvent::GoTo { slide, chunk } => Command::GoToSlideChunk { slide, chunk },
            SpeakerNotesEvent::Exit => Command::Exit,
        }
    }
}

/// A command.
#[derive(Clone, Debug, PartialEq, Eq, EnumDiscriminants)]
#[strum_discriminants(derive(Deserialize))]
pub(crate) enum Command {
    /// Redraw the presentation.
    ///
    /// This can happen on terminal resize.
    Redraw,

    /// Move forward in the presentation.
    Next,

    /// Move to the next slide fast.
    NextFast,

    /// Move backwards in the presentation.
    Previous,

    /// Move to the previous slide fast.
    PreviousFast,

    /// Go to the first slide.
    FirstSlide,

    /// Go to the last slide.
    LastSlide,

    /// Go to one particular slide.
    GoToSlide(u32),

    /// Go to one particular slide + chunk.
    GoToSlideChunk { slide: u32, chunk: u32 },

    /// Render any async render operations in the current slide.
    RenderAsyncOperations,

    /// Exit the presentation.
    Exit,

    /// Suspend the presentation.
    Suspend,

    /// The presentation has changed and needs to be reloaded.
    Reload,

    /// Hard reload the presentation.
    ///
    /// Like [Command::Reload] but also reloads any external resources like images and themes.
    HardReload,

    /// Toggle the slide index view.
    ToggleSlideIndex,

    /// Toggle the key bindings config view.
    ToggleKeyBindingsConfig,

    /// Toggle layout grid.
    ToggleLayoutGrid,

    /// Hide the currently open modal, if any.
    CloseModal,

    /// Skip pauses in the current slide.
    SkipPauses,
}
