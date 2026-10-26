use crate::api::TranscriptSearchResult;
use serde_json::{Map, Value};
use sqlx::SqlitePool;

/// Owns the durable, user-visible meeting search index.
///
/// Search content is deliberately derived from the source tables instead of
/// indexing arbitrary JSON. This keeps caches, model metadata, and hidden
/// translations out of the user-facing search surface.
pub struct SearchRepository;

impl SearchRepository {
    /// Recreate the index from current meeting data.
    ///
    /// Rebuilding in one transaction makes a reopen or an imported database
    /// deterministic and means a query never observes a half-built index.
    pub async fn rebuild(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let mut transaction = pool.begin().await?;

        Self::rebuild_in_transaction(&mut transaction).await?;
        transaction.commit().await
    }

    /// Refresh search inside a source-data transaction so accepted summaries
    /// and their index become visible together.
    pub(crate) async fn rebuild_in_transaction(
        connection: &mut sqlx::SqliteConnection,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM meeting_search_index")
            .execute(&mut *connection)
            .await?;

        let meetings = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, title, created_at FROM meetings ORDER BY created_at DESC",
        )
        .fetch_all(&mut *connection)
        .await?;

        for (meeting_id, title, created_at) in meetings {
            sqlx::query(
                "INSERT INTO meeting_search_index
                    (meeting_id, source_type, source_id, content, timestamp, audio_start_time)
                 VALUES (?, 'title', NULL, ?, ?, NULL)",
            )
            .bind(&meeting_id)
            .bind(&title)
            .bind(&created_at)
            .execute(&mut *connection)
            .await?;
        }

        let transcripts = sqlx::query_as::<_, (String, String, String, String, Option<f64>)>(
            "SELECT meeting_id, id, transcript, timestamp, audio_start_time
             FROM transcripts
             ORDER BY meeting_id, audio_start_time ASC, timestamp ASC, id ASC",
        )
        .fetch_all(&mut *connection)
        .await?;

        for (meeting_id, transcript_id, transcript, timestamp, audio_start_time) in transcripts {
            if transcript.trim().is_empty() {
                continue;
            }

            sqlx::query(
                "INSERT INTO meeting_search_index
                    (meeting_id, source_type, source_id, content, timestamp, audio_start_time)
                 VALUES (?, 'transcript', ?, ?, ?, ?)",
            )
            .bind(&meeting_id)
            .bind(&transcript_id)
            .bind(&transcript)
            .bind(&timestamp)
            .bind(audio_start_time)
            .execute(&mut *connection)
            .await?;
        }

        let summaries = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT p.meeting_id, p.updated_at, p.result
             FROM summary_processes p
             JOIN meetings m ON m.id = p.meeting_id
             ORDER BY p.updated_at DESC",
        )
        .fetch_all(&mut *connection)
        .await?;

        for (meeting_id, timestamp, result) in summaries {
            let Some(summary) = result.as_deref().and_then(visible_summary_text) else {
                continue;
            };

            sqlx::query(
                "INSERT INTO meeting_search_index
                    (meeting_id, source_type, source_id, content, timestamp, audio_start_time)
                 VALUES (?, 'summary', NULL, ?, ?, NULL)",
            )
            .bind(&meeting_id)
            .bind(summary)
            .bind(&timestamp)
            .execute(&mut *connection)
            .await?;
        }

        Ok(())
    }

    /// Search the rebuilt index and return source-aware snippets.
    pub async fn search(
        pool: &SqlitePool,
        query: &str,
    ) -> Result<Vec<TranscriptSearchResult>, sqlx::Error> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // Rebuild on every query to cover legacy imports and maintenance paths
        // that write the source tables directly. The rebuild is transactional.
        Self::rebuild(pool).await?;

        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                String,
                String,
                Option<f64>,
            ),
        >(
            "SELECT i.meeting_id, m.title, i.source_type, i.source_id,
                    i.content, i.timestamp, i.audio_start_time
             FROM meeting_search_index i
             JOIN meetings m ON m.id = i.meeting_id
             ORDER BY m.created_at DESC, i.id ASC",
        )
        .fetch_all(pool)
        .await?;

        let query_lower = query.to_lowercase();
        Ok(rows
            .into_iter()
            .filter_map(
                |(id, title, source_type, source_id, content, timestamp, audio_start_time)| {
                    if !content.to_lowercase().contains(&query_lower) {
                        return None;
                    }

                    Some(TranscriptSearchResult {
                        id,
                        title,
                        match_context: match_context(&content, query),
                        timestamp,
                        source_type,
                        source_id,
                        audio_start_time,
                    })
                },
            )
            .collect())
    }
}

/// Extract only the summary content rendered by the frontend.
///
/// Supported stored formats are markdown, BlockNote blocks, and the legacy
/// section/block shape. Fields such as `english_cache`, `_section_order`, and
/// arbitrary metadata are intentionally ignored.
pub fn visible_summary_text(raw: &str) -> Option<String> {
    let mut value = serde_json::from_str::<Value>(raw).ok()?;
    for _ in 0..2 {
        if let Value::String(encoded) = value {
            value = serde_json::from_str(&encoded).ok()?;
        } else {
            break;
        }
    }

    let object = value.as_object()?;

    if let Some(blocks) = object.get("summary_json").and_then(Value::as_array) {
        if blocks.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        if !append_blocknote_blocks(blocks, &mut parts) {
            return None;
        }
        if !parts.is_empty() {
            return Some(parts.join("\n"));
        }
        return None;
    }

    if let Some(markdown) = object.get("markdown").and_then(Value::as_str) {
        if let Some(markdown) = visible_markdown_text(markdown) {
            return Some(markdown);
        }
    }

    visible_legacy_summary_text(object)
}

/// Convert markdown to the text that a reader sees in the BlockNote view.
///
/// The frontend parses markdown into blocks before rendering it. Indexing the
/// stored source directly would therefore make formatting markers searchable
/// and would expose link destinations that are not shown as text. This small
/// parser deliberately keeps the user-visible labels/content while dropping
/// markdown structure and URLs. It is intentionally conservative: unknown
/// syntax remains text instead of making a summary disappear from search.
fn visible_markdown_text(markdown: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut in_fence = false;

    for raw_line in markdown.lines() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            let text = normalize_markdown_line(trimmed);
            if !text.is_empty() {
                lines.push(text);
            }
            continue;
        }

        let line = strip_markdown_block_prefix(trimmed);
        if line.is_empty() || is_horizontal_rule(line) || is_reference_definition(line) {
            continue;
        }

        let text = normalize_markdown_line(&visible_inline_text(line));
        if !text.is_empty() {
            lines.push(text);
        }
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn is_reference_definition(line: &str) -> bool {
    let characters = line.chars().collect::<Vec<_>>();
    find_closing_delimiter(&characters, 0, '[', ']')
        .is_some_and(|label_end| characters.get(label_end + 1) == Some(&':'))
}

fn strip_markdown_block_prefix(mut line: &str) -> &str {
    loop {
        let original = line;
        let after_quote = line.strip_prefix('>').map(str::trim_start);
        if let Some(after_quote) = after_quote {
            line = after_quote;
            continue;
        }

        let heading_end = line
            .as_bytes()
            .iter()
            .take(6)
            .position(|byte| *byte != b'#');
        if let Some(count) = heading_end {
            if count > 0 && line.as_bytes().get(count) == Some(&b' ') {
                line = line[count..].trim_start();
                continue;
            }
        } else if line.len() >= 2 && line.as_bytes().get(6) == Some(&b' ') {
            line = line[6..].trim_start();
            continue;
        }

        if let Some(after_bullet) = ["- ", "+ ", "* "]
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix))
        {
            line = after_bullet.trim_start();
            continue;
        }

        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0
            && matches!(line.as_bytes().get(digits), Some(b'.' | b')'))
            && line.as_bytes().get(digits + 1) == Some(&b' ')
        {
            line = line[digits + 1..].trim_start();
            continue;
        }

        if let Some(after_checkbox) = line
            .strip_prefix("[ ] ")
            .or_else(|| line.strip_prefix("[x] "))
            .or_else(|| line.strip_prefix("[X] "))
        {
            line = after_checkbox.trim_start();
            continue;
        }

        if line == original {
            return line;
        }
    }
}

fn is_horizontal_rule(line: &str) -> bool {
    let line = line.trim();
    line.len() >= 3
        && line
            .chars()
            .all(|character| matches!(character, '-' | '_' | '*' | ' '))
}

fn normalize_markdown_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn visible_inline_text(input: &str) -> String {
    let characters = input.chars().collect::<Vec<_>>();
    visible_inline_characters(&characters)
}

fn visible_inline_characters(characters: &[char]) -> String {
    let mut output = String::new();
    let mut index = 0;

    while index < characters.len() {
        if characters[index] == '\\' {
            if let Some(next) = characters.get(index + 1) {
                output.push(*next);
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if characters[index] == '!' && characters.get(index + 1) == Some(&'[') {
            if let Some(label_end) = find_closing_delimiter(characters, index + 1, '[', ']') {
                if characters.get(label_end + 1) == Some(&'(') {
                    if let Some(destination_end) =
                        find_closing_parenthesis(characters, label_end + 2)
                    {
                        // Images render as an image, not as their alt text in
                        // the summary editor, so do not index the alt text.
                        index = destination_end + 1;
                        continue;
                    }
                }
            }
        }

        if characters[index] == '[' {
            if let Some(label_end) = find_closing_delimiter(characters, index, '[', ']') {
                if characters.get(label_end + 1) == Some(&'(') {
                    if let Some(destination_end) =
                        find_closing_parenthesis(characters, label_end + 2)
                    {
                        output.push_str(&visible_inline_characters(
                            &characters[index + 1..label_end],
                        ));
                        index = destination_end + 1;
                        continue;
                    }
                }
                if characters.get(label_end + 1) == Some(&'[') {
                    if let Some(reference_end) =
                        find_closing_delimiter(characters, label_end + 1, '[', ']')
                    {
                        output.push_str(&visible_inline_characters(
                            &characters[index + 1..label_end],
                        ));
                        index = reference_end + 1;
                        continue;
                    }
                }
            }
            output.push('[');
            index += 1;
            continue;
        }

        if characters[index] == '<' {
            if let Some(tag_end) = characters[index + 1..]
                .iter()
                .position(|character| *character == '>')
            {
                let tag_end = index + 1 + tag_end;
                let tag = characters[index + 1..tag_end].iter().collect::<String>();
                if tag.starts_with("http://")
                    || tag.starts_with("https://")
                    || tag.starts_with("mailto:")
                {
                    // Markdown autolinks display their destination as the
                    // link label, unlike destinations in `[label](url)`.
                    output.push_str(&tag);
                    index = tag_end + 1;
                    continue;
                }
                if tag.starts_with('/')
                    || tag
                        .chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_alphabetic())
                    || tag.starts_with('!')
                {
                    index = tag_end + 1;
                    continue;
                }
            }
        }

        let code_marker = 96 as char;
        if characters[index] == code_marker {
            let delimiter_len = character_run_len(characters, index, code_marker);
            if let Some(code_end) = find_matching_run(
                characters,
                index + delimiter_len,
                code_marker,
                delimiter_len,
            ) {
                output.extend(characters[index + delimiter_len..code_end].iter().copied());
                index = code_end + delimiter_len;
                continue;
            }
            output.extend(std::iter::repeat(code_marker).take(delimiter_len));
            index += delimiter_len;
            continue;
        }

        let marker = characters[index];
        if matches!(marker, '*' | '_' | '~') {
            let delimiter_len = character_run_len(characters, index, marker);
            if delimiter_len > 0 && can_open_emphasis(characters, index, delimiter_len, marker) {
                if let Some(close_start) =
                    find_closing_emphasis(characters, index + delimiter_len, marker, delimiter_len)
                {
                    output.push_str(&visible_inline_characters(
                        &characters[index + delimiter_len..close_start],
                    ));
                    index = close_start + delimiter_len;
                    continue;
                }
            }
            output.extend(std::iter::repeat(marker).take(delimiter_len));
            index += delimiter_len;
            continue;
        }

        match characters[index] {
            '[' | ']' => {
                output.push(characters[index]);
                index += 1;
            }
            '|' => {
                output.push(' ');
                index += 1;
            }
            '<' => {
                output.push('<');
                index += 1;
            }
            _ => {
                output.push(characters[index]);
                index += 1;
            }
        }
    }

    output
}

fn character_run_len(characters: &[char], start: usize, character: char) -> usize {
    characters[start..]
        .iter()
        .take_while(|candidate| **candidate == character)
        .count()
}

fn find_matching_run(
    characters: &[char],
    start: usize,
    marker: char,
    run_len: usize,
) -> Option<usize> {
    let escape_marker = 92 as char;
    let mut index = start;
    while index < characters.len() {
        if characters[index] == escape_marker {
            index = (index + 2).min(characters.len());
        } else if characters[index] == marker {
            let candidate_len = character_run_len(characters, index, marker);
            if candidate_len == run_len {
                return Some(index);
            }
            index += candidate_len;
        } else {
            index += 1;
        }
    }
    None
}

fn can_open_emphasis(characters: &[char], start: usize, run_len: usize, marker: char) -> bool {
    if marker == '~' && run_len < 2 {
        return false;
    }
    let Some(next) = characters.get(start + run_len) else {
        return false;
    };
    if next.is_whitespace() {
        return false;
    }
    if marker == '_' {
        return !is_intraword_underscore(characters, start, run_len);
    }
    true
}

fn is_intraword_underscore(characters: &[char], start: usize, run_len: usize) -> bool {
    let previous_is_word = start
        .checked_sub(1)
        .and_then(|index| characters.get(index))
        .is_some_and(|previous| previous.is_alphanumeric());
    let next_is_word = characters
        .get(start + run_len)
        .is_some_and(|next| next.is_alphanumeric());
    previous_is_word && next_is_word
}

fn can_close_emphasis(characters: &[char], start: usize, run_len: usize, marker: char) -> bool {
    if marker == '~' && run_len < 2 {
        return false;
    }
    let Some(previous) = start.checked_sub(1).and_then(|index| characters.get(index)) else {
        return false;
    };
    if previous.is_whitespace() {
        return false;
    }
    if marker == '_' {
        return !is_intraword_underscore(characters, start, run_len);
    }
    true
}

fn find_closing_emphasis(
    characters: &[char],
    start: usize,
    marker: char,
    run_len: usize,
) -> Option<usize> {
    let escape_marker = 92 as char;
    let code_marker = 96 as char;
    let mut index = start;
    while index < characters.len() {
        if characters[index] == escape_marker {
            index = (index + 2).min(characters.len());
            continue;
        }
        if characters[index] == code_marker {
            let code_delimiter_len = character_run_len(characters, index, code_marker);
            if let Some(code_end) = find_matching_run(
                characters,
                index + code_delimiter_len,
                code_marker,
                code_delimiter_len,
            ) {
                index = code_end + code_delimiter_len;
            } else {
                index += code_delimiter_len;
            }
            continue;
        }
        if characters[index] == marker {
            let candidate_len = character_run_len(characters, index, marker);
            if candidate_len == run_len && can_close_emphasis(characters, index, run_len, marker) {
                return Some(index);
            }
            index += candidate_len;
            continue;
        }
        index += 1;
    }
    None
}

fn find_closing_delimiter(
    characters: &[char],
    start: usize,
    opening: char,
    closing: char,
) -> Option<usize> {
    let mut depth = 0;
    for (index, character) in characters.iter().enumerate().skip(start) {
        if *character == opening {
            depth += 1;
        } else if *character == closing {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn find_closing_parenthesis(characters: &[char], start: usize) -> Option<usize> {
    let mut depth = 0;
    for (index, character) in characters.iter().enumerate().skip(start) {
        match character {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn append_blocknote_blocks(blocks: &[Value], output: &mut Vec<String>) -> bool {
    for block in blocks {
        if !append_blocknote_block(block, output) {
            return false;
        }
    }
    true
}

fn append_blocknote_block(value: &Value, output: &mut Vec<String>) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.get("id").and_then(Value::as_str).is_none()
        || object.get("type").and_then(Value::as_str).is_none()
        || object.get("props").is_some_and(|props| !props.is_object())
    {
        return false;
    }

    if let Some(content) = object.get("content") {
        let mut inline_parts = Vec::new();
        append_inline_content(content, &mut inline_parts);
        let text = inline_parts.concat();
        if !text.trim().is_empty() {
            output.push(text.trim().to_string());
        }
    }
    if let Some(children) = object.get("children") {
        let Some(children) = children.as_array() else {
            return false;
        };
        if !append_blocknote_blocks(children, output) {
            return false;
        }
    }

    true
}

fn append_inline_content(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(text) if !text.is_empty() => output.push(text.to_string()),
        Value::Array(values) => {
            for value in values {
                append_inline_content(value, output);
            }
        }
        Value::Object(object) => {
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                if !text.is_empty() {
                    output.push(text.to_string());
                }
                return;
            }

            // BlockNote links keep their visible label in `content` and the
            // destination in `href`. Recurse only into content so the hidden
            // destination and any other props never enter the index.
            if object.get("type").and_then(Value::as_str) == Some("tableContent") {
                if let Some(rows) = object.get("rows").and_then(Value::as_array) {
                    for (row_index, row) in rows.iter().enumerate() {
                        if row_index > 0 {
                            output.push("\n".to_string());
                        }
                        if let Some(cells) = row.get("cells").and_then(Value::as_array) {
                            for (cell_index, cell) in cells.iter().enumerate() {
                                if cell_index > 0 {
                                    output.push(" ".to_string());
                                }
                                append_inline_content(cell, output);
                            }
                        }
                    }
                }
                return;
            }

            if let Some(content) = object.get("content") {
                append_inline_content(content, output);
            }
        }
        _ => {}
    }
}

fn append_legacy_section(key: &str, value: &Value, output: &mut Vec<String>) -> Option<bool> {
    let object = value.as_object()?;

    if object.get("title").is_some_and(|title| !title.is_string()) {
        return None;
    }
    let blocks = object.get("blocks")?.as_array()?;

    if let Some(title) = object.get("title").and_then(Value::as_str) {
        if !title.trim().is_empty() {
            output.push(title.trim().to_string());
        }
    } else if !key.is_empty() && !key.starts_with('_') {
        output.push(key.to_string());
    }

    for block in blocks {
        let block = block.as_object()?;
        if block.get("id").and_then(Value::as_str).is_none()
            || block.get("type").and_then(Value::as_str).is_none()
            || block.get("content").and_then(Value::as_str).is_none()
        {
            return None;
        }
        if let Some(content) = block.get("content").and_then(Value::as_str) {
            if !content.trim().is_empty() {
                output.push(content.trim().to_string());
            }
        }
    }

    Some(!blocks.is_empty())
}

fn visible_legacy_summary_text(object: &Map<String, Value>) -> Option<String> {
    if object
        .get("MeetingName")
        .is_some_and(|value| !value.is_string())
    {
        return None;
    }
    if let Some(order) = object.get("_section_order") {
        let order = order.as_array()?;
        if order.iter().any(|key| key.as_str().is_none()) {
            return None;
        }
    }

    let meeting_notes_metadata = object
        .get("MeetingNotes")
        .and_then(Value::as_object)
        .filter(|meeting_notes| {
            !meeting_notes.contains_key("title")
                && !meeting_notes.contains_key("blocks")
                && (meeting_notes.contains_key("meeting_name")
                    || meeting_notes.contains_key("sections"))
        });

    if let Some(meeting_notes) = meeting_notes_metadata {
        if !valid_meeting_notes_metadata(meeting_notes) {
            return None;
        }

        let direct_keys = legacy_section_keys(object, true);
        if !direct_keys.is_empty() {
            return collect_legacy_sections(object, &direct_keys);
        }

        let sections = meeting_notes.get("sections")?.as_array()?;
        return collect_legacy_section_values(sections);
    }

    let section_keys = legacy_section_keys(object, false);
    collect_legacy_sections(object, &section_keys)
}

fn valid_meeting_notes_metadata(object: &Map<String, Value>) -> bool {
    if object
        .keys()
        .any(|key| key != "meeting_name" && key != "sections")
        || object
            .get("meeting_name")
            .is_some_and(|value| !value.is_string())
    {
        return false;
    }

    let Some(sections) = object.get("sections").and_then(Value::as_array) else {
        return false;
    };
    sections.iter().all(|section| {
        let Some(section_object) = section.as_object() else {
            return false;
        };
        section_object
            .get("title")
            .and_then(Value::as_str)
            .is_some()
            && section_object
                .keys()
                .all(|key| key == "title" || key == "blocks")
            && append_legacy_section("", section, &mut Vec::new()).is_some()
    })
}

fn legacy_section_keys(object: &Map<String, Value>, ignore_meeting_notes: bool) -> Vec<String> {
    if let Some(order) = object.get("_section_order").and_then(Value::as_array) {
        return order
            .iter()
            .filter_map(Value::as_str)
            .filter(|key| !ignore_meeting_notes || *key != "MeetingNotes")
            .map(str::to_string)
            .collect();
    }

    object
        .keys()
        .filter(|key| {
            *key != "MeetingName"
                && *key != "_section_order"
                && (!ignore_meeting_notes || *key != "MeetingNotes")
        })
        .cloned()
        .collect()
}

fn collect_legacy_sections(object: &Map<String, Value>, keys: &[String]) -> Option<String> {
    if keys.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut has_blocks = false;
    for key in keys {
        let section = object.get(key)?;
        has_blocks |= append_legacy_section(key, section, &mut parts)?;
    }

    (has_blocks && !parts.is_empty()).then(|| parts.join("\n"))
}

fn collect_legacy_section_values(values: &[Value]) -> Option<String> {
    let mut parts = Vec::new();
    let mut has_blocks = false;
    for value in values {
        has_blocks |= append_legacy_section("", value, &mut parts)?;
    }

    (has_blocks && !parts.is_empty()).then(|| parts.join("\n"))
}

fn match_context(content: &str, query: &str) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();
    let Some(match_index) = content_lower.find(&query_lower) else {
        return content.chars().take(240).collect();
    };

    // `find` returns a byte offset. Clamp the range to UTF-8 boundaries before
    // slicing the original string; this keeps snippets safe for Cyrillic and
    // other multibyte meeting text.
    let start = match_index.saturating_sub(100);
    let end = (match_index + query.len() + 100).min(content.len());
    let start = (0..=start)
        .rev()
        .find(|offset| content.is_char_boundary(*offset))
        .unwrap_or(0);
    let end = (end..=content.len())
        .find(|offset| content.is_char_boundary(*offset))
        .unwrap_or(content.len());

    let mut context = String::new();
    if start > 0 {
        context.push_str("...");
    }
    context.push_str(&content[start..end]);
    if end < content.len() {
        context.push_str("...");
    }
    context
}

#[cfg(test)]
mod tests {
    use super::visible_summary_text;

    #[test]
    fn visible_summary_ignores_internal_cache_and_metadata() {
        let raw = serde_json::json!({
            "markdown": "Visible decision",
            "english_cache": { "markdown": "Hidden translation" },
            "internal_json": "Do not index me"
        })
        .to_string();

        let visible = visible_summary_text(&raw).unwrap();
        assert_eq!(visible, "Visible decision");
    }

    #[test]
    fn visible_summary_reads_blocknote_text_without_indexing_block_metadata() {
        let raw = serde_json::json!({
            "summary_json": [{
                "id": "block-1",
                "type": "heading",
                "content": [
                    {"type": "text", "text": "Visible ", "styles": {}},
                    {"type": "text", "text": "heading", "styles": {}}
                ],
                "props": {"backgroundColor": "secret metadata"},
                "children": []
            }],
            "english_cache": {"markdown": "Hidden translation"}
        })
        .to_string();

        let visible = visible_summary_text(&raw).unwrap();
        assert_eq!(visible, "Visible heading");
    }

    #[test]
    fn visible_summary_does_not_fallback_to_stale_markdown_when_blocks_are_empty() {
        let raw = serde_json::json!({
            "summary_json": [{
                "id": "block-1",
                "type": "paragraph",
                "content": [],
                "children": []
            }],
            "markdown": "Hidden stale markdown"
        })
        .to_string();

        assert!(visible_summary_text(&raw).is_none());
    }

    #[test]
    fn visible_summary_reads_nested_links_and_table_cells_without_destinations() {
        let raw = serde_json::json!({
            "summary_json": [{
                "id": "table-1",
                "type": "table",
                "props": {},
                "content": {
                    "type": "tableContent",
                    "columnWidths": [],
                    "rows": [{
                        "cells": [
                            [
                                {"type": "text", "text": "Visible ", "styles": {}},
                                {"type": "link", "href": "https://hidden.example/secret", "content": [
                                    {"type": "text", "text": "launch", "styles": {}}
                                ]}
                            ],
                            [{"type": "text", "text": "decision", "styles": {}}]
                        ]
                    }]
                },
                "children": []
            }]
        })
        .to_string();

        let visible = visible_summary_text(&raw).unwrap();
        assert_eq!(visible, "Visible launch decision");
        assert!(!visible.contains("hidden.example"));
    }
}
