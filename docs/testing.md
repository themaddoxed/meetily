# Проверка и выпуск Meetily

Этот документ рассчитан на человека без опыта в тестировании. Главное правило
Meetily: уже сохранённые аудио и текст встречи нельзя потерять из-за ошибки
распознавания, диаризации, саммари, экспорта или интерфейса.

Сборка сама по себе не означает, что релиз готов. Релиз разрешён только когда
по одному и тому же commit SHA прошли автоматические проверки, проверка
production-кандидата и ручной smoke-чеклист.

## Самые нужные команды

Запускайте команды из корня репозитория.

```bash
# После обычного изменения кода. Не собирает production frontend.
./scripts/check.sh --quick

# Перед любой сборкой кандидата. Это единая полная автоматическая проверка.
./scripts/check.sh --full

# Собрать и проверить Universal macOS app + DMG.
./scripts/release-candidate.sh

# Последняя обязательная проверка заполненного smoke-отчёта.
./scripts/release-gate.sh --smoke-report путь/к/заполненному-отчёту.md

# Локальный тест-план без Apple distribution trust.
./scripts/test-plan-gate.sh --smoke-report путь/к/заполненному-тест-плану.md
```

`test-plan-gate.sh` использует отдельный шаблон
[`docs/release/test-plan-smoke-report-template.md`](release/test-plan-smoke-report-template.md).
Он не требует Developer ID, Gatekeeper и notarization, но сохраняет проверки
точного commit/артефакта, provenance кандидата, остальных package checks и всех
функциональных smoke-сценариев. Его `PASS` означает только завершённый локальный
тест-план и не является разрешением на публичный релиз.

## Адресные проверки

Когда нужно проверить только один участок, используйте отдельный маршрут
существующего `check.sh`:

```bash
# Контракт этого руководства. Не запускает browser, Rust или production build.
./scripts/check.sh --docs

# Все unit-тесты библиотеки Meetily под фильтром audio::.
./scripts/check.sh --audio

# Все unit-тесты библиотеки Meetily под фильтром summary::.
./scripts/check.sh --summary
```

`--docs` запускает `cd frontend && pnpm test:docs`. Этот маршрут проверяет
содержание руководства одним реальным Node-тестом и не запускает browser, Rust
или production build. `--audio` сначала выполняет
`cargo test -p meetily --lib audio:: -- --list`, а `--summary` —
`cargo test -p meetily --lib summary:: -- --list`; затем каждый маршрут
запускает тот же фильтр на реальных Rust-тестах. Если фильтр не находит тесты,
маршрут сообщает «ноль совпавших тестов» и
завершается с ошибкой; такая проверка не считается `PASS`. Неизвестный флаг
завершается ошибкой до запуска дочерних команд. `--quick` и `--full` сохраняют
свой полный состав этапов; эти маршруты нужны для адресного запуска и не
заменяют полную проверку перед кандидатом.

Аналогичные frontend-команды доступны из `frontend`: `pnpm check:quick`,
`pnpm check` и `pnpm release:candidate`.

`--quick` обычно нужен во время разработки. `--full` обязателен перед
кандидатом. Обе команды продолжают работу после отдельной ошибки, чтобы в
конце показать все упавшие этапы. Успех — только итоговая строка `PASS` и код
возврата 0. `SKIP` в quick-режиме не является доказательством для релиза.

## Что именно проверяется автоматически

Полная команда запускает:

1. проверку пробелов и конфликтных фрагментов в diff;
2. production-policy: Tauri-конфигурацию, ресурсы, sidecar-файлы, dev/test
   bootstrap, очевидные секреты, опасные URL и пути разработчика;
3. frontend lint и TypeScript typecheck;
4. frontend unit/contract-тесты, включая проверку Cargo workspace: секции,
   которые работают только в корневом `Cargo.toml`, не должны оставаться в
   manifests участников, а зафиксированные `cpal` и `esaxx-rs` должны
   продолжать браться из registry;
5. Playwright-тесты восстановления, Tauri browser-контрактов и папки записей.
   Отдельный `desktop-local-safety` contract проверяет два безопасных отказа:
   отсутствующая local summary model не меняет сохранённые transcript и старое
   summary; ошибка updater не ломает открытие встречи, пока test harness
   блокирует и учитывает внешний HTTP/WebSocket-трафик WebView;
6. статическую production-сборку Next.js;
7. `cargo fmt --check`, `cargo clippy -D warnings` и все Rust-тесты;
8. реальные файловые SQLite-тесты миграций, транзакций, WAL-recovery и
   повторного открытия;
9. восстановление настоящего MP4 checkpoint через packaged FFmpeg на macOS.

`desktop-local-safety` — это browser contract с Tauri mocks. Он не запускает
установленное `.app`, не отключает сеть macOS и не видит соединения, открытые
Rust, sidecar или системными процессами. Поэтому это **не** полный native или
packaged offline smoke и не основание отмечать ручной `SMOKE-OFFLINE` как
`PASS`.

Python/FastAPI runtime и дерево `backend/` удалены из продукта. Совместимость
осталась только для проверяемого импорта старой SQLite-базы. Production-
приложение — статический Next.js frontend внутри Tauri 2; Rust отвечает за
SQLite, аудио, STT, диаризацию, саммари и desktop-команды.

## Критический путь и защита данных

| Этап | Вход | Результат | Возможная ошибка | Что обязано сохраниться | Проверка |
| --- | --- | --- | --- | --- | --- |
| Захват аудио | разрешения macOS, выбранные микрофон и system audio | сырые фрагменты и `metadata.json` | запрет разрешения, устройство исчезло, crash | уже записанные checkpoints и идентификатор встречи | Rust filesystem-тесты + ручной реальный Mac |
| Обработка аудио | асинхронные mic/system buffers | mixed/resampled chunks | несовпадение sample rate, FFmpeg/VAD error | исходные checkpoints | Rust mixer/VAD-тесты + ручное прослушивание |
| STT | речь и локальная модель Whisper/Parakeet | новые сегменты текста | модель отсутствует, worker error, пустой результат | аудио, встреча и все прежние сегменты | unit-тесты + Playwright failure/recovery |
| Промежуточное сохранение | meeting id, transcript updates, checkpoints | IndexedDB snapshot, `transcripts.json`, SQLite state | I/O/DB error, принудительное завершение | последняя успешно записанная версия | Playwright + реальный temp SQLite + ручной force quit |
| Завершение встречи | активная запись | финальное аудио и статус meeting | merge/finalize error или crash | незавершённая встреча, текст и checkpoints | unit/integration-тесты + ручной smoke |
| Постоянное хранение | meeting/transcript/summary | мигрированная SQLite DB | новая/старая схема, повторная migration, rollback или повреждённый WAL | все ранее сохранённые строки и backup DB/WAL/SHM до repair | реальные файловые SQLite migration/WAL-тесты |
| Диаризация | сохранённые тайминги и локальный результат | speaker segments/assignments | model/payload/transaction error | исходный текст и старые speaker data | реальная SQLite transaction-интеграция + smoke |
| Саммари | сохранённый transcript и выбранная модель | сохранённое summary | отсутствующая модель, LLM error, неверный ответ | transcript всегда; старое summary при regeneration error | SQLite preservation-тесты + local-model smoke |
| Экспорт | повторно открытая встреча | файл пользователя | неверный путь, права, запись/rename error | исходная встреча и transcript | реальная atomic filesystem-запись + ручной UI smoke |
| Перезапуск | app-data после normal/forced quit | та же встреча после запуска | повреждённый recovery item | все здоровые встречи; сломанный item изолирован | новый SQLite pool + Playwright reload + packaged-app smoke |

Тяжёлые модели можно подменять только на их границе. SQLite, миграции,
транзакции, checkpoints и запись экспортируемого файла тестируются на реальном
временном хранилище.

## Подготовка компьютера

Нужны Node/pnpm, Rust и полный Xcode. Один раз выполните:

```bash
cd frontend
pnpm install --frozen-lockfile
cd ..
rustup component add rustfmt clippy
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

Если macOS выбирает только Command Line Tools:

```bash
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
```

Universal-сборке нужны проверенные локальные FFmpeg и ONNX Runtime артефакты.
Их устройство и инициализация описаны в
[`docs/development/universal-macos-local-artifacts.md`](development/universal-macos-local-artifacts.md).
Скрипт кандидата сам переносит уже подготовленные артефакты в worktree и
проверяет checksum/архитектуры.

## Дополнительные проверки локальных моделей и аудио

Эти проверки запускаются только явно на том Mac, который вы тестируете. Они не
входят в `check.sh`: тесты с реальной моделью и аудиоустройством помечены
`ignored`, чтобы CI и обычный `cargo test` не выдавали отсутствие оборудования
за успех или случайную ошибку. Скрипты ничего не скачивают, не меняют SQLite и
не записывают transcript встречи. Они используют уже установленные модели и
создают временные аудиофайлы, которые удаляются после запуска.

### Реальные local STT и summary model

Сначала соберите или найдите локальный `llama-helper`, затем задайте пути явно.
Ниже нет пути конкретного разработчика или секрета. Пример использует одну
зарегистрированную пару model name/filename; если проверяете другую модель,
замените оба значения на пару из её production `ModelDef`.

```bash
cargo build --release -p llama-helper --features metal

export MEETILY_TEST_MODELS_DIR="$HOME/Library/Application Support/com.meetily.ai/models"
export MEETILY_LLAMA_HELPER="$PWD/target/release/llama-helper"
export MEETILY_TEST_SUMMARY_MODEL_PATH="$MEETILY_TEST_MODELS_DIR/summary/gemma-4-E2B-it-Q4_K_M.gguf"
export MEETILY_TEST_SUMMARY_MODEL_NAME="gemma4:e2b"
export MEETILY_TEST_SUMMARY_EXPECTED_TEXT="Dark theme Friday."
export MEETILY_TEST_SAY_VOICE="Samantha"

./scripts/smoke-local-models.sh
```

Скрипт создаёт известную речь через macOS `say`, преобразует её локальным
FFmpeg и требует, чтобы реальный Parakeet распознал достаточную часть слов.
По умолчанию для английской контрольной фразы используется английский голос
`Samantha`, а `MEETILY_TEST_SAY_VOICE` позволяет явно выбрать другой
установленный английский голос. Это не даёт языку голоса по умолчанию в macOS
превратить несовпадение test fixture в ложный STT failure.
Затем он форматирует prompt той же функцией Meetily, которую использует
приложение, запускает указанный `llama-helper` с указанной GGUF-моделью и
требует точный осмысленный ответ без служебных маркеров модели. Результат
конкретной модели и конкретного компьютера нужно сохранять в evidence текущего
запуска, а не считать постоянным свойством всех Mac.

Template, параметры генерации и stop tokens harness берёт из production
`ModelDef` для `MEETILY_TEST_SUMMARY_MODEL_NAME`, а имя GGUF-файла обязано
совпасть с определением этой модели. Для быстрой проверки совместимости
используются явно ограниченные test-only пределы: context до 1024 tokens и
ответ до 48 tokens. Это не полный production summary с настоящим длинным
transcript, production context/chunking и видимым путём packaged UI — они
остаются ручными пунктами.

Parakeet и LLM запускаются как две отдельные под-проверки. Скрипт сохраняет оба
результата, даже если первая под-проверка упала, и только в конце возвращает
ненулевой код, если упала хотя бы одна.

### Реальная post-call диаризация

Контрольная запись создаётся только из встроенных macOS-голосов и не содержит
пользовательского аудио. WAV игнорируется Git. По умолчанию используются два
разных английских голоса `Samantha` и `Daniel`, четыре поочерёдные реплики и
паузы между ними:

```bash
./scripts/generate-diarization-fixture.sh

export MEETILY_DIARIZATION_FIXTURE_PATH="$PWD/artifacts/diarization-fixture/two-speaker-16k.wav"
export MEETILY_DIARIZATION_RESOURCE_ROOT="$PWD/frontend/src-tauri/resources"

cd frontend/src-tauri
cargo test -p meetily --test diarization_runtime \
  production_sherpa_runtime_detects_two_ordered_speakers_without_a_count \
  -- --ignored --exact --nocapture
```

Проверка использует production `SherpaDiarizationRuntime`, PCM mono 16 кГц и
threshold clustering без заданного количества участников. Она требует минимум
два кластера, упорядоченные интервалы и валидное покрытие записи, но не привязывает
нейтральные номера `Speaker 1`/`Speaker 2` к конкретному системному голосу.

Модели в установленном приложении находятся в
`Contents/Resources/diarization/models`. Production runtime ничего не скачивает:
Universal build переносит только pinned-артефакты из локального store и перед
запуском проверяет точный размер и SHA-256. Если тест сообщает
`model_unavailable` или `model_invalid`, первый шаг — из корня репозитория снова
выполнить `node frontend/scripts/provision-universal-artifacts.js seed` и
убедиться, что `frontend/src-tauri/resources/diarization/models` заполнен.

Для 27-секундного fixture ориентир на проверенном Intel Mac — около 10 секунд
самого inference после компиляции test binary. Это диагностический ориентир, а
не лимит для пользовательских встреч. Результат записывается отдельно для
`x86_64` и `arm64`: успешный запуск на Intel не даёт права отметить Apple
Silicon как `PASS`, и наоборот.

### Реальные system-audio device и capture

На тестовом Mac выдайте необходимые разрешения, закройте приложения с важным
звуком и укажите точный уже собранный `.app`, который проверяется:

```bash
export MEETILY_PACKAGED_APP="$PWD/target/universal-apple-darwin/release/bundle/macos/meetily.app"
MEETILY_RUN_HARDWARE_TESTS=1 ./scripts/smoke-macos-audio.sh
```

Скрипт создаёт временный тон локальным FFmpeg, воспроизводит его через
`afplay`, проверяет реальные имена system-audio devices и требует, чтобы
CoreAudio capture успел завершиться до timeout и получил настоящий сигнал —
это проверяется по RMS/peak. Во время проверки будет слышен тестовый тон.
Чтобы CoreAudio работал с корректными macOS metadata и разрешениями, скрипт
копирует указанный packaged app во временную папку, заменяет executable только
в этой копии на Rust test harness и ad-hoc подписывает временный test host.
Исходный `.app` не изменяется. Проверка микрофона остаётся
`NOT_RUN`: её нужно выполнить через видимый UI и ручной smoke-чеклист.

Скрипт собирает результаты обеих системных под-проверок. Если хотя бы одна
упала, общий результат — `OVERALL FAIL` и код 1; микрофон при этом всё равно
остаётся `NOT_RUN`. Если обе напечатали `PASS`, весь `smoke-macos-audio.sh`
завершится кодом 2 (`OVERALL NOT_RUN`), потому что микрофон и видимый UI ещё не
проверены. Это ожидаемый честный итог, а не ошибка самого скрипта.

Оба скрипта — дополнительное runtime evidence, но не замена запуску exact
packaged artifact. Их `PASS` сам по себе не подтверждает UI, разрешения в
чистой учётной записи, сохранение встречи или строку smoke-отчёта.

## Как создать release candidate

1. Закоммитьте изменения и удалите лишние untracked-файлы. Скрипт откажется
   работать при любых неигнорируемых изменениях в worktree.
2. Выполните `./scripts/release-candidate.sh`.
3. Скрипт заново запустит full check, соберёт Universal app/DMG и проверит
   metadata, resources, arm64+x86_64 executables и bundle contents. Для `.app`
   отдельно проверяются целостность подписи, Developer ID, hardened runtime и
   фактически встроенные entitlements. Для DMG отдельно проверяются структура,
   целостность подписи, Developer ID, Gatekeeper и stapler/notarization. DMG
   также монтируется только для чтения: fingerprint находящегося внутри `.app`
   обязан совпасть с уже проверенным приложением. Mach-O executables сканируются
   на пути workstation; известные пути из закреплённых сторонних ONNX Runtime и
   FFmpeg перечисляются в отчёте как аудированные, любой иной `/Users/...`
   блокирует кандидата.
4. Результаты обычного кандидата появятся в
   `artifacts/release-gate/<commit-sha>/`:
   `automatic-check.log`, `artifact-seed.log`, `universal-build.log`,
   `version-resolution.log`, `candidate-verifier.log`,
   `source-fingerprint.log`, `candidate-report.json` и `status.env`.
   Внутри `candidate-report.json` обязательно сохраняются режим, чистота
   worktree, run ID, fingerprint исходников и release eligibility.
   Исходники повторно проверяются после сборки и после verifier. Отчёт сначала
   создаётся как pending и публикуется атомарно только после последнего
   совпавшего fingerprint; при изменении исходников готового report не остаётся.
   Финальный gate заново вычисляет текущий fingerprint, читает provenance из
   JSON и не доверяет имени каталога или одному `status.env`.

При диагностике упаковки допустима команда:

```bash
./scripts/release-candidate.sh --diagnostic-build
```

Она продолжит сборку после упавших автоматических проверок и может использоваться
для диагностики упаковки из dirty worktree, но всегда завершится ошибкой и
пометит артефакт `UNGATED`. Каждый запуск получает отдельный каталог:
`artifacts/release-gate/diagnostic/<commit-sha>/run-*`, а для dirty worktree —
`artifacts/release-gate/diagnostic-dirty/<commit-sha>/run-*`. В `status.env`
записываются `WORKTREE_CLEAN`, `RUN_ID` и `SOURCE_FINGERPRINT`; в JSON им
соответствуют `provenance.worktreeClean`, `provenance.runId` и
`provenance.sourceFingerprint`. Fingerprint покрывает commit, tracked diff,
пути и хеши содержимого untracked-файлов; содержимое diff и файлов в лог не
печатается. Такой DMG нельзя называть release candidate, передавать как готовый
релиз или использовать в smoke-отчёте.

Если обычный режим обнаружит dirty worktree, он не запускает автоматические
проверки или сборку, но оставляет уникальную preflight-запись в
`artifacts/release-gate/preflight-dirty/<commit-sha>/run-*/status.env`. Это
диагностическое доказательство отказа, а не release evidence.

Если Developer ID пока недоступен, запускайте только диагностическую сборку.
В evidence/заметках укажите `Developer ID signing: NOT_AVAILABLE` и
`notarization: NOT_RUN`; соответствующие строки smoke-отчёта остаются
`NOT_RUN`, потому что допустимые статусы строк — только `PASS`, `FAIL` и
`NOT_RUN`. Ad-hoc подпись может подтвердить целостность локальной сборки, но не
заменяет Developer ID, Gatekeeper или notarization. Публичный release gate при
этом остаётся заблокирован.

## Ручной smoke-тест на реальном Mac

Скопируйте [`docs/release/smoke-report-template.md`](release/smoke-report-template.md)
в новый файл и заполните шапку. Каждый пункт относится к точному commit и SHA-256
DMG. Разрешены только `PASS`, `FAIL`, `NOT_RUN`; для `PASS` нужно конкретное
доказательство, а не «работает».

Пройдите все строки шаблона. В практическом порядке это означает:

1. проверить checksum, подпись, notarization и установку DMG;
2. в чистой учётной записи macOS выдать microphone и screen/system-audio
   permissions;
3. выбрать реальные источники, создать встречу и записать уникальные фразы из
   микрофона и system audio;
4. увидеть live transcript и убедиться, что checkpoints появились в папке
   встречи;
5. один раз выполнить Force Quit во время записи, перезапустить приложение и
   проверить точные восстановленные фразы;
6. вторую встречу завершить штатно, открыть её и проверить постоянное сохранение;
7. выполнить diarization, переименование speaker и повторное открытие;
8. создать local summary, затем отдельно проверить отсутствующую/сломавшуюся
   модель: сохранённый transcript не должен измениться;
9. экспортировать из видимого UI, открыть файл и сверить фразу, summary и
   checksum;
10. отключить сеть и повторить local recording, STT, summary, restart/reopen;
11. на копии данных предыдущей версии проверить миграцию и старую встречу;
12. приложить пути/время логов и запустить финальный `release-gate.sh`.

Сейчас аудит репозитория не нашёл доступного пользователю действия
переименования speaker и вызова file export из frontend UI. Наличие Rust-команды
или теста сериализации не заменяет продуктовый сценарий. Пока соответствующие
действия нельзя реально выполнить, `SMOKE-SPEAKER-RENAME` и
`SMOKE-EXPORT-UI` должны быть `FAIL`/`NOT_RUN`, а релиз заблокирован.

Разрешения macOS, реальный микрофон/system audio, качество STT, чистая
установка, offline network audit, Developer ID подпись и notarization нельзя
подтвердить unit-тестом или запуском в sandbox. Их результат всегда сообщается
отдельно.

Для offline-пункта нужен запуск точного установленного `.app` с отключённой
сетью и отдельное наблюдение native sockets. Playwright route видит только
HTTP/WebSocket WebView; Rust и sidecar остаются за этой границей. Для upgrade
нужны резервная копия данных настоящей предыдущей версии и точный проверяемый
DMG. Browser reload, новая временная SQLite и test fixture не заменяют эти
ручные проверки.

## Проверка обновления предыдущей версии

Никогда не экспериментируйте на единственной пользовательской базе.

1. Закройте Meetily и сделайте копию всей папки app data и папки нужной встречи.
2. Запишите известные контрольные данные: название, две фразы transcript,
   speaker, summary и расположение аудио.
3. На отдельной тестовой учётной записи установите предыдущую версию и откройте
   эти данные.
4. Установите проверяемый DMG поверх неё и запустите приложение.
5. Сверьте все контрольные данные, закройте приложение, запустите снова и
   повторите сверку.
6. Если migration падает или что-либо пропало, не повторяйте попытки на
   оригинале: сохраните копию и ставьте `FAIL`.

App data можно открыть из настроек Meetily. Для ручного поиска базы:

```bash
find "$HOME/Library/Application Support" -name meeting_minutes.sqlite -print
```

Не удаляйте `.sqlite`, `-wal` или `-shm` ради «исправления» релизного теста.

## Проверка восстановления после падения

1. Выберите папку записей в Settings и откройте её из приложения.
2. Начните встречу, дождитесь минимум двух уникальных transcript-фраз и
   нескольких аудио checkpoints.
3. Запишите текущее время и выполните Force Quit, не нажимая Stop.
4. Скопируйте папку незавершённой встречи до любых ручных исправлений.
5. Запустите Meetily, восстановите встречу и сверьте обе фразы, metadata и
   доступное аудио.
6. Завершите/сохраните восстановление, перезапустите приложение и проверьте его
   ещё раз. Повреждённая тестовая встреча не должна мешать открытию остальных.

Максимально восстановимая запись зависит от последнего успешно сброшенного
checkpoint. Потеря всего transcript или невозможность запуска из-за одного
повреждённого item — блокирующий дефект.

Checkpoints удаляются автоматически только после успешного восстановления
аудио и постоянного сохранения встречи. При failed/partial recovery они
остаются на диске. Если crash произошёл до первого STT-сегмента, встреча может
быть сохранена с пустым transcript только когда recovery вернул реальный audio
file; если не восстановились ни текст, ни аудио, item остаётся незавершённым.
При подозрении на повреждение SQLite приложение сначала копирует существующие
`.sqlite`, `-wal` и `-shm` в отдельный recovery-каталог и только затем может
удалить sidecar-файлы; ошибка backup запрещает такое удаление.

Переносимые Rust-тесты проверяют настоящие `transcripts.json`, безопасную ошибку
записи, no-op recovery и cleanup во временной файловой системе. Полное
восстановление настоящего 30-секундного MP4 checkpoint через packaged FFmpeg
помечено `ignored` в обычном `cargo test --workspace`: `check.sh` запускает его
отдельным обязательным этапом на macOS, а на других ОС честно сообщает `SKIP`.
Browser-тесты проверяют обработку уже обнаруженной recovery-записи после reload
и отдельный контракт `native sequence_id → IndexedDB save → reload → read`: в
настоящем Chromium идентификатор сегмента должен сохраниться и определить
правильный порядок транскрипта после повторного чтения. Этот контракт не
проводит настоящий `recording-started` через весь event path. Поэтому Force Quit
и полный путь `recording → IndexedDB → reload` остаются обязательными ручными
пунктами и не должны отмечаться как автоматически проверенные.

## Быстрый повтор одной проверки

Из `frontend`:

```bash
pnpm lint
pnpm typecheck
pnpm test:frontend
pnpm exec playwright test tests/interrupted-recovery.spec.ts tests/indexeddb-transcript-contract.spec.ts tests/fixture-contracts.spec.ts tests/recording-folder-selection.spec.ts tests/desktop-local-safety.spec.ts
node --test tests/cargo-workspace-config.test.mjs
node --test tests/runtime-smoke-contract.test.mjs
pnpm check:policy
pnpm build
```

Из корня:

```bash
cargo test --workspace
cargo test -p meetily --test local_model_runtime --locked --offline
cargo test --test data_integrity
cargo test --test database_recovery_backup
cargo test --test recording_recovery
cargo test --test recording_recovery successful_checkpoint_recovery_preserves_neighboring_meeting_files -- --ignored
cargo test --test diarization_apply
cargo test --test diarization_migration
cargo test --test export_integrity
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

`runtime-smoke-contract.test.mjs` проверяет устройство opt-in скриптов без
запуска моделей и оборудования. Обычная команда `cargo test ...
local_model_runtime` проверяет разбор и защиту ответа helper; два тяжёлых теста
реальных моделей остаются явно `ignored` до запуска
`smoke-local-models.sh`. Ни одна из этих команд не должна быть выдана за
аппаратный или packaged smoke.

## Где искать диагностику

- Gate/build: `artifacts/release-gate/<commit-sha>/`.
- Development frontend: терминал с `pnpm dev` и DevTools (`Cmd+Shift+I`).
- Development Rust: терминал с Tauri; для подробностей запустите
  `RUST_LOG=debug pnpm run tauri:dev`.
- Packaged macOS app: кнопка Console в интерфейсе запускает
  `log stream --process meetily --level info --style compact`; тот же command
  можно выполнить вручную в Terminal.
- Recording/recovery: выбранная в Settings папка; по умолчанию на macOS это
  `~/Movies/meetily-recordings`. Кнопка папки у встречи открывает её напрямую.
- SQLite/models/preferences: app data, которую можно открыть из Settings.

Если дополнительная runtime-проверка упала:

- `model ... does not exist`, `not available`, несовпадение GGUF filename с
  production `ModelDef` или ошибка чтения/загрузки — файл отсутствует,
  повреждён или выбран не тот model name; проверьте заданные имя модели и путь,
  права чтения и целостность локального файла; модель автоматически не удаляйте
  и не скачивайте заново поверх неё;
- `empty response`, `control marker` или `did not return the exact normalized
  reply` — сохраните полный terminal output: helper запустился, но ответ модели
  не прошёл контракт;
- `timed out` — зафиксируйте этап (`startup ping`, generation, Parakeet load/STT
  или CoreAudio capture), модель, Mac/chip и время; не увеличивайте timeout,
  пока не сохранена причина;
- `silent`, низкие `RMS`/`peak` — проверьте system output, громкость, разрешение
  macOS и что тестовый тон действительно слышен, затем повторите один раз;
- stderr `llama-helper`, строки `PASS`/`FAIL` Rust и `RUST_LOG=debug` находятся
  в том же Terminal; packaged-логи ищите через Console/`log stream`, как
  описано выше.

Любой такой `FAIL` запрещает использовать результат как evidence, но не требует
и не разрешает менять transcript, SQLite или папку встречи. Сначала сохраните
лог и копию данных, исправляйте модель/окружение отдельно; повреждённую модель
Meetily не должен удалять автоматически.

Для бага приложите весь каталог evidence gate и копию всей проблемной папки
встречи, а не отдельный случайно выбранный файл. Встреча содержит приватные
аудио и transcript: архивируйте и передавайте её только явным, разрешённым
пользователем действием. Автоматические проверки ничего не загружают наружу.

В описании бага нужны: commit SHA, версия, SHA-256 DMG, Mac/chip, macOS,
устройства, модели, состояние сети, точный шаг, ожидаемый и фактический
результат, время ошибки, пути логов и что именно удалось восстановить. Не
прикладывайте токены, ключи или пароли.

## Что запрещает релиз

Релиз заблокирован при любом из условий:

- потеря/повреждение аудио, transcript, meeting metadata или старых данных;
- упавшая/непроверенная migration, transaction или crash recovery;
- сохранённую встречу нельзя открыть после перезапуска;
- ошибка STT/diarization/LLM меняет уже сохранённый transcript;
- локальный сценарий неожиданно отправляет данные во внешний сервис;
- production использует dev server, путь репозитория, секрет или отсутствующий
  resource/sidecar;
- обязательная подпись, Gatekeeper или notarization не подтверждены;
- full check, candidate verifier или хотя бы одна строка smoke не `PASS`;
- отсутствующая пользовательская функция замаскирована тестовым mock;
- diagnostic/`UNGATED` artifact выдан за release candidate.

При ошибке сначала сохраните данные и evidence, затем повторите ровно команду,
которую gate напечатал как `FAILED COMMAND`. Не отключайте проверку и не
заменяйте её фиктивным успехом.
