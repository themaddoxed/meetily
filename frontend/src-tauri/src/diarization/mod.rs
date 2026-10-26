pub mod alignment;
pub mod audio;
pub mod commands;
pub mod coordinator;
pub mod runtime;
pub mod types;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiarizationMode {
    LivePlusPostCall,
    PostCallOnly,
    Off,
}

impl DiarizationMode {
    pub(crate) const fn as_database_value(self) -> &'static str {
        match self {
            Self::LivePlusPostCall => "live_plus_post_call",
            Self::PostCallOnly => "post_call_only",
            Self::Off => "off",
        }
    }
}

impl TryFrom<&str> for DiarizationMode {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "live_plus_post_call" => Ok(Self::LivePlusPostCall),
            "post_call_only" => Ok(Self::PostCallOnly),
            "off" => Ok(Self::Off),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlapHandling {
    MultipleSpeakers,
}

impl OverlapHandling {
    pub(crate) const fn as_database_value(self) -> &'static str {
        match self {
            Self::MultipleSpeakers => "multiple_speakers",
        }
    }
}

impl TryFrom<&str> for OverlapHandling {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "multiple_speakers" => Ok(Self::MultipleSpeakers),
            _ => Err(()),
        }
    }
}
