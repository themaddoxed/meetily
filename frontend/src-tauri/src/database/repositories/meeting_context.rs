use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingQuestionContext {
    pub meeting_id: String,
    pub title: String,
    pub sources: Vec<MeetingContextSource>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingContextSource {
    pub transcript_id: String,
    pub text: String,
    pub timestamp: String,
    pub audio_start_time: Option<f64>,
    pub href: String,
}

/// Read a single-meeting snapshot. No global search, model call or persistence.
pub struct MeetingContextRepository;

impl MeetingContextRepository {
    pub async fn read(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<MeetingQuestionContext, sqlx::Error> {
        if meeting_id.trim().is_empty() {
            return Err(sqlx::Error::Protocol("meeting_id cannot be empty".into()));
        }
        let mut transaction = pool.begin().await?;
        let title: String = sqlx::query_scalar("SELECT title FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_one(&mut *transaction)
            .await?;
        let rows = sqlx::query_as::<_, (String, String, String, Option<f64>)>(
            "SELECT id, transcript, timestamp, audio_start_time
             FROM transcripts WHERE meeting_id = ?
             ORDER BY audio_start_time ASC, timestamp ASC, id ASC",
        )
        .bind(meeting_id)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;

        let sources = rows
            .into_iter()
            .filter(|(_, text, _, _)| !text.trim().is_empty())
            .map(|(transcript_id, text, timestamp, start)| {
                let audio_start_time = start.filter(|value| value.is_finite() && *value >= 0.0);
                let mut query = url::form_urlencoded::Serializer::new(String::new());
                query.append_pair("id", meeting_id);
                query.append_pair("source", "question");
                query.append_pair("transcriptId", &transcript_id);
                if let Some(time) = audio_start_time {
                    query.append_pair("at", &time.to_string());
                }
                MeetingContextSource {
                    transcript_id,
                    text,
                    timestamp,
                    audio_start_time,
                    href: format!("/meeting-details?{}", query.finish()),
                }
            })
            .collect();
        Ok(MeetingQuestionContext {
            meeting_id: meeting_id.to_owned(),
            title,
            sources,
        })
    }
}
