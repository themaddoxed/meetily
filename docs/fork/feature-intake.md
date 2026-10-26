# Feature intake — выборочный перенос функций

Статус на 2026-09-06: исследование источников и проект задач завершены; применимость коллегиного patch к текущей базе `NOT VERIFIED`. Приложения, runtime, качество распознавания и физический macOS-сценарий в рамках intake не запускались.

## Источники

- Публичный снимок [Meetily-ActuallyFree](https://github.com/TylerBuza/Meetily-ActuallyFree/tree/4e1527dca64339f9bb21acbc191ebc21db99af2d).
- Переданный коллегами patch: SHA-256 `e7b464e57dd50cac7dfd3091126e35bcab3d4fdef3c233d3d3dfb18158cef330`.
- Переданная коллегами HTML-спецификация: SHA-256 `45ed287397b375a3775aad1c6faef6a348860b42b9b5968d988f873429325433`.
- Предложение задач: SHA-256 `b9590214aaeb64741d10d8ad0aad529bbbac6bf685cf70dcaad47a6ffd03bb2c`.
- Документальная foundation-база: commit `4fe8c0e48a4e5b9f4039cfd890d2de3f46884b53`; product integration ref: `b7126ea82be887c38e8a8e8de30a48f9bb409e51` (`refactor(runtime): remove legacy FastAPI product backend`). Exact applicability patch нужно проверять в изолированном checkout перед копированием кода.

## Решение по этапу 0

Закрытый `FORK-030` не переоткрывается и не получает искусственный blocker. Реестр переносов остаётся источником статуса источников, а каждая новая `FEAT`-задача начинает работу с проверяемого source-applicability preflight. Если patch не применяется или условия переноса неясны, задача остаётся незавершённой с записью причины и без копирования закрытого кода.

## Расширения существующих FORK-задач

- Пункт 1 proposal — `FORK-030`: карта источников, базы и условий применимости.
- Пункт 11 — `FORK-045` и `FORK-049`: короткие и длинные фразы Whisper; `FORK-044` остаётся закрытым.
- Пункт 14 — `FORK-047` и `FORK-048`: общая сессия для хоткея и компактной панели.
- Пункт 15 — `FORK-041` и `FORK-042`: отдельные дорожки и связь говорящих с настройками.
- Пункт 24 — `FORK-036`: сценарии безопасного обновления после выбора identity.
- Пункт 25 — `FORK-037`, `FORK-043` и `FORK-052`: сохранность данных и recovery.
- Пункт 26 — `FORK-046`, `FORK-047` и `FORK-048`: требования нативного macOS-дизайна.

Пункты 2–10, 12–13 и 16–23 сохранены отдельными задачами в [fork-features.md](../prd/fork-features.md). Зависимости между ними добавлены после автоматической выдачи глобальных `FEAT` ID; существующие статусы и прежние зависимости `FORK` сохранены, а новые согласованные связи добавлены только для `FORK-041`, `FORK-045` и `FORK-047`.


## FEAT-100 — source preflight, 2026-10-02

Pinned public source `4e1527dca64339f9bb21acbc191ebc21db99af2d` was read via
raw GitHub (architecture, API, command registration and `live_assistant.rs`).
GitHub API returned HTTP 403; that access failure is not a claim that the
source is unavailable. Donor live-assistant context is frontend-supplied,
whereas FEAT-100 requires Rust to restrict the context to a selected SQLite
meeting and preserve transcript links. The prepared original implementation
uses the current schema; provider/person/prompt code is not transferred.
[Candidate and census](meeting-context-2026-10-02.md) record the exact source
hashes, five owned files and four passing temporary-SQLite tests. Independent
review and installed native invocation are not run; the task remains open and
the candidate is not integrated into main. This does not change applicability
or permissions for the colleague patch or other FEAT tasks.
