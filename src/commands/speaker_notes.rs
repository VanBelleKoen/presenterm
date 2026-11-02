#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SpeakerNotesEvent {
    GoTo { slide: u32, chunk: u32 },
    Exit,
}
