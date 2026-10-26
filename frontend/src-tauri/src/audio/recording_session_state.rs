//! Dependency-free Recording Session lifecycle reducer.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordingSessionState {
    Starting,
    Recording,
    Paused,
    Stopping,
    Finalizing,
    Completed,
    Interrupted,
    Failed,
}

impl RecordingSessionState {
    pub fn is_nonterminal(self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Recording | Self::Paused | Self::Stopping | Self::Finalizing
        )
    }

    pub fn as_contract_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Recording => "recording",
            Self::Paused => "paused",
            Self::Stopping => "stopping",
            Self::Finalizing => "finalizing",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordingSessionAction {
    CaptureStarted,
    PauseRequested,
    ResumeRequested,
    StopRequested,
    CaptureOwnerJoined,
    Interrupted,
    FinalizationVerified,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransitionError {
    pub current: RecordingSessionState,
    pub action: RecordingSessionAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartIdentity {
    pub session_id: String,
    pub idempotency_key: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartIdentityDecision {
    New,
    Reuse(String),
    Conflict,
}

pub fn reduce_transition(
    current: RecordingSessionState,
    action: RecordingSessionAction,
) -> Result<RecordingSessionState, TransitionError> {
    use RecordingSessionAction as Action;
    use RecordingSessionState as State;

    let next = match (current, action) {
        (State::Starting, Action::CaptureStarted) => State::Recording,
        (State::Recording, Action::PauseRequested) => State::Paused,
        (State::Paused, Action::ResumeRequested) => State::Recording,
        (State::Paused, Action::PauseRequested) => State::Paused,
        (State::Recording, Action::ResumeRequested) => State::Recording,
        (State::Starting | State::Recording | State::Paused, Action::StopRequested) => {
            State::Stopping
        }
        (
            State::Stopping | State::Finalizing | State::Completed | State::Failed,
            Action::StopRequested,
        ) => current,
        (State::Stopping | State::Interrupted, Action::CaptureOwnerJoined) => State::Finalizing,
        (State::Finalizing, Action::FinalizationVerified) => State::Completed,
        (
            State::Starting
            | State::Recording
            | State::Paused
            | State::Stopping
            | State::Finalizing,
            Action::Interrupted,
        ) => State::Interrupted,
        (
            State::Starting
            | State::Recording
            | State::Paused
            | State::Stopping
            | State::Finalizing
            | State::Interrupted,
            Action::Failed,
        ) => State::Failed,
        _ => return Err(TransitionError { current, action }),
    };

    Ok(next)
}

pub fn decide_start_identity(
    existing: Option<&StartIdentity>,
    idempotency_key: &str,
    fingerprint: &str,
) -> StartIdentityDecision {
    match existing {
        Some(identity) if identity.idempotency_key == idempotency_key => {
            if identity.fingerprint == fingerprint {
                StartIdentityDecision::Reuse(identity.session_id.clone())
            } else {
                StartIdentityDecision::Conflict
            }
        }
        _ => StartIdentityDecision::New,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permits_only_the_recording_session_contract_edges() {
        let legal = [
            (
                RecordingSessionState::Starting,
                RecordingSessionAction::CaptureStarted,
                RecordingSessionState::Recording,
            ),
            (
                RecordingSessionState::Recording,
                RecordingSessionAction::PauseRequested,
                RecordingSessionState::Paused,
            ),
            (
                RecordingSessionState::Paused,
                RecordingSessionAction::ResumeRequested,
                RecordingSessionState::Recording,
            ),
            (
                RecordingSessionState::Starting,
                RecordingSessionAction::StopRequested,
                RecordingSessionState::Stopping,
            ),
            (
                RecordingSessionState::Recording,
                RecordingSessionAction::StopRequested,
                RecordingSessionState::Stopping,
            ),
            (
                RecordingSessionState::Paused,
                RecordingSessionAction::StopRequested,
                RecordingSessionState::Stopping,
            ),
            (
                RecordingSessionState::Stopping,
                RecordingSessionAction::CaptureOwnerJoined,
                RecordingSessionState::Finalizing,
            ),
            (
                RecordingSessionState::Interrupted,
                RecordingSessionAction::CaptureOwnerJoined,
                RecordingSessionState::Finalizing,
            ),
            (
                RecordingSessionState::Finalizing,
                RecordingSessionAction::FinalizationVerified,
                RecordingSessionState::Completed,
            ),
        ];

        for (current, action, expected) in legal {
            assert_eq!(reduce_transition(current, action), Ok(expected));
        }
    }

    #[test]
    fn every_nonterminal_state_can_interrupt_or_fail() {
        for current in [
            RecordingSessionState::Starting,
            RecordingSessionState::Recording,
            RecordingSessionState::Paused,
            RecordingSessionState::Stopping,
            RecordingSessionState::Finalizing,
        ] {
            assert_eq!(
                reduce_transition(current, RecordingSessionAction::Interrupted),
                Ok(RecordingSessionState::Interrupted)
            );
            assert_eq!(
                reduce_transition(current, RecordingSessionAction::Failed),
                Ok(RecordingSessionState::Failed)
            );
        }
    }

    #[test]
    fn duplicate_desired_state_and_stop_actions_are_idempotent() {
        assert_eq!(
            reduce_transition(
                RecordingSessionState::Paused,
                RecordingSessionAction::PauseRequested
            ),
            Ok(RecordingSessionState::Paused)
        );
        assert_eq!(
            reduce_transition(
                RecordingSessionState::Recording,
                RecordingSessionAction::ResumeRequested
            ),
            Ok(RecordingSessionState::Recording)
        );
        for current in [
            RecordingSessionState::Stopping,
            RecordingSessionState::Finalizing,
            RecordingSessionState::Completed,
            RecordingSessionState::Failed,
        ] {
            assert_eq!(
                reduce_transition(current, RecordingSessionAction::StopRequested),
                Ok(current)
            );
        }
    }

    #[test]
    fn capture_acknowledgement_alone_cannot_advance_to_finalizing() {
        assert_eq!(
            reduce_transition(
                RecordingSessionState::Stopping,
                RecordingSessionAction::CaptureStarted
            ),
            Err(TransitionError {
                current: RecordingSessionState::Stopping,
                action: RecordingSessionAction::CaptureStarted,
            })
        );
    }

    #[test]
    fn rejects_illegal_edges_and_all_terminal_mutations() {
        for (current, action) in [
            (
                RecordingSessionState::Starting,
                RecordingSessionAction::PauseRequested,
            ),
            (
                RecordingSessionState::Paused,
                RecordingSessionAction::CaptureStarted,
            ),
            (
                RecordingSessionState::Recording,
                RecordingSessionAction::CaptureOwnerJoined,
            ),
            (
                RecordingSessionState::Completed,
                RecordingSessionAction::Failed,
            ),
            (
                RecordingSessionState::Failed,
                RecordingSessionAction::Interrupted,
            ),
        ] {
            assert_eq!(
                reduce_transition(current, action),
                Err(TransitionError { current, action })
            );
        }
    }

    #[test]
    fn start_identity_reuses_only_the_same_key_and_fingerprint() {
        let existing = StartIdentity {
            session_id: "session-1".into(),
            idempotency_key: "key-1".into(),
            fingerprint: "meeting-a|mic-a|system-a".into(),
        };

        assert_eq!(
            decide_start_identity(Some(&existing), "key-1", "meeting-a|mic-a|system-a"),
            StartIdentityDecision::Reuse("session-1".into())
        );
        assert_eq!(
            decide_start_identity(Some(&existing), "key-1", "meeting-b|mic-a|system-a"),
            StartIdentityDecision::Conflict
        );
        assert_eq!(
            decide_start_identity(Some(&existing), "key-2", "meeting-a|mic-a|system-a"),
            StartIdentityDecision::New
        );
        assert_eq!(
            decide_start_identity(None, "key-1", "any"),
            StartIdentityDecision::New
        );
    }
}
