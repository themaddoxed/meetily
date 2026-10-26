# macOS Fork — mdtask

## Как выполнять обновлённый план

Актуальная очередь — в этих двух PRD интегрированного `main@53f4415058fb01e38b8892a47ac036bfffa394ba` от 2 октября 2026. Сентябрьские product/docs checkout остаются сохранёнными источниками; новые изменения проверяются относительно этой текущей базы. Контекст, ограничения и порядок пачек — в [execution plan](../fork/execution-plan-2026-09-22.md), обоснование — в [handoff](../fork/codex-handoff-2026-09-22.md).

FEAT-186–188 и FEAT-094–096 уже интегрированы. CI/theme планы получили независимый SPEC/QUALITY PASS; storage также получил SPEC/QUALITY PASS после уточнения контракта. Перепривязки FORK-126/150 выполнены; FORK-148 реализован, его scanner выявляет оставшуюся платформенную поверхность. Native-задачи выполняются только с доказательствами exact artifact. `mdtask --path docs/prd list --tag batch1` показывает ближайшую пачку; команда запускается из checkout плана. `#exec` означает короткую задачу, `#milestone` — итоговую приёмку. `@parent`, `@iter`, `@estimate`, `@kind`, `@review` — описательные метки; реальные блокировки заданы через `@blocked_by`. Родитель закрывается вручную после всех детей и исходных критериев; отсутствие открытых blockers само по себе не является приёмкой.

Существующие IDs и 22 закрытые задачи сохранены. I01–I19 остаются предварительной группировкой дальнего объёма; ближайший приоритет задают пачки в execution plan. Статусы и evidence относятся к указанному снимку кода; документальный план не подтверждает готовность выпуска.

План отделения самостоятельного macOS-продукта от Meetily. Исходная линия сохраняет проверенную локальную историю, а upstream используется только как источник для ручного просмотра изменений.

## Встроенная Gemma: текущий путь

В настройках встроенных моделей и onboarding каталог текущего main предлагает
Gemma 4 E2B и E4B. Выбор сохраняется через общий model-config API; скачивание
использует проверку размера/SHA-256 и атомарную установку FEAT-070. Отдельный
загрузчик Gemma не требуется. [Проверка исходников и fixtures](../fork/gemma-path-2026-10-02.md)
подтвердила этот маршрут на main от 2 октября. Реальная генерация, reopen,
offline и RAM-поведение на Intel/Apple Silicon ещё требуют FORK-134/167;
рекомендация по памяти не является результатом benchmark.

## Fork baseline

Исходники форка зафиксированы полным commit SHA `1e742fa22030fe5720db6a791407c64eea03ede4` и tree SHA `1fbcc32d422e5d86c5266f4dfb8b7ce863c86845`. Ветка `feature/release-ready-ui` служит только изменяемым указателем; при проверках нужно использовать полный commit SHA.

Состав исходников, локальный снимок ancestry, диагностическая Universal-сборка и ограничения release readiness записаны в [macOS fork baseline](../fork/baseline.md). Создание новой integration-line остаётся отдельной задачей `FORK-031`.

Publish-safe реестр переноса и его frozen manifests записаны в
[macOS fork migration registry](../fork/migration-registry.md). Точные локальные
пути, session/worktree locators и ключ проверки остаются вне Git в защищённом
evidence bundle; обычная проверка реестра от него не зависит.

## Поддержка платформы

На старте форк распространяется одним Universal macOS-приложением с нативными
`x86_64` и `arm64` срезами и поддерживает Intel и Apple Silicon начиная с
macOS 14.2 Sonoma. Наличие обоих срезов подтверждает сборку, но runtime требует
отдельного физического smoke на Intel и Apple Silicon для одного точного source
commit и артефакта. Нижняя граница macOS 14.2 проверяется отдельно; более новая
macOS её не доказывает. Полная политика и условия пересмотра Intel описаны в
[macOS platform support](../fork/platform-support.md).

## Git policy

Текущий execution context от 22 сентября 2026 указан в начале PRD и [Git policy](../fork/git-policy.md#execution-context-2026-09-22); он уточняет базу следующих task checkout. Ниже сохранена история создания integration-line, а не инструкция начинать новую задачу от старого tip. Интеграция и handoff этим планом не выполняются.

Provisional integration-line `fork/macos-integration` создана в
`.worktrees/macos-fork-integration` от immutable commit
`1e742fa22030fe5720db6a791407c64eea03ede4`; после `FORK-058` её текущий tip равен
`dbf9bc3e6a019acd6c265cda72b76b4697a0cdf1`. До `FORK-054` это единственная
база и destination для продуктовых задач `FORK-032`–`FORK-053` и добавленных
архитектурных задач, а task-ветки получают namespace
`fork/<task-id>-<slug>`. Нынешние `main`, `dev`, remotes и ранее существовавшие
worktrees не изменены. Push и публикация не выполнялись; rollback допускает
только карантин линии без удаления. После canonical handoff `dev` принимает
проверенную интеграцию, `main` содержит только релизы, а `feature/*` создаются
от `dev`.
`upstream` остаётся read-only источником: изменения берутся только после ручного
просмотра и выборочного переноса. Полные правила и handoff-гейты описаны в
[macOS fork Git policy](../fork/git-policy.md).

## macOS-only packaging surface

В integration commit `45bebcbf184da4d2b16c48a0415050019becff98`
удалены 34 Windows-скрипта и Windows-ресурса. Tauri теперь объявляет только
пакеты `.app` и `.dmg`, использует PNG/ICNS-иконки и не содержит Windows bundle
configuration; tracked DEB/RPM/AppImage outputs в baseline не было.
Platform audio, CI/release workflows и общий постоянный gate отсутствия чужих
платформ остаются отдельными задачами.

Шесть найденных остаточных путей маршрутизированы явно: Linux/Windows audio
source остаётся в `FORK-034`, два Windows/Linux workflow — в `FORK-035`, а
Linux build-документ и исключение для сохранённого web favicon — в `FORK-038`.
Scoped release-policy прошла 8 из 8 тестов. `tauri info` остаётся
`NOT_COMPLETED`: команда зависла после вывода environment, а полный Xcode на
хосте не установлен. Поэтому этот этап не подтверждает сборку или runtime.

## macOS-only Rust и build surface

В integration commit `7d5f9c9b470361dc643a6cdb859965c5e1e9d6ec`
прямые Rust target-зависимости и build-маршруты ограничены macOS. Intel
`x86_64` использует CPU-only Whisper и llama-helper, а `arm64` отдельно включает
Metal/CoreML для Whisper и Metal для helper. При отказе Metal загрузка делает
не более одной новой CPU-попытки; обязательный Alpha-путь не зависит от
Apple-Silicon-only runtime.

Удалены ручные CUDA/Vulkan/Metal feature-команды и GPU auto-detection wrappers.
Universal helper передаёт Metal только arm64-срезу; FFmpeg build принимает
только два Darwin target и сохраняет проверку кэша и `lipo`. Runtime FFmpeg,
llama sidecar, console/folder/URL handlers и single-instance wiring очищены от
Windows/Linux веток в границах `FORK-033`. Постоянная семантическая policy
проверяет Cargo target tables, architecture routing, удалённые wrappers и
ограниченные Rust/build source-файлы.

На точном commit прошли policy 168/168, полный Node-набор 334/334, Vitest 34/34,
Universal Playwright 15/15, production static build 11/11, llama-helper tests
9/9 и target checks для Intel CPU и arm64 Metal. Native Meetily tests/checks
остаются `NOT_COMPLETED`: `cidre` требует полный Xcode, а активен только
`/Library/Developer/CommandLineTools`. Поэтому этот этап не подтверждает
физический runtime, Universal artifact, package, signing или notarization.
Platform audio остаётся владельцем `FORK-034`, CI/release — `FORK-035`, общий
absence gate — `FORK-038`, permissions и microphone lifecycle — `FORK-051`, а
legacy Python/FastAPI runtime — `FORK-057`.

## Короткие фразы Whisper

В integration commit `d7ca29901995687cca2ab8a0bd0b81aaabca2144`
worker отправляет любой результат Whisper с непробельным текстом независимо от
значения confidence. Строки, состоящие только из пробельных символов, по-прежнему
отбрасываются, а confidence сохраняется без изменений как metadata в
`TranscriptUpdate`.

Изменение выполнено одним non-merge commit и затрагивает только
`frontend/src-tauri/src/audio/transcription/worker.rs`. На точном integration
commit прошли независимый Rust harness 3/3, policy 168/168 и полный Node-набор
334/334. Native Meetily Cargo compile/test остаётся `NOT_COMPLETED`: `cidre`
требует полный Xcode, а активны только Command Line Tools. Capture и границы
`FORK-034`, а также FORK-033 routing Intel CPU, arm64 Metal/CoreML и ограниченный
одной новой попыткой fallback Metal→CPU не менялись; физический runtime, push,
PR, release и публикация не выполнялись.

## Runtime architecture contract

В integration commit `e695a41f2aacab2c61c6ce7e2e4b57b1bf85f298`
принят документационный контракт целевой runtime-архитектуры: словарь
`CONTEXT.md`, ADR `docs/adr/0001-rust-owned-modular-monolith.md` и нормативный
контракт `docs/architecture/macos-modular-monolith-contract.md`.

В целевом модульном монолите Rust является единственным владельцем lifecycle
записи и фоновых jobs, переходов persistence, состояния моделей и нумерации
updates. Верхнеуровневых interfaces ровно семь: `RecordingSession`,
`CaptureBackend`, `TranscriptionEngine`, `DiarizationEngine`, `SummaryEngine`,
`MeetingRepository` и `ModelRegistry`. UI использует query/action-команды и
упорядоченные owner-scoped Channels; highest-contiguous ACK проходит через
`ackObservation(ownerId, subscriptionGeneration, sequence)`.

Контракт фиксирует закрытые переходы состояний Recording Session и Processing
Job, стабильный error envelope и коды ошибок, ограниченные по числу сообщений и
байтам Rust outboxes и UI buffers, resync по полному snapshot, а также
идемпотентные операции с canonical fingerprints. SQLite хранит идентификатор
Recording Root и нормализованные относительные пути. Файлы публикуются с
проверкой целостности, без перезаписи существующего назначения и через атомарный
rename; recovery не перезаписывает финальные файлы и завершает запись только
после согласования полного набора Tracks.

Один контракт действует для Universal `x86_64` + `arm64` на macOS 14.2:
обязательные Alpha-функции имеют Intel CPU path, а опциональное ускорение Apple
Silicon при отказе возвращается на CPU. Это только архитектурный контракт, а не
доказательство runtime, сборки, физического hardware, signing или notarization.
Legacy identity `Meetily` остаётся временной; решение о финальном имени и
публикации не принято.

## Legacy updater выключен

В integration commit `261947797fc713bb909d5293a19cecb1210cfae1`
удалён прежний Meetily update channel: frontend-проверки обновлений, Tauri
updater/process plugins и permissions, updater endpoints и public key,
генерация updater artifacts, updater-signing переменные и updater-манифесты в
текущих workflow. Финальная identity остаётся в `FORK-028`; новый endpoint и
updater signing key относятся к `FORK-036`; публикация возможна только после
release-гейтов и финального handoff `FORK-053`–`FORK-054`.

Постоянная release-policy проверяет package/Cargo dependencies, Rust plugin
registration, production imports и direct invokes, Tauri config/capabilities и
workflow-поверхность. TOML, JavaScript/TypeScript и YAML разбираются
семантически и падают закрыто при ошибке синтаксиса. На точном commit прошли
policy 125/125, полный Node-набор 291/291 и production static build 11/11.
Browser-проверка 10/10 подтверждает отсутствие updater-команд и WebView egress
только в browser harness; это не packaged offline, native runtime, signing или
notarization evidence. Полная Rust-сборка остаётся ограничена отсутствием
выбранного full Xcode на текущем хосте.

## План удаления legacy runtime

На integration commit `dbf9bc3e6a019acd6c265cda72b76b4697a0cdf1`
повторно утверждён план будущего удаления Python/FastAPI и локального
HTTP-сервера из поставляемого приложения. План сохраняет поддерживаемые внешние
provider endpoints, включая Ollama и OpenAI-compatible, а также Python-инструменты
для разработки, которые не входят в product runtime.

Старые базы должны переноситься без потери данных: с учётом SQLite WAL,
проверкой полного содержимого, неизменностью исходных файлов и восстановлением
после сбоя. Для одного каталога данных проверка первого запуска, восстановление,
инициализация и импорт выполняются последовательно; уже работающая база запрещает
повторную установку состояния или замену файлов.

Это описание повторного утверждения плана FORK-057. Последующая source-реализация FORK-034 уже интегрирована в текущий main; native-приёмка и упаковка проверяются отдельно по исходным карточкам и release gate.

## Запись на macOS

Rust управляет одной `RecordingSession` для start, pause, resume и stop. Один
macOS capture adapter передаёт microphone и system audio как отдельные Tracks,
которые persistence сохраняет раздельно до следующей обработки. Это результат
интеграции исходного кода, а не подтверждение физического runtime.

## Матрица приёмки Alpha

Откройте [матрицу оставшейся приёмки](../fork/alpha-acceptance-matrix-2026-10-02.md)
перед подготовкой финального кандидата. Она связывает 13 исходных блокеров
FORK-053, финальные этапы FORK-175–180 и все 25 строк release smoke с нужными
артефактами, устройствами и доказательствами. Текущий интегрированный main —
`53f4415`; существующие автоматические результаты отделены от приёмки
конкретного приложения. Матрица не является заполненным smoke report.

## План macOS-only CI

[Обновлённый CI/release-план](../plans/macos-ci/2026-10-02-macos-ci-release.md)
привязан к `main@53f4415`. Его [census](../plans/macos-ci/2026-10-02-macos-ci-census.json)
содержит hashes десяти текущих workflows, jobs, runners, callers и объявленные
артефакты. FORK-144 завершён: независимое ревью актуального плана — PASS;
локальный план не подтверждает hosted runner, защищённое окружение или подпись.
Старый reviewed план сохранён как источник; повторно выполнять закрытые
FORK-034/057 не нужно.

Допуск CI от 3 октября уточняет текущие prerequisites, сохраняя прежние версии
как предложения: macos-15-intel документирован как стандартный Intel runner,
Xcode 16.4 есть в обоих образах. Поправка плана получила независимый SPEC/QUALITY
PASS. [Отчёт входных условий](../fork/evidence/macos-ci-prerequisites-2026-10-03.json)
сохраняет пробелы: источник и provenance четырёх FFmpeg/ONNX артефактов,
fresh acquisition, review action-кода, точные toolchains и фактические dual-host
запуски ещё не приняты. Восемь action pins остаются кандидатами.
После archive-only checkpoint локальный actionlint 1.7.12 проверил три текущих
workflow с exit 0; [syntax baseline](../fork/evidence/macos-ci-syntax-baseline-2026-10-03.json)
относится к прежним mixed-platform workflows, а не к готовому macOS-only CI.
Реализация FORK-145 ожидает чистой согласованной базы; Git refs не менялись.

## Проверка платформенной поверхности

Запустите `pnpm --dir frontend check:platform` на выбранном source checkout;
`node frontend/scripts/platform-surface.mjs --root <checkout> --json` выдаёт
машиночитаемый отчёт. Exit 0 означает отсутствие замечаний в ограниченной
области scanner, exit 1 — найденное нарушение, exit 2 — ошибка исполнения.
Проверяются текущие Cargo/CI/package/bundle настройки, product paths,
платформенные условия и актуальные инструкции. Отдельные dev Python tools,
исторические планы, тестовые fixtures и provider URL явно отделены от
product runtime. Подробные границы и текущий FAIL описаны в
[отчёте платформенной поверхности](../fork/platform-surface-2026-10-02.md).
Scanner пока вызывается отдельно: FORK-149 и родитель FORK-038 требуют
очистки CI и проверки итогового source/package; этот инструмент их не закрывает.

## Пользовательская миграция WebView

Готовый low-level adapter ещё не подключён к пользовательскому export/import.
[Аудит подключения](../plans/webview-migration/2026-10-02-webview-migration-integration.md)
связывает существующие prepare/publish/status, WebView marker и обязательную
границу recovery-only startup. Полная миграция не завершена: публикация файлов
и импорт browser state должны подтверждаться отдельно до входа в обычный профиль.
Незавершённые или чужие данные нельзя принимать через обход startup guard.

## План Recording Root и безопасной публикации

[Перепривязанный storage-план](../plans/recording-storage/2026-10-02-recording-storage.md)
и [census](../plans/recording-storage/2026-10-02-recording-storage-census.json) описывают
переход от текущих абсолютных folder_path к устойчивым root ID, относительным
путям и SQLite journal. Они сохраняют no-clobber, проверку inode, отмену только
своего незавершённого файла и recovery без удаления пользовательских записей.
Список будущих правок расширен с 53 до 60 путей: учтены скрытая видимость в
поиске, summary writers и FFmpeg descriptor adapters. Новая миграция должна
идти после уже применённой search migration. FORK-127 завершён с независимым PASS точных байтов и отдельного контракта
immutable summary exports; FORK-043 и его реализации пока не выполнены.

## План проверки семантической темы

[Актуальный theme-план](../plans/theme/2026-10-02-semantic-theme.md)
и [census/baseline](../plans/theme/2026-10-02-theme-census.json) сохраняют
system/light/dark, единственного runtime-владельца и двух BlockNote-потребителей.
Текущий browser baseline: 30 passed, 3 failed. Ранний dark toggle подтверждён
диагностикой, но старый тест перезаписывает наблюдение; sidebar snapshots
содержат прежнюю структуру встреч. Новые snapshots не приняты. Prepaint до
первого stylesheet, hydration, keyboard focus и контраст реальных поверхностей
остаются критериями следующей проверки. FORK-151 завершён с независимым SPEC/QUALITY PASS;
FORK-039 и нативная приёмка темы открыты.

Допуск FORK-152 от 3 октября: clean admission получил независимые SPEC/QUALITY
PASS. Владелец выбрал вариант 1: локальный dev обновлён до main@53f4415,
прежний dev сохранён в `themaddoxed/dev-before-main-2026-10-03`; создана
feature/semantic-theme-2026-10-03. Исправлены mutation oracle и prepaint mount:
9 focused PASS со всеми 12 navigation/reload переходами, исходный script-узел
и ресурсы Next сохранены, скрипт исполняется один раз, hydration diagnostics нет.
Светлый muted-foreground минимально затемнён после подтверждённого RED 4,349:
реальные подписи Settings теперь имеют минимум 4,542:1; восемь пар в двух
режимах проверены. Последний persistence/system-pressure прогон: 4 PASS.
Предыдущие исходники и persistence-тесты приняты SPEC/QUALITY без C/I/M.
Добавлены policy-фикстуры: неизменённый CLI на временном src нашёл все 11
запрещённых форм и принял semantic-clean пример; 1 PASS, независимые
SPEC/QUALITY без C/I/M, ESLint и TypeScript PASS. Focus-обводка теперь явно
использует семантический цвет и остаётся видимой благодаря зазору 4 px между
кнопками. Focus/geometry: 22 PASS; все шесть реальных Tab-измерений выше 3:1
(минимум 7,015:1 light и 4,974:1 dark), переполнения при 1440×1000 нет.
Ошибка измерителя для scale воспроизведена и исправлена отказом для
неподдерживаемых преобразований; простой перенос сохранён. Текущие focus-правки
приняты SPEC/QUALITY без C/I/M. Регрессия текста/alpha и реальные подписи
Appearance: 8 PASS. Текущие targeted ESLint, TypeScript и theme-audit PASS.
4 октября текст на реальных Home, Settings и Meeting Details проверен в обоих
режимах: 2 PASS, 18 измерений, минимум 4,542:1 light / 5,999:1 dark. Внешняя
непересекающаяся тень допускается только после совместного отключения всех
кандидатов, пяти идентичных PNG и проверки неизменности DOM/CSS; восстановление
свойств обязательно. Девять проверок механизма PASS, включая подмену DOM между
последней проверкой и расчётом; SPEC/QUALITY C0/I0/M0. TypeScript и targeted
ESLint PASS. [Новые свидетельства](../fork/evidence/theme-shadow-proof-2026-10-04.json).
Фоны теперь проверяются отдельной консервативной поканальной границей с
учётом всех фоновых слоёв и прозрачности, а текст/focus сохраняют точный
измеритель. Исправлены три замечания ревью; 45 регрессионных сценариев PASS,
SPEC/QUALITY C0/I0/M0. [Свидетельства](../fork/evidence/theme-background-proof-2026-10-04.json).
Полный набор пяти theme-suite на текущих байтах: 128 PASS / 2 FAIL, без пропусков;
все три реальные проверки фонов и overflow PASS. Два FAIL — старые снимки
Sidebar без уже интегрированных даты, статуса и длительности встречи. После
просмотра и независимого допуска обновлены только два эталона; старые сохранены
с SHA-256. Обычное сравнение — 2 PASS, SPEC/QUALITY C0/I0/M0.
[Свидетельства снимков](../fork/evidence/theme-sidebar-snapshots-2026-10-04.json).
Это 128 неизменённых успешных сценариев плюс два повторно проверенных снимка,
а не новый полный прогон 130 тестов. `check.sh --full` завершился exit 1:
восемь этапов PASS, Clippy/Rust tests/FFmpeg recovery — FAIL exit 101 из-за
пустого пути ONNX Runtime в offline-режиме. Повтор с локальной библиотекой завершился тремя FAIL exit 101: отсутствует
Tauri sidecar `llama-helper-x86_64-apple-darwin`. Текущий Intel helper собран
успешно как игнорируемый тестовый ресурс; три повторных Rust-этапа PASS exit0
(workspace 603 PASS, 10 ignored; отдельный FFmpeg recovery 1 PASS); native остаётся открытым.
[Project evidence](../fork/evidence/theme-project-full-2026-10-04.json). [Evidence](../fork/evidence/theme-progress-2026-10-03.json).
Последующий обязательный полный `test:e2e`: exit 1, 210 PASS / 6 FAIL.
Все 130 theme-сценариев на текущих байтах PASS. Изолированный повтор шести
сценариев: 4 PASS / 2 FAIL. После независимого допуска исправлены два устаревших
локатора; targeted 2 PASS, полные два spec 9 PASS, SPEC/QUALITY C0/I0/M0.
Повтор обязательного полного e2e: 216 PASS, exit 0, без пропусков.
Канонический `check.sh --full` на том же diff: exit 0, все 11 этапов PASS,
пропущенных этапов нет, 416 секунд. Использованы локальная ONNX Runtime и
current-source Intel helper; Rust 603 PASS / 10 ignored, отдельный FFmpeg 1 PASS.
Прежние FAIL сохранены. Нативная матрица macOS и интеграция ещё открыты.
[Project evidence](../fork/evidence/theme-project-full-2026-10-04.json).
Изменения находятся только в feature checkout, не закоммичены и не перенесены
в dev/main; FORK-152/153 не закрыты.

## Tasks

### Основание форка

- [x] FORK-026 Зафиксировать неизменяемую базу macOS-форка `feature/release-ready-ui@1e742fa` #source #evidence !crit
  Записать commit, merge-base, состав сборки и исходные результаты проверки.

  **Implemented:**
  - Зафиксированы полный commit SHA, tree SHA и локальный ancestry snapshot
  - Описаны стек, Universal-состав сборки и измеренная Windows/Linux поверхность
  - Повторно проверены SHA-256 и структура существующего диагностического DMG
  - Отдельно записаны ограничения: артефакт не подписан для распространения и не notarized

- [x] FORK-027 Утвердить поддержку процессоров и минимальную версию macOS #product #decision !crit @blocked_by:FORK-026
  Рекомендуемый старт: Universal, Intel и Apple Silicon.

  **Implemented:**
  - Утверждено одно Universal-приложение с нативными `x86_64` и `arm64` срезами
  - Минимальная поддерживаемая версия зафиксирована как macOS 14.2 Sonoma
  - Build evidence отделено от физического runtime smoke на Intel и Apple Silicon
  - Пересмотр Intel привязан к данным пользователей и поддержке toolchain без выдуманной даты

- [ ] FORK-028 Выбрать новое имя продукта, bundle ID и имя репозитория #branding #decision !high @blocked_by:FORK-026 #user-required #milestone @blocked_by:FORK-155 @blocked_by:FORK-156
  **Оставшаяся работа по декомпозиции 2026-09-22:** Зафиксировать выбранные имя, bundle ID и repository slug; Определить момент выхода из fork network. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Отдельно решить, когда выходить из GitHub fork network.

  **Parking note:**
  - Владелец отложил финальные display name, slug, bundle ID, имя репозитория и выход из GitHub fork network до конца текущей цели; новых naming-вопросов сейчас не требуется.
  - Временная внутренняя integration-line может сохранять `Meetily`, `meetily` и `com.meetily.ai` только как рабочую legacy identity, без признания её финальным брендом.
  - `FORK-036`, `FORK-037`, `FORK-046` и любая публикация остаются заблокированы финальным решением `FORK-028`.
  - Live-проверка товарных знаков, доменов и GitHub availability не проводилась; доступность не подтверждена.

- [x] FORK-029 Зафиксировать правила веток и remotes #git #architecture !high @blocked_by:FORK-026
  Новая разработка через `dev`, релизы через `main`, feature-ветки от `dev`, upstream только read-only.

  **Implemented:**
  - Текущая Git-топология отделена от будущей схемы macOS-продукта без изменения refs, remotes и worktrees
  - Закреплены владельцы, provisional `fork/*` flow до handoff и canonical `feature/*` → `dev` → `main` после него
  - Upstream intake ограничен fetch, ручным review и выборочным переносом с проверкой происхождения
  - Запрещены push в upstream и переписывание общей истории; определены требования к будущей локальной и серверной защите
  - Зафиксированы отдельные гейты изолированной линии `FORK-031` и финального переключения `FORK-054`

- [x] FORK-030 Построить реестр переноса всех PR, веток и сессий #migration #evidence !crit @blocked_by:FORK-026
  Для каждой работы указать `present`, `partial`, `missing`, `superseded` и проверяющий тест.

  **Implemented:**
  - Все 29 WORK-маршрутов получили один консервативный статус и владельца следующей проверки
  - Tracked census содержит только project-scoped pseudonyms, HMAC-токены и агрегаты; raw local evidence хранится вне Git
  - Frozen validation отделена от явных protected evidence и live drift режимов
  - Только immutable baseline и утверждённая platform policy имеют статус `present`

  **Feature-intake extension:**
  - Реестр расширен картой источников для выборочного переноса из [feature intake](../fork/feature-intake.md); закрытый статус FORK-030 сохраняется.
  - Исследование источников завершено, но применимость коллегиного patch к текущей базе остаётся `NOT VERIFIED`; до копирования кода нужен отдельный preflight в каждой FEAT-задаче.
  - Новая карта не содержит закрытых архивных путей или личных локаторов; точные checksum и публичный URL находятся в intake-документе.

- [x] FORK-031 Создать изолированную macOS integration-line без изменения нынешних main и dev #git #migration !crit @blocked_by:FORK-027 @blocked_by:FORK-029 @blocked_by:FORK-030
  Сначала отдельная ветка и worktree, затем явное решение о переключении основных веток.

  **Implemented:**
  - Созданы точные локальные ref `fork/macos-integration` и worktree `.worktrees/macos-fork-integration`
  - Base и closeout линии зафиксированы на immutable commit `1e742fa22030fe5720db6a791407c64eea03ede4`
  - Линия назначена единственной базой и destination для всех продуктовых задач до `FORK-054`
  - `main`, `dev`, remotes и ранее существовавшие worktrees подтверждены без изменений
  - Push и публикация не выполнялись; разрешён только quarantine rollback без удаления

### Удаление чужих платформ

- [x] FORK-032 Удалить Windows и Linux установщики, скрипты и бинарные файлы #macos #cleanup !high @blocked_by:FORK-031
  Удалить MSI, NSIS, DEB, AppImage, PowerShell, BAT и платформенные артефакты.

  **Implemented:**
  - Удалены 34 Windows-скрипта и бинарных ресурса; tracked DEB/RPM/AppImage outputs в baseline отсутствовали; точный product diff содержит 35 путей: 34 удаления и одну правку Tauri config
  - Tauri bundle ограничен `app`/`dmg` и PNG/ICNS-иконками; Windows bundle configuration удалена
  - `fork/macos-integration` и task branch чисто указывают на fast-forward commit `45bebcbf184da4d2b16c48a0415050019becff98`
  - Шесть остаточных путей явно переданы: Linux/Windows audio source в `FORK-034`, два CI workflow в `FORK-035`, Linux build-документ и web favicon exception в `FORK-038`
  - Scoped release-policy: 8/8 PASS; `tauri info` и Xcode gate — `NOT_COMPLETED`, поэтому build/runtime не заявлены; push и публикация не выполнялись

- [x] FORK-033 Удалить Windows/Linux зависимости и build features из Rust #rust #macos !high @blocked_by:FORK-031 @blocked_by:FORK-055
  Удалить Windows/Linux зависимости и features. `x86_64` обязан иметь CPU fallback; Metal/CoreML и другие arm64-ускорения включаются только там, где они поддержаны. Ни одна обязательная Alpha-функция не должна зависеть от Apple-Silicon-only runtime.

  **Implemented:**
  - Exact integration commit `7d5f9c9b470361dc643a6cdb859965c5e1e9d6ec`: один non-merge commit после `FORK-056`, ноль merge-коммитов и ровно 27 проверенных product paths
  - Прямые Rust target-зависимости и build/runtime routing ограничены macOS; ручные CUDA/Vulkan/GPU feature wrappers удалены
  - Intel использует CPU-only Whisper и llama-helper; Metal/CoreML включаются только для arm64, а Metal failure имеет реальный ограниченный переход к одной новой CPU-попытке
  - Fresh integration gates: policy 168/168, Node 334/334, Vitest 34/34, Universal Playwright 15/15, production build 11/11, llama-helper 9/9 и оба helper target checks `PASS`
  - Native Meetily tests/checks — `NOT_COMPLETED` из-за full Xcode/CommandLineTools gate; physical runtime, Universal artifact, package, signing и notarization не заявлены
  - Audio, CI/release, общий absence gate, permissions/lifecycle и legacy Python/FastAPI runtime сохранены за `FORK-034`, `FORK-035`, `FORK-038`, `FORK-051` и `FORK-057`; push, PR, release и публикация не выполнялись

- [x] FORK-034 Свести платформенный audio-код к одной macOS-реализации #audio #macos !crit
  Оставить один публичный `RecordingSession` interface и один macOS capture adapter. Rust владеет start/pause/resume/stop; microphone и system audio сохраняются отдельными дорожками до дальнейшей обработки. Старые реализации удаляются только после переноса живых callers и regression checks.

  **Implemented:**
  - Один Rust-owned `RecordingSession` и один macOS capture adapter заменили прежнюю платформенную поверхность; microphone и system audio сохраняются отдельными Tracks.
  - Product и integration commit: `dbf9bc3e6a019acd6c265cda72b76b4697a0cdf1`, один non-merge commit `feat(macos): unify recording session and capture` с parent/base `d7ca29901995687cca2ab8a0bd0b81aaabca2144`.
  - Diff содержит 57 paths и 23 удаления; binary diff SHA-256: `33ae386562be3579ce9474a32daed7a3716fad873fa788ea1a38844dbb11b56b`. Focused release-policy/macOS audio 200/200 и `pnpm --dir frontend check:policy` прошли `PASS`; product и integration worktrees чисты и указывают на этот commit.
  - Prior candidate SPEC и QUALITY Review 7: `PASS`, Critical 0 / Important 0 / Minor 0.
  - Native `cidre` runtime и full Xcode: `NOT_COMPLETED`; TCC и physical microphone/system capture: `NOT_RUN`; Universal package, signing и notarization: `NOT_COMPLETED`. Push, PR, release и publication не выполнялись.

- [ ] FORK-035 Перевести CI и release pipeline на macOS-only #ci #release !high @blocked_by:FORK-027 @blocked_by:FORK-031 @blocked_by:FORK-032 @blocked_by:FORK-033 @blocked_by:FORK-057 @blocked_by:FORK-061 #milestone @blocked_by:FORK-145 @blocked_by:FORK-146 @blocked_by:FORK-147
  **Оставшаяся работа по декомпозиции 2026-09-22:** Внести macOS-only jobs по утверждённому плану; Проверить secretless Universal build и package; Проверить отделение подписи и финального draft job. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Проверять Universal `x86_64` + `arm64` при minimum deployment target macOS 14.2 и собирать только `.app` и `.dmg`.

  **План готов, исполнение не начато:**
  - Reviewed plan SHA-256 `a1d57ae7e1e1c418e4c90d9495af402c1f078a44606eb0e8b44c8e6cb6db2637`; независимый review — Critical 0 / Important 0
  - Исторический STOP на FORK-034/057 снят их завершением; актуальный вход — FORK-061: сверка текущей базы, обновление плана и независимое ревью. Повторно выполнять FORK-034/057 не нужно.
  - План разделяет Universal build, secretless verification, signing/notarization environments и последний draft job; workflow dispatch, signing, notarization и publication не выполнялись

- [ ] FORK-036 Настроить новый подписанный updater #update #security !crit @blocked_by:FORK-028 @blocked_by:FORK-056 #milestone @blocked_by:FORK-157 @blocked_by:FORK-158 @blocked_by:FORK-159 @blocked_by:FORK-173
  **Оставшаяся работа по декомпозиции 2026-09-22:** Подключить update channel и проверку пакета; Добавить управление обновлением без конфликта с записью; Проверить rollback и недоверенный пакет; Подтвердить настоящий подписанный update. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  После выбора identity настроить новый подписанный update channel, проверку подписи, архитектуры и rollback. До выполнения задачи updater остаётся выключенным результатом `FORK-056`.
  Свежая read-only проверка `security find-identity -v -p codesigning` на том же хосте вернула `0 valid identities found`; поэтому Developer ID signing, notarization и публикация updater не могут быть подтверждены без внешней identity/секретов. Никаких ключей и внешних сервисов не использовалось.

  **Feature-intake extension:**
  - Учесть сценарии коллег: отложить, пропустить, показать прогресс и ошибку, запретить update во время записи.
  - Сценарии проверяются на нашей identity и подписи; скачанный shell-установщик и обход quarantine не переносятся.

- [ ] FORK-037 Спроектировать перенос пользовательских данных при смене bundle ID #storage #migration !crit @blocked_by:FORK-028 #milestone @blocked_by:FORK-123 @blocked_by:FORK-124 @blocked_by:FORK-125 @blocked_by:FORK-160
  **Оставшаяся работа по декомпозиции 2026-09-22:** Подключить пользовательский WebView export/import к готовым командам; Проверить полный перенос контрольного профиля; Проверить crash/retry и отказ переноса; Подтвердить upgrade между старым и новым bundle ID. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Не терять записи, модели, настройки и SQLite при установке нового продукта.
  Подготовлен [проект переноса 2026-09-19, уточнён 20 сентября](../fork/fork037-migration-design-2026-09-19.md): карта данных, безопасная последовательность, восстановление после сбоя и матрица проверок. Добавлен точный список Tauri Stores, localStorage/IndexedDB, внешних notifications и пользовательских templates/selection. Предложены явный WebView export, межпроцессный lock, журнал v1 и ограниченный дисковый бюджет. Независимое ревью уточнений завершено после двух исправлений: документ SHA-256 `6a4a7c690e4e631f947f9acd9ce79fd0b9bd855b7e979a772e555dc35e46a48e`, существенных замечаний в проверенной области нет. Состав 12 пользовательских таблиц подтверждён применением 15 SQL migration-файлов к пустой временной SQLite; это не проверка переноса. Core staging layer добавлен и проверен `23/23` Rust-тестами и Clippy: durable journal, no-clobber публикация, lock, checked disk-budget arithmetic, caller-side statvfs free-space measurement and rejection и SQLite main/-wal/-shm snapshot с close/reopen integrity, а также read-only startup guard для незавершённых operation journals включены, включая отсутствующие необязательные каталоги. Реальный IndexedDB source/sink adapter добавлен и покрыт `12/12` WebView-тестами с transactional destination marker, collision-safe записью и rollback localStorage. Публичные migration commands и реальный bundle-ID upgrade ещё не выполнены; startup guard подключён до открытия базы и блокирует частично опубликованный профиль. Final identity отложена; задача остаётся открытой.

  Продолжение 2026-09-21: no-clobber publish/readback primitive FORK-037 добавлен поверх staging и startup guard; migration suite `24/24`, строгий Clippy и полный gate прошли (402 Node, 61 Vitest, 26 Playwright, 461 Rust (5 ignored), FFmpeg 1/1). Публикация копирует только проверенные staged paths через `create_new`, повторная публикация существующего destination отклоняется, а отдельный readback сверяет размер и SHA-256 каждого файла. Текущий Universal diagnostic package привязан к fingerprint `36bcf7a6…`, bundle `d329b225…`, DMG `6f4a3709…`; public migration commands, final identity и реальный bundle-ID upgrade ещё не выполнены.
  Продолжение 2026-09-21: startup guard расширен на `is_first_launch`, обычное открытие и legacy import/fresh-init; регрессионный тест закрывает все три database entry points, а сам guard выполняется до уведомлений и моделей. На fingerprint `c458acd7…` полный gate дал Node `402`, Vitest `61`, Playwright `26`, Rust `468` (5 ignored), Clippy, production build и FFmpeg recovery `1/1`; permission lifecycle `12/12`, coordinator Rust `7/7`. Universal diagnostic package имеет bundle `4ea930f4…`, DMG `b1739230…`, `hdiutil verify` и `dmg.bundle-match` PASS. Публичные migration commands, final identity и реальный bundle-ID upgrade остаются открытыми; `.app` не запускался.
  Продолжение 2026-09-21: после исправления IndexedDB-тестовых фикстур на schema version `3` полный gate на fingerprint `eac035c8…` снова прошёл: Node `402`, Vitest `61`, critical Playwright `26`, Rust `468` (5 ignored), Clippy, production build и FFmpeg recovery `1/1`; отдельная IndexedDB/recovery проверка `10/10`. Universal diagnostic package пересобран: bundle `ef598cea…`, DMG SHA-256 `4bd2376b…` (`157174193` bytes), `hdiutil verify` и `dmg.bundle-match` PASS. WebView migration теперь читает meetings/transcripts одной readonly-транзакцией, связывает marker с operation/source/target/payload identity и исключает selected-template key из общего allowlist. Публичные migration commands и реальный bundle-ID upgrade всё ещё не выполнены; `.app` не запускался.

  Продолжение 2026-09-21: зарегистрированы Tauri-команды `prepare_profile_migration`, `publish_profile_migration` и `get_profile_migration_status`. Prepare фиксирует явные source/target identity, manifest, SHA-256, бюджет места и durable journal; publish удерживает lock, переводит фазы `publishing → verified → completed`, а после сбоя продолжает через resumable readback. Staging вынесен в sibling-каталог на той же файловой системе, чтобы не пересекаться с target app-data. Командный prepare→publish→retry тест прошёл; migration group `33/33`, strict Clippy PASS. Пользовательский WebView export/import, final bundle ID и реальный старый-to-новый upgrade остаются открытыми; `.app` не запускался.
  Продолжение 2026-09-21: точный локальный commit `1ae518a1c90e3175539ac03ab286b5519d7e28fe` и source fingerprint `4bfe56ffe5396445cec3957248a066cce52e22f9f1f7f4b284be4548df991b5e` прошли полный gate: Node `402`, Vitest `61`, Playwright `26`, Rust `469` (5 ignored), Clippy, production build и FFmpeg recovery `1/1`. Diagnostic Universal package имеет bundle fingerprint `dca8c6bce6ec94e93b30644facf8216185cbb3e26890dad428d49f32d491ab2c`, DMG SHA-256 `d503d07ee18ccbc7a831c32899f3df685f51bc01b9b6821e199ab3a722b5f444`; `hdiutil verify` и `dmg.bundle-match` PASS. Релизный trust, Developer ID, notarization и native `.app` smoke не подтверждены; пакет не является Alpha.

  **Feature-intake extension:**
  - Добавить к плану контрольные данные для встреч, шаблонов, говорящих, настроек, прерванной обработки и заполненного диска.
  - Проверять старый каталог и SQLite без потери данных; неизвестную несовместимость не обходить молча.

- [ ] FORK-038 Добавить проверку отсутствия Windows/Linux поверхности #verification #macos !high @blocked_by:FORK-032 @blocked_by:FORK-033 @blocked_by:FORK-034 @blocked_by:FORK-035 @blocked_by:FORK-057 #milestone @blocked_by:FORK-148 @blocked_by:FORK-149
  **Оставшаяся работа по декомпозиции 2026-09-22:** Добавить ограниченный scanner платформенной поверхности; Проверить итоговый source и package. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Gate должен находить старые targets, скрипты, пути, документацию, платформенные условия и legacy Python/FastAPI product runtime.

### Перенос наших функций

- [ ] FORK-039 Проверить и закрепить semantic theme из PR 556 #theme #migration @blocked_by:FORK-031 @blocked_by:FORK-062 #milestone @blocked_by:FORK-152 @blocked_by:FORK-153
  **Оставшаяся работа по декомпозиции 2026-09-22:** Закрепить semantic theme по утверждённому плану; Проверить prepaint, hydration и контраст. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Сохранить поведение system/light/dark и контраст, не считать это финальным дизайном.

  **Историческая граница rev3; текущий статус см. допуск FORK-152 выше:**
  - Plan rev3 SHA-256 `076e8cdb505d284b81d5382a1575faa1b3047799bacbad059f26360b307182a3`
  - Rev3 закрывает ThemeScript/layout/BlockNote census, initial/reload prepaint matrix, hydration diagnostics, реальный focus contrast 3:1 и macOS accessibility ledger
  - Повторный независимый review rev3 не состоялся из-за внешнего usage limit; последний завершённый review относится к rev2 и остаётся `FAIL`, поэтому implementation не разрешён
  - Playwright baseline — `NOT_COMPLETED` из-за sandbox loopback `EPERM`; product/Git не менялись

- [ ] FORK-040 Проверить полный сценарий Gemma 4 из PR 593 #models #migration !high @blocked_by:FORK-031 @blocked_by:FORK-033 @blocked_by:FORK-057 #milestone @blocked_by:FORK-133 @blocked_by:FORK-134 @blocked_by:FORK-167
  **Оставшаяся работа по декомпозиции 2026-09-22:** Сверить Gemma-путь и повторно использовать готовый downloader; Проверить локальную генерацию и настройки на Intel; Подтвердить Gemma runtime на Apple Silicon. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Скачивание, точный SHA-256, архитектурная совместимость, атомарная установка, выбор модели, шаблон, RAM-рекомендации и локальная генерация.

- [ ] FORK-041 Проверить новую diarization-реализацию вместо прямого переноса PR 598 #diarization #migration !high @blocked_by:FORK-031 @blocked_by:FORK-034 @blocked_by:FORK-057 @blocked_by:FEAT-071 @blocked_by:FORK-049 #milestone @blocked_by:FORK-137 @blocked_by:FORK-138 @blocked_by:FORK-139
  **Оставшаяся работа по декомпозиции 2026-09-22:** Сравнить diarization-кандидатов и выбрать подход; Проверить runtime и согласованность live/post-call; Проверить имена и повторное открытие встречи. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Runtime, назначение спикеров, переименование, сохранение и повторное открытие встречи.

  **Feature-intake extension:**
  - Сравнить существующий sherpa-процесс с публичным donor-подходом на воспроизводимом наборе встреч; отделить источник `microphone/system` от личности и оставить одновременную речь проверяемым сценарием.
  - Сохранить ручные имена и явно показывать неопределённую связь; live и post-call должны выдавать согласованный результат.

- [ ] FORK-042 Добавить видимые настройки diarization #diarization #ui !high @blocked_by:FORK-041 #milestone @blocked_by:FORK-140 @blocked_by:FORK-141 @blocked_by:FORK-170
  **Оставшаяся работа по декомпозиции 2026-09-22:** Добавить видимую настройку post-call diarization; Добавить ручную связь говорящего с дорожкой; Проверить переименование и повторную обработку. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Пользователь должен включать и отключать post-call обработку без скрытых команд.

  **Feature-intake extension:**
  - Настройки должны покрывать связь говорящего с отдельной дорожкой и ручное исправление, не создавая постоянных людей из `You`, `Guest` или `Speaker N` автоматически.
  - Проверить, что переименование говорящего в одной встрече не меняет человека во всех встречах и что повторная обработка не затирает результат diarization.

- [ ] FORK-043 Проверить выбор папки записей вместо прямого переноса PR 664 #recording #migration !high @blocked_by:FORK-031 @blocked_by:FORK-055 @blocked_by:FORK-063 #milestone @blocked_by:FORK-128 @blocked_by:FORK-129 @blocked_by:FORK-130 @blocked_by:FORK-131
  **Оставшаяся работа по декомпозиции 2026-09-22:** Добавить выбор корня и относительные пути; Проверить start/restart/import и атомарное сохранение; Проверить конкуренцию, отмену и recovery; Проверить перенос каталога, нехватку места и старую схему. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Новая запись, повторный старт, импорт, конкурентное сохранение и восстановление. SQLite хранит относительный путь и состояние; запись файла завершается атомарным rename, повторный запуск не создаёт дубль.

  **Feature-intake extension:**
  - Для проверки сохранности данных добавить старые встречи, шаблоны, связи говорящих, настройки, прерванную обработку, заполненный диск и перенос каталога.
  - Отмена и повторный запуск не должны уничтожать данные; совместимость схемы проверяется явно, неизвестную миграцию нельзя пропускать молча.

  **План подготовлен, финальный review не завершён:**
  - Plan rev2 SHA-256 `2bda589971d6e6cf6bfbbc4e170325afc1b34db2b64a058263be880886620689`; provisional allowlist 53 paths
  - План добавляет durable root registry, relative path/revision model, crash journal, no-clobber publication, scoped import cancellation и descriptor/inode проверки
  - Повторный независимый review rev2 не завершён; последний завершённый review относится к rev1 и остаётся `FAIL`, поэтому implementation не разрешён
  - Исторический STOP на FORK-034 снят её завершением; актуальный вход — FORK-063: сверка текущих recording/storage seams, обновление плана и fresh SPEC/QUALITY PASS.

- [x] FORK-044 Перенести исправление коротких фраз Whisper из PR 676 #whisper #bug !crit @blocked_by:FORK-031
  Длина текста не должна использоваться как ложная уверенность распознавания.

  **Implemented:**
  - Exact integration commit `d7ca29901995687cca2ab8a0bd0b81aaabca2144`: один non-merge commit после `FORK-033`, ноль merge-коммитов и ровно один product path, `frontend/src-tauri/src/audio/transcription/worker.rs`
  - Любой непробельный результат, включая короткие фразы, отправляется независимо от confidence; строки только из пробельных символов остаются подавлены
  - Confidence больше не управляет отправкой, но сохраняется как metadata в `TranscriptUpdate`
  - Fresh integration gates: независимый Rust harness 3/3, policy 168/168 и полный Node-набор 334/334 `PASS`
  - Native Meetily Cargo compile/test — `NOT_COMPLETED`, потому что `cidre` требует полный Xcode, а активны только Command Line Tools
  - Capture и область `FORK-034` не менялись; physical runtime, push, PR, release и публикация не выполнялись

- [ ] FORK-045 Пересмотреть Whisper streaming из PR 605 после стабилизации форка #whisper #architecture @blocked_by:FORK-038 @blocked_by:FORK-044 @blocked_by:FORK-051 @blocked_by:FORK-057 @blocked_by:FEAT-064 #milestone @blocked_by:FORK-183 @blocked_by:FORK-184 @blocked_by:FORK-185
  **Оставшаяся работа по декомпозиции 2026-09-22:** Пересмотреть streaming-контракт и риски; Проверить фразы, паузы и ресемплинг; Сравнить Intel/arm64 и принять решение. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Сначала закрыть замечания по памяти, Intel fallback и жизненному циклу записи. Использовать bounded queues, monotonic sequence, `sessionId`/`jobId` и ordered Tauri Channel; глобальные events не являются transport для аудиочанков.

  **Feature-intake extension:**
  - В обзор добавить короткие и длинные фразы: разбиение по паузам, ожидание продолжения, преобразование частоты и расширенные настройки только после проверки их пользы.
  - Приёмка требует отсутствия потери концов и дублей, а также отдельных измерений точности, задержки и нагрузки на Intel/arm64; закрытый FORK-044 не переоткрывать.

### Новый продукт

- [ ] FORK-046 Подготовить macOS UI specification по Apple HIG #design #macos !high @blocked_by:FORK-028 @blocked_by:FORK-039 #milestone @blocked_by:FORK-154 @blocked_by:FORK-161
  **Оставшаяся работа по декомпозиции 2026-09-22:** Описать структуру окна и пользовательские состояния; Уточнить команды и accessibility, применить identity. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Перестраиваемое окно, список встреч, транскрипт, summary inspector, toolbar, меню и отдельные Settings.

  **Feature-intake extension:**
  - Спецификация должна охватить системное меню и команды, клавиатуру, окна и панели, Settings, sidebar, VoiceOver, контраст, светлую/тёмную тему и уменьшение анимации.
  - Выбор Tauri/AppKit/SwiftUI остаётся отдельным прототипом по пользовательскому результату; переписывание интерфейса заранее не обещается.

- [ ] FORK-047 Реализовать системное меню, команды и поведение окон macOS #design #macos @blocked_by:FORK-046 @blocked_by:FEAT-071 #milestone @blocked_by:FORK-162 @blocked_by:FORK-163 @blocked_by:FORK-164
  **Оставшаяся работа по декомпозиции 2026-09-22:** Подключить системное меню и keyboard shortcuts; Реализовать окна и компактную панель; Проверить команды и окна в macOS. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Каждая toolbar-команда должна иметь эквивалент в меню и клавиатурный shortcut.

  **Feature-intake extension:**
  - Подключить хоткей и компактную панель к одной существующей сессии: start, pause, resume и stop используют общий timer и общий state.
  - Повторная команда не запускает вторую запись; права и ошибки одинаково видны во всех окнах; внешний вид следует принятой спецификации FORK-046.

- [ ] FORK-048 Пересобрать основные пользовательские сценарии в новом дизайне #design #product !high @blocked_by:FORK-042 @blocked_by:FORK-043 @blocked_by:FORK-047 #milestone @blocked_by:FORK-165 @blocked_by:FORK-166 @blocked_by:FORK-171
  **Оставшаяся работа по декомпозиции 2026-09-22:** Собрать recording/processing и список встреч; Собрать transcript/speakers/summary и существующий экспорт; Проверить сквозной UX, recovery и перетаскивание. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Запись, состояние обработки, транскрипт, спикеры, summary, экспорт и восстановление.

  **Feature-intake extension:**
  - В сценарии включить компактную панель, список встреч, состояние обработки, транскрипт, speaker-поля, summary, export и recovery; перетаскивание и финальные визуальные решения проверять здесь.
  - Сценарии должны использовать общий lifecycle записи и сохранять ручное название, начало встречи и состояние фоновой работы.

- [ ] FORK-049 Создать локальный benchmark моделей на реальных русских встречах #models #research !high @blocked_by:FORK-040 #milestone @blocked_by:FORK-135 @blocked_by:FORK-136 @blocked_by:FORK-168
  **Оставшаяся работа по декомпозиции 2026-09-22:** Зафиксировать корпус и эталоны benchmark; Подготовить измерения и выполнить Intel benchmark; Повторить benchmark на arm64 и свести сравнение. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Сравнивать качество, скорость, RAM, размер и работу без сети на одинаковом наборе аудио.

  **Feature-intake extension:**
  - В одинаковый набор добавить тихую речь, шум, длинные и короткие фразы, перекрытие реплик и отдельные дорожки; сохранять исходные записи для повторной проверки.
  - Результаты должны отдельно показывать ошибки текста, разделение говорящих, задержку, память/CPU и offline-поведение на Intel/arm64.

- [ ] FORK-050 Сформировать roadmap локальных моделей по результатам benchmark #models #roadmap @blocked_by:FORK-049 #milestone @blocked_by:FORK-169
  **Оставшаяся работа по декомпозиции 2026-09-22:** Сформировать выводы и roadmap моделей. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Сначала текущие Whisper и Parakeet, затем эксперимент нового русского провайдера.

- [ ] FORK-051 Закрыть macOS permissions и microphone lifecycle #audio #permissions !crit @blocked_by:FORK-034 @status:in-progress #milestone @blocked_by:FORK-119 @blocked_by:FORK-120 @blocked_by:FORK-121 @blocked_by:FORK-122
  Уточнение текущего состояния из локального PRD (сохранено 22 сентября; повторный native-прогон не выполнялся):
  Статус на 2026-09-22 после последнего прогона:
  - Основные изменения FORK-051 находятся в worktree `fork-051-permissions-lifecycle` на `f35d864`; подготовлен целевой набор из восьми файлов. Проверки: Rust capture `44 passed, 0 failed, 1 ignored`, recording session `37/37`, Node `391/391`, Vitest `40/40`, TypeScript, `cargo fmt` и проверка diff — успешно. Playwright: `101/102`, один известный сбой `theme-foundation`. Сборка Tauri прошла; упаковка DMG остановилась в `bundle_dmg.sh`.
  - Подтверждён один полный запуск точного release `.app`: создана завершённая mic-only запись при отсутствии системных сэмплов, checkpoint-файлы убраны, запись и строка SQLite сохранены, обычный Quit завершился с кодом `0`, в логе есть `Application cleanup complete`. Реальный TCC-прогон подтвердил цепочку «запрещено → включено в Settings → повторный запрос»: `request_granted=true`.
  - Физический USB-микрофон реально переинициализирован через IOKit, но отключение и повторное подключение во время активной записи Meetily не подтверждено. Матрица TCC в UI Meetily также не закрыта полностью.
  - Текущий exact `.app` после последних изменений не дошёл до UI: на macOS 15.8 он падает внутри `tao` в `did_finish_launching`. Временный no-tray/patch эксперименты запускали только диагностический debug-бинарник и не являются финальным доказательством; их нельзя оставлять в итоговом форке.
  - Работа остановлена по запросу пользователя. Коммит и push не выполнялись. В worktree остаются целевые staged-изменения и временные unstaged платформенные диагностические правки; перед продолжением их нужно отделить и вернуть дерево к чистому FORK-051.
  Следующее действие: сверить целевые и диагностические правки в отдельном FORK-051 checkout; они не включены в чистую базу 61e890f и не разрешают закрыть native-детей.
  Перепроверка 2026-09-23 на combined candidate: указанный ниже в FORK-119 `.app` был открыт штатно через Launch Services; `ps` подтвердил живой PID `84731` по точному bundle executable, а `CGWindowListCopyWindowInfo(.optionOnScreenOnly, ...)` обнаружил видимое окно `meetily` размером 1100×700. Это показывает, что конкретный combined candidate дошёл до окна и не воспроизводит прежний стартовый сбой; вывод относится только к этой сборке и не заменяет запись/permission/device сценарии. Контролы и содержимое окна, microphone/system capture, TCC deny/recovery, device reconnect и quit/relaunch не проверялись; TCC не менялся. Exact hashes и native test matrix — в [отчёте FORK-119](../fork/fork119-artifact-selection-2026-09-23.md). FORK-051 остаётся `in-progress` до FORK-120–122 и нативного lifecycle acceptance.
  **Оставшаяся работа по декомпозиции 2026-09-22:** Зафиксировать exact .app и план native-проверок; Проверить TCC deny→Settings→recovery; Проверить физическую смену и возврат устройства; Подтвердить stop/Quit/relaunch и закрыть lifecycle. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Проверить первый запуск, TCC-разрешения, stop/start, смену устройств и восстановление после ошибки.
  Продолжение 2026-09-14: [свежие проверки и область переноса](../fork/fork051-feat071-verification-2026-09-14.md). Исходный `f35d864` проверен заново; автоматическая часть переноса на базу `fcfa6e0` и исправления после ревью прошли на отдельной ветке `themaddoxed/feat071-audio-controls`. Физические сценарии разрешений, отключение устройства и упакованное приложение пока не проверены; задача не закрыта.
  Продолжение 2026-09-19: [проверка диагностического приложения и текущих исходников](../fork/urgent-verification-2026-09-19.md): TypeScript, Node 397/397, Vitest 48/48, policy и целостность подписи PASS. Наблюдение реальных разрешений и записи через `.app` всё ещё ожидается; статус не изменён.
  Дополнительный прогон: [собранные ошибки и доказательства](../fork/audio-errors-2026-09-19.md). Rust вне sandbox: 393 passed / 4 failed / 3 ignored; три стабильных сбоя в тестовых сценариях записи и один нестабильный тест загрузок. Ложный PASS отсутствующего аппаратного теста не засчитан. Нативный CoreAudio capture прошёл отдельно; запись через интерфейс не подтверждена.
  Последующее нативное продолжение: один цикл start/pause/resume/stop в точном `.app` завершён и сохранил обе дорожки; повторный старт подтверждён. Предыдущая тестовая встреча имеет `interrupted` / `Recording did not stop cleanly`; причина ещё не установлена. Подробности и контрольные суммы добавлены в тот же отчёт. Полная матрица FORK-051 не закрыта.
  Повтор после штатного перезапуска: запись снова завершилась без Stop агента через 32.597s, около первого checkpoint. Исправлены три дефекта тестового окружения записи: 40/40 тестов RecordingSession прошли; браузерная матрица разрешений 11/11. Защита smoke от нуля тестов и проверка именно output-устройств независимо проверены; аппаратный прогон обновлённого smoke ожидается. Диагностика реальной остановки продолжается, статус `in-progress` сохранён.
  2026-09-20: в отдельном 35s тесте реального сохранения воспроизведён `TrackBackpressure(Microphone)` на 30.64s; добавлен запас очереди raw-дорожек при прежнем ограничении 8 MiB. Исправленный сценарий и 203 аудиотеста прошли. Нативная проверка новой сборки и независимое review этой правки ожидаются; подробности в отчёте ошибок, статус сохранён.
  Продолжение: исправлено распознавание готового audio.mp4 после удаления checkpoints; неоднозначные/повреждённые данные сохраняются, публикация восстановления не заменяет существующий файл. Итоговый Rust-набор 404/404, recovery hook 16/16; независимая read-only оценка обеих правок без blocker. Общая диагностическая сборка собрана; реальный цикл записи ещё ожидает разблокировки, FORK-051 остаётся открыт.

  Нативное продолжение 2026-09-20: после разблокировки проверен `.app` с SHA executable `0121175…`; запись прошла три 30s checkpoint, pause/resume/stop, metadata completed, три MP4 декодируются. Исправление прежнего обрыва подтверждено в этом цикле. Найдены отдельные ошибки: первый старт попал в timeout CoreAudio, живой счётчик не обновлялся; исправление счётчика прошло 41 тест и независимое review, затем проверку в новой `.app` (рост → пауза → продолжение → completed). Первый системный запрос воспроизводимо превышает внутреннее ожидание 5s. Дополнительно обычный Quit во время записи обходит финализацию: два нативных воспроизведения, metadata остаётся recording. Подготовка разрешений и безопасное завершение при выходе исправлены совместно, прошли независимое review и 408 Rust-тестов. Новая `.app` SHA `fc63eca…` проверена после разблокирования: выход при незавершённом запросе микрофона и сохранение при Quit прошли в запуске через инструмент разработчика. Дополнительно обычный запуск через macOS подтвердил отказ запрещённому микрофону и запись после восстановления доступа; выявлены сохранение старой ошибки интерфейса и сдвиг системной дорожки около 19.5s на контрольных сигналах. Прежние проверки разрешений через прямой запуск исполняемого файла не доказывают разрешения самостоятельного `.app`. Полная матрица разрешений/смены физических устройств ещё не закрыта.

  Продолжение 20 сентября, 16:49: на fingerprint `52a49335…` собрана и нативно проверена новая `.app` executable `bb539181…`: выдача разрешений, pause/resume/Stop и отдельный Quit во время записи с финализацией до DB cleanup. Rust531 PASS/10ignored, strict Clippy остаётся FAIL. Свежие MP4 подтверждают оставшийся сдвиг raw system около22s и DTS warnings. Settings0 при ненулевом system.mp4 не доказывает эффективный отказ этой копии; возврат доступа ожидает системного пароля. Актуальные доказательства и границы — в [отчёте ошибок](../fork/audio-errors-2026-09-19.md). Статус не закрыт.

  Финальная нативная проверка 20 сентября, 19:00: exact `.app` на fingerprint `dc4b92b2…` и build executable `a9d4cd9b…` запущен через LaunchServices на разблокированном рабочем столе. Разрешения системного звука и микрофона выданы через `UserNotificationCenter`; Start перешёл в `Recording` примерно за 2 секунды, Pause/Resume дали `Paused`/`Recording`, Stop вернул главный экран. Встреча завершилась с `metadata.status=completed`, `error=null`; `audio.mp4` и `microphone.mp4` декодируются. Асинхронный CoreAudio tap не выдал кадров, поэтому backend закрыл системный источник как необязательный и сохранил микрофонную запись; `system.mp4` отсутствует. Этот цикл закрывает недостающую проверку самостоятельного `.app` для микрофона и lifecycle, но physical device switch, полная TCC-denied/recovery матрица и полноценная system Track остаются открытыми. Артефакт ad-hoc, dirty, `UNGATED`, не Alpha и не release.
  Финальная проверка после исправления задержанного CoreAudio stop: exact `.app` на fingerprint `992eb628…` и packaged executable `2b315890…` запущен через LaunchServices. Две карточки разрешений были разрешены один раз для этой ad-hoc identity; последующих сбросов TCC не выполнялось. Нативный цикл `Start → Pause → Resume → stop/exit cleanup` завершился без зависания, `metadata.status=completed`, `error=null`, `duration_seconds=40.071158257`; CoreAudio system stream стал доступен асинхронно, но не дал кадров, поэтому сохранён microphone-only fallback без `system.mp4`. Автоматические проверки snapshot: capture22/22, RecordingSession48 passed/1 ignored, permissions7/7, release build PASS. Доказательство: [native-audio-2026-09-20-final4.json](../fork/evidence/native-audio-2026-09-20-final4.json). Артефакт остаётся dirty/ad-hoc Intel `UNGATED`; physical device switch, полная TCC-denied/recovery матрица, system-track signals и release signing/notarization не закрыты.
  После исправления test-only bounded-channel helper exact текущий пакет перепроверен на fingerprint `e645918f…` как `run-20260920-permission-fast-fallback-final5/meetily.app`; release executable SHA остался `2b315890…`. Новый нативный запуск намеренно не выполнялся, чтобы не провоцировать ещё один TCC prompt; его граница и ссылка на final4 runtime evidence зафиксированы в [final5 evidence](../fork/evidence/native-audio-2026-09-20-final5.json). Статус задачи не закрыт.
  После разбора policy-сбоя добавлено точное разрешение штатной формы `Command::new(&ffmpeg)` при источнике `find_ffmpeg_path()` и регрессионный тест. Exact package final8 собран из fingerprint `15dd5521…`; `pnpm run check:policy` и агрегированный Node-набор `225/225` прошли. Нативный запуск не повторялся, поэтому физическая смена устройства, полная TCC deny/recovery матрица и system Track остаются открытыми; strict Clippy остаётся `13/16` красным. [Evidence](../fork/evidence/native-audio-2026-09-20-final8.json).
  После точечных исправлений Clippy exact package final9 собран из fingerprint `d69e40bd…`; строгий release Clippy, `pnpm run check:policy`, агрегированный Node `225/225`, Rust capture `23/23`, RecordingSession `48 passed/1 ignored`, typecheck и release build прошли. Нативный запуск не повторялся, поэтому физическая смена устройства, полная TCC deny/recovery матрица и system Track остаются открытыми. [Evidence](../fork/evidence/native-audio-2026-09-20-final9.json).
  После исправления terminal-state race в TrackWriter exact package final10 собран из fingerprint `cbf0160c…`; полный release Rust library `433 passed, 0 failed, 5 ignored`, строгий release Clippy, policy, typecheck, Vitest, aggregate Node `225`, release build и codesign verification прошли. Дополнительный `./scripts/check.sh --full` вне sandbox прошёл целиком: frontend Node `401 passed`, critical Playwright `26 passed`, workspace Clippy/tests и packaged FFmpeg recovery `1 passed`. Нативный запуск намеренно не повторялся, поэтому физическая смена устройства, полная TCC deny/recovery матрица и system Track остаются открытыми. [Evidence](../fork/evidence/native-audio-2026-09-20-final10.json), [full gate](../fork/evidence/full-check-2026-09-20-final10.json).
  После этого исправлено найденное смещение времени: microphone и system используют одну active-time шкалу, pause/resume увеличивает epoch без учёта паузы, recovery-gap выравнивается вперёд, а raw saver заполняет forward gaps тишиной и отклоняет перекрытия. На текущем dirty fingerprint `eceffe76…` прошли полный Rust library `437/437` без ошибок (5 ignored), все integration suites `99/99` без ошибок (5 ignored), strict Clippy, capture 25/25 и macOS capture 23/23, saver 25/25 и полный автоматический gate `./scripts/check.sh --full` (включая frontend Node `401`, Vitest `49`, Playwright `26`, production build и packaged FFmpeg recovery `1/1`). `.app` после этой правки намеренно не запускался, поэтому физическая смена устройства, TCC deny/recovery, current system Track и native mute/gain/synchronization остаются открытыми. [Timeline evidence](../fork/evidence/audio-timeline-repair-2026-09-20.json). Статус `in-progress` сохранён.
  После hardening проверки необязательных transcript timing fields в WebView migration protocol core собран новый Universal diagnostic `.app` из канонического dirty fingerprint `690f4478…`: обе архитектуры (`x86_64`, `arm64`), `LSMinimumSystemVersion=14.2`, bundle identity сохранена, глубокая проверка подписи PASS. Полная автоматическая проверка дала Node `402`, Vitest `57`, Playwright `26`, Rust `455/455`, Universal config `15/15`, WebView migration protocol `8/8` и FFmpeg recovery `1/1`. Это только упаковочная и автоматическая проверка; приложение не запускалось, TCC не сбрасывался и новые разрешения не запрашивались. Физическая смена устройства, полный deny/recovery TCC, нативный цикл текущего артефакта и реальный WebView/SQLite upgrade остаются открытыми. [Universal evidence](../fork/evidence/native-universal-2026-09-21.json), [full gate](../fork/evidence/full-check-2026-09-21-universal.json). Статус `in-progress` сохранён.
  Продолжение 2026-09-21: новый dirty fingerprint `c9f8e252…` включает реальный IndexedDB adapter; полный gate дал Node `402`, Vitest `61`, Playwright `26`, Rust `455/455`, migration `18/18`, WebView migration `12/12` и FFmpeg recovery `1/1`. Новый `.app` получил bundle fingerprint `0d8f54e6…` и обе архитектуры, а созданный вне песочницы DMG прошёл `hdiutil verify` и `dmg.bundle-match`. Release eligibility, Developer ID, notarization и bundle-ID upgrade не подтверждены. `.app` не запускался, TCC не сбрасывался; физическая смена устройства, deny/recovery и текущий native system/mute/gain остаются открытыми. Статус `in-progress` сохранён.
  Продолжение 2026-09-21: SQLite WAL snapshot/reopen primitive проверен отдельными `20/20` Rust-тестами: staging trio не меняет источник, не перезаписывает существующий destination и переживает закрытие/immutable reopen. Текущий полный gate на fingerprint `c9f191c6…` прошёл с Rust `457/457` (5 ignored), Node `402`, Vitest `61`, Playwright `26`; текущие `.app`/DMG пересобраны, bundle `de58c037…`, DMG `c037c69f…`, `dmg.verify` и `dmg.bundle-match` PASS. Startup wiring, caller-side free-space measurement и реальный bundle-ID upgrade остаются открытыми; статус `in-progress` сохранён.
  Продолжение 2026-09-21: SQLite WAL snapshot/reopen primitive проверен отдельными `20/20` Rust-тестами: staging trio не меняет источник, не перезаписывает существующий destination и переживает закрытие/immutable reopen. Текущий полный gate на fingerprint `c9f191c6…` прошёл с Rust `457/457` (5 ignored), Node `402`, Vitest `61`, Playwright `26`; текущие `.app`/DMG пересобраны, bundle `de58c037…`, DMG `c037c69f…`, `dmg.verify` и `dmg.bundle-match` PASS. Startup wiring, caller-side free-space measurement и реальный bundle-ID upgrade остаются открытыми; статус `in-progress` сохранён.
  Продолжение 2026-09-21: `statvfs` free-space probe для caller boundary добавлен и проверен на существующем и ещё не созданном destination; migration staging теперь `21/21`. Полный gate на fingerprint `784c1510…` прошёл с Rust `458/458` (5 ignored), Node `402`, Vitest `61`, Playwright `26`; Universal package пересобран, bundle `7a91f315…`, DMG `4ed59742…`, `hdiutil verify` и `dmg.bundle-match` PASS. Startup wiring и реальный bundle-ID upgrade остаются открытыми; статус `in-progress` сохранён.

  Продолжение 2026-09-21: после publish/readback hardening текущий Universal `.app` только инспектировался и прошёл диагностический verifier; запуск через LaunchServices сознательно не повторялся, TCC не сбрасывался и новые permission prompts не вызывались. Автоматический gate и код lifecycle зелёные, но current-artifact system.mp4, mute/gain, physical device reconnect и полный deny/recovery остаются непроверенными; FORK-051 сохраняет `in-progress`. [Evidence](../fork/evidence/native-universal-2026-09-21.json), [full gate](../fork/evidence/full-check-2026-09-21-universal.json).

- [ ] FORK-052 Пройти recovery, offline и upgrade сценарии #recovery #smoke !high @blocked_by:FORK-037 @blocked_by:FORK-042 @blocked_by:FORK-043 @blocked_by:FORK-051 @blocked_by:FORK-057 #milestone @blocked_by:FORK-132 @blocked_by:FORK-142 @blocked_by:FORK-172
  **Оставшаяся работа по декомпозиции 2026-09-22:** Проверить crash/relaunch capture и transcribe; Проверить recovery diarize и summarize; Проверить offline, filled disk, folder switch и upgrade вместе. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Rust является источником истины для meeting/job state. На отдельном тестовом профиле проверить crash/relaunch на стадиях capture, transcribe, diarize и summarize, offline, смену recording folder и upgrade; повтор не создаёт дубли, а raw audio и завершённые результаты не теряются.

  **Feature-intake extension:**
  - Добавить контрольные данные для старых встреч, шаблонов, говорящих, настроек, прерванной обработки, заполненного диска и смены каталога.
  - Проверить отмену и повторный запуск без уничтожения raw audio и завершённых результатов; граница rollback определяется совместимостью схемы, а не скрытием ошибки.

- [ ] FORK-053 Собрать функциональный macOS Alpha на точном source commit #alpha #build !crit @blocked_by:FORK-035 @blocked_by:FORK-036 @blocked_by:FORK-038 @blocked_by:FORK-039 @blocked_by:FORK-040 @blocked_by:FORK-042 @blocked_by:FORK-043 @blocked_by:FORK-044 @blocked_by:FORK-047 @blocked_by:FORK-048 @blocked_by:FORK-050 @blocked_by:FORK-051 @blocked_by:FORK-052 #milestone @blocked_by:FORK-174 @blocked_by:FORK-175 @blocked_by:FORK-176 @blocked_by:FORK-177 @blocked_by:FORK-178 @blocked_by:FORK-179 @blocked_by:FORK-180
  **Оставшаяся работа по декомпозиции 2026-09-22:** Собрать точную матрицу оставшихся Alpha-проверок; Подготовить чистый проверенный source commit; Собрать и проверить финальный Universal package; Подтвердить подпись и доверие финального пакета; Пройти итоговый native smoke на Intel; Пройти итоговый native smoke на Apple Silicon; Подтвердить минимальную macOS и вынести Alpha verdict. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Автоматические проверки и ручные сценарии должны относиться к одному и тому же артефакту.
  Продолжение 2026-09-21: Universal diagnostic package собран из канонического dirty source fingerprint `46aeebe4…`, с `x86_64+arm64`, `LSMinimumSystemVersion=14.2` и валидной ad-hoc подписью. Developer ID, notarization, чистый точный commit и полный native smoke отсутствуют; пакет не является Alpha и задача остаётся открытой. WebView migration protocol core проверен отдельными `7/7` тестами, но фактический export/import IndexedDB и startup upgrade ещё не подключены. [Evidence](../fork/evidence/native-universal-2026-09-21.json), [full gate](../fork/evidence/full-check-2026-09-21-universal.json).
  Продолжение 2026-09-21: текущий `.app` и DMG пересобраны из dirty fingerprint `c9f8e252…`; проверка ресурсов, архитектур, строгой целостности подписи, `hdiutil verify` и `dmg.bundle-match` прошла, полный gate зелёный (`402` Node, `61` Vitest, `26` Playwright, `455` Rust). Developer ID, notarization, чистый exact commit и полный native smoke отсутствуют; пакет не является Alpha и задача остаётся открытой. [Evidence](../fork/evidence/native-universal-2026-09-21.json), [full gate](../fork/evidence/full-check-2026-09-21-universal.json).
  Продолжение 2026-09-21: после изменения SQLite migration primitive исходный fingerprint стал `c9f191c6…`; Universal package пересобран уже на этом точном dirty состоянии: bundle `de58c037…`, DMG SHA-256 `c037c69f…`, обе архитектуры, строгая целостность подписи, `hdiutil verify` и `dmg.bundle-match` PASS. Проверка release eligibility ожидаемо FAIL из-за diagnostic/dirty provenance, ad-hoc signing, отсутствия Developer ID/notarization и не выполненного native smoke. Пакет не является функциональным Alpha; задача остаётся открытой.
  Продолжение 2026-09-21: после изменения SQLite migration primitive исходный fingerprint стал `c9f191c6…`; Universal package пересобран уже на этом точном dirty состоянии: bundle `de58c037…`, DMG SHA-256 `c037c69f…`, обе архитектуры, строгая целостность подписи, `hdiutil verify` и `dmg.bundle-match` PASS. Проверка release eligibility ожидаемо FAIL из-за diagnostic/dirty provenance, ad-hoc signing, отсутствия Developer ID/notarization и не выполненного native smoke. Пакет не является функциональным Alpha; задача остаётся открытой.
  Продолжение 2026-09-21: актуальный пакет пересобран на fingerprint `e9416fc4…` после подключения read-only startup guard для незавершённых migration journals. Bundle `3b49f7cc…`, DMG `85f626fa…`, обе архитектуры, строгая целостность подписи, `hdiutil verify` и `dmg.bundle-match` PASS; полный gate зелёный (`402` Node, `61` Vitest, `26` Playwright, `460` Rust, FFmpeg `1/1`). Release eligibility остаётся ожидаемо FAIL из-за dirty/diagnostic provenance, ad-hoc signing, отсутствия Developer ID/notarization и native smoke. Пакет не является функциональным Alpha; задача остаётся открытой.
  Продолжение 2026-09-21: текущий Universal diagnostic package пересобран на fingerprint `eac035c8…`; bundle `ef598cea…`, DMG SHA-256 `4bd2376b…`, Universal slices, resources, signature integrity, `hdiutil verify` и `dmg.bundle-match` PASS. Browser permission lifecycle `12/12`, permission coordinator Rust `7/7`, а повторная IndexedDB/recovery проверка `10/10` прошли без запуска `.app`; TCC prompts не вызывались. Current-artifact system.mp4, native mute/gain, pause/resume synchronization, physical device reconnect и полный deny/recovery остаются непроверенными, поэтому статус `in-progress` сохраняется.
  Продолжение 2026-09-21: пакет пересобран после no-clobber publish/readback hardening из dirty fingerprint `36bcf7a6…`. Bundle fingerprint `d329b225…`, DMG SHA-256 `6f4a3709…`; Universal slices, resources, signature integrity, `hdiutil verify` и `dmg.bundle-match` прошли. Full gate: 402 Node, 61 Vitest, 26 Playwright, 461 Rust (5 ignored), FFmpeg 1/1. Diagnostic report `/private/tmp/meetily-universal-report-publish-readback-20260921.json` показывает 8 ожидаемых блокеров: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization и unsigned DMG. Developer ID, notarization, чистый exact commit и current-artifact native smoke отсутствуют, поэтому FORK-053 остаётся открытой и пакет не считается Alpha.
  Отдельный permission lifecycle browser suite прошёл `12/12`: повторный microphone request не появляется после authorization/Settings return, denied system audio остаётся optional, а unconfirmed system audio не запускает новый probe во время записи. Это не заменяет native TCC и current-artifact smoke, поэтому FORK-051 сохраняет `in-progress`.
  Продолжение 2026-09-21: после atomic publish hardening пакет пересобран на dirty fingerprint `f06a6f8d…`; migration suite `30/30`, полный gate `402` Node, `61` Vitest, `26` Playwright, `467` Rust (5 ignored), FFmpeg `1/1`. Bundle `a1c1830c…`, DMG SHA-256 `33a3e4c0…` (`157156920` bytes); Universal slices, resources, signature integrity, `hdiutil verify` и `dmg.bundle-match` прошли. Diagnostic report `/private/tmp/meetily-universal-report-atomic-publish-20260921.json` оставил 8 ожидаемых блокеров: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization и unsigned DMG. Developer ID, notarization, чистый exact commit и current-artifact native smoke отсутствуют; FORK-053 остаётся открытой и пакет не является Alpha.
  Продолжение 2026-09-21: diagnostic package обновлён после WebView migration hardening на fingerprint `eac035c8…`; bundle `ef598cea…`, DMG SHA-256 `4bd2376b…` (`157174193` bytes), `hdiutil verify` и `dmg.bundle-match` PASS. Verifier сохранил ожидаемые 8 отказов: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization и unsigned DMG; Developer ID, notarization, чистый exact source commit и current-artifact native smoke отсутствуют. Пакет остаётся диагностическим и не считается функциональным Alpha; задача открыта. [Universal evidence](../fork/evidence/native-universal-2026-09-21.json), [full gate](../fork/evidence/full-check-2026-09-21-universal.json).

- [ ] FORK-054 Выполнить финальный handoff из Meetily после готовности новой линии #github #migration !high @blocked_by:FORK-053 #milestone @blocked_by:FORK-181 @blocked_by:FORK-182
  **Оставшаяся работа по декомпозиции 2026-09-22:** Подготовить read-only handoff с перечнем веток и PR; Выполнить отдельно разрешённый handoff и readback. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Заархивировать старые ветки, прокомментировать PR и закрыть их только после подтверждённого переноса.

### Архитектурное укрепление до handoff

- [x] FORK-055 Зафиксировать контракт модульного монолита и Universal capability matrix #architecture #macos !crit @blocked_by:FORK-032
  Rust является единственным владельцем lifecycle встречи и фоновых стадий; UI обращается к нему через Tauri-команды и упорядоченные каналы с `sessionId` и `jobId`.
  Зафиксировать interfaces `RecordingSession`, `CaptureBackend`, `TranscriptionEngine`, `DiarizationEngine`, `SummaryEngine`, `MeetingRepository` и `ModelRegistry`, состояния и ошибки, разделение SQLite/файлов, а также обязательный Intel CPU fallback и опциональное ускорение Apple Silicon.
  Legacy `Meetily` identity остаётся временной и не становится финальным названием.

  **Implemented:**
  - `fork/macos-integration` и task branch чисто указывают на exact commit `e695a41f2aacab2c61c6ce7e2e4b57b1bf85f298`
  - Точный range после `FORK-032` содержит 3 tracked documentation files и 421 insertions: 3 commits без merge commits
  - Утверждён amended plan SHA-256 `3a33624b11113ab0ef3dcb42983160c8fd3a25e7db6dc3353d75b0afa2d46c9a`; SPEC-review и QUALITY-review — `PASS`
  - Scoped release-policy: 8/8 `PASS`
  - Зафиксирован только архитектурный контракт: runtime, build, физическое hardware, signing и notarization не подтверждены; push и публикация не выполнялись

- [x] FORK-056 Отключить legacy Meetily updater до выбора новой identity #update #security !crit @blocked_by:FORK-032
  Убрать автоматические и ручные обращения к старому update channel, отключить updater capability/runtime registration и генерацию updater artifacts.
  Приложение должно запускаться и работать offline без сетевого запроса обновлений; новый endpoint, signing key, имя и публикация в эту задачу не входят.

  **Implemented:**
  - Удалены frontend updater UI/runtime, Tauri updater/process registration, dependencies, capabilities, endpoints, keys и updater artifact generation
  - Удалены updater-specific scripts и workflow signing/manifest surfaces; Apple application signing и provider/model network paths сохранены
  - Добавлен fail-closed semantic policy для TOML, JavaScript/TypeScript, YAML, Tauri config/capabilities и updater-owned paths
  - Exact integration commit `261947797fc713bb909d5293a19cecb1210cfae1`: четыре non-merge commits, исходные 33 пути, независимый review Critical 0 / Important 0
  - Fresh post-commit gates: policy 125/125, Node 291/291, production build 11/11, integration policy 125/125; browser 10/10 остаётся browser-only evidence
  - Cargo check/test и native/package/signing/notarization не заявлены: full Xcode на хосте не выбран; push, PR, release и публикация не выполнялись

- [x] FORK-057 Удалить legacy Python/FastAPI product runtime #architecture #cleanup !high @blocked_by:FORK-034 @blocked_by:FORK-060
  Перенести оставшиеся живые сценарии `localhost:5167` и `127.0.0.1:8178` на Rust modules из архитектурного контракта, затем удалить FastAPI/Docker server runtime, старые config/commands и их CSP-разрешения.
  Сохранить только явно поддержанные внешние provider endpoints, например Ollama/OpenAI-compatible, и Python build/dev tools, если они не входят в поставляемое приложение.
  `.app` должен работать offline без Python, Docker и локального HTTP-сервера.

  **Implemented:**
  - Один non-merge product commit `b7126ea82be887c38e8a8e8de30a48f9bb409e51` с parent `dbf9bc3e6a019acd6c265cda72b76b4697a0cdf1` и tree `7673c05cf7bcd09f5b38eb702a8b8251dad9507e` создан из exact reviewed cache и fast-forward интегрирован в `fork/macos-integration`.
  - Commit содержит ровно 53 path = 2 additions + 18 modifications + 33 deletions; binary diff SHA-256 `bb9e2dd0ffdc29e4b58cdbcc282cd80d06db9c1262a10589ab20df372279f5c6`. Удалены tracked FastAPI/Docker backend, gitlink, legacy config, пять zero-caller HTTP commands и CSP-порты 5167/8178; все 15 живых route equivalents остались native Rust/Tauri commands.
  - Ledgerless Python SQLite и старые SQLx базы мигрируются через WAL-aware read-only snapshot, semantic schema classifier, полный typed digest и durable crash journal. Source/current/legacy main/WAL/SHM защищены от alias/symlink/hard-link mutation; first-launch, recovery, init и import используют общий gate, а live managed pool запрещает повторную установку или замену базы.
  - Поддерживаемые Ollama, OpenAI-compatible и hosted provider endpoints, FFmpeg, llama-helper и `scripts/inject_transcript.py` сохранены; единственный production `backend/meeting_minutes.db` locator ограничен migration-only кодом.
  - На exact candidate прошли: 11 GREEN-DB + 2 recovery-gate + first-launch regression по 1/1, Sidebar 1/1, policy 2/2 и 180/180, repository policy, typecheck, Node 371/371, Vitest 34/34, Next 11/11, format, Playwright 26/26 и `.app` build. На integration повторно прошли те же 14 DB/recovery filters, Sidebar и policy 2/2, 180/180, repository policy.
  - По выбранному варианту 1 отдельно приняты только exact baseline-equivalent nonzero результаты: lint 4, Clippy 10 и четыре старых `audio::recording_session` failures; новых путей, категорий или ошибок нет. Независимые final SPEC и QUALITY reviews дали `PASS`, Critical 0 / Important 0 / Minor 0.
  - Собранный `.app` содержит 16 файлов и три thin x86_64 Mach-O, не содержит Python source/interpreter/library, backend/FastAPI/Docker assets, launch strings или 5167/8178 endpoints; подпись ad hoc с hardened runtime и без TeamIdentifier. Packaged interactive smoke, physical capture, disconnected all-provider behavior, Universal slices, Developer ID signing, notarization, release, push и публикация — `NOT_RUN` и не заявлены.

### Ближайшая связанная пачка

- [x] FORK-058 Зафиксировать и fast-forward интегрировать проверенный кандидат FORK-034 #audio #git #integration !crit
  Ожидаемый результат: проверенные байты FORK-034 оформлены одним локальным product-коммитом и без изменения содержимого fast-forward интегрированы в `fork/macos-integration`.

  **Критерии приёмки:**
  - Перед коммитом staged diff содержит ровно 57 путей и 23 удаления, без unstaged и untracked product-файлов.
  - `git diff --cached --check` проходит, SHA-256 binary diff равен `33ae386562be3579ce9474a32daed7a3716fad873fa788ea1a38844dbb11b56b`.
  - Создан ровно один non-merge commit `feat(macos): unify recording session and capture` с parent `d7ca29901995687cca2ab8a0bd0b81aaabca2144`.
  - Task-worktree чистый; focused policy/audio 200/200 и `pnpm --dir frontend check:policy` проходят.
  - `fork/macos-integration` передвинут только fast-forward и указывает на точный product-коммит.
  - При любом изменении байтов работа останавливается до повторных SPEC и QUALITY reviews.
  - Push, PR, release и публикация не выполняются.

  **Implemented:**
  - Один non-merge product commit `dbf9bc3e6a019acd6c265cda72b76b4697a0cdf1` с subject `feat(macos): unify recording session and capture` создан от parent `d7ca29901995687cca2ab8a0bd0b81aaabca2144`.
  - В commit ровно 57 changed paths и 23 deletions; binary diff SHA-256 равен `33ae386562be3579ce9474a32daed7a3716fad873fa788ea1a38844dbb11b56b`.
  - Focused release-policy/macOS audio 200/200 и `pnpm --dir frontend check:policy` прошли `PASS`; product и integration worktrees чисты и равны exact commit после fast-forward.
  - Push, PR, release и publication не выполнялись.

- [x] FORK-059 Закрыть FORK-034 в каноническом PRD #docs #evidence !high
  Ожидаемый результат: foundation PRD содержит точный integration SHA, фактические проверки и честные границы доказательств FORK-034; FORK-034 отмечен выполненным отдельным docs-closeout.

  **Критерии приёмки:**
  - Изменяется только `docs/prd/macos-fork.md`.
  - Записаны точные product/integration SHA, состав diff и реально выполненные проверки.
  - Native runtime, TCC, физический capture, Universal package, signing и notarization остаются `NOT_COMPLETED` или `NOT_RUN`.
  - Проходят `mdtask --path docs/prd/macos-fork.md validate`, migration registry validator и `git diff --check`.
  - Независимый closeout-review имеет Critical 0 / Important 0.
  - Docs-коммит отделён от product-коммита; push и публикация не выполняются.

  **Implemented:**
  - Git-policy и user-facing описание обновлены точным integration commit и моделью одной Rust-owned `RecordingSession` с отдельными persisted Tracks.
  - FORK-034 и FORK-058 закрыты с exact commit, составом diff, проверками и границами доказательств.
  - `mdtask --path docs/prd/macos-fork.md validate`, migration registry validator и `git diff --check` прошли `PASS` перед отдельным docs-коммитом.
  - Push, PR, release и publication не выполнялись.

- [x] FORK-060 Перепривязать и заново утвердить план FORK-057 #architecture #planning #review !high @blocked_by:FORK-059
  Ожидаемый результат: план удаления Python/FastAPI привязан к новому integration tip и снова готов к исполнению.

  **Критерии приёмки:**
  - В плане указан точный новый base SHA.
  - Повторены caller census, path census и allowlist; устаревшие числа и пути удалены.
  - Повторно проверены SQLite/WAL snapshot, typed digest, crash journal, source immutability и совместимость старых баз.
  - Сохранены разрешённые внешние provider endpoints и Python build/dev tools, не входящие в product runtime.
  - Новые независимые SPEC и QUALITY reviews дают Critical 0 / Important 0.
  - Product-файлы не изменяются, выполнение FORK-057 не начинается в этой задаче.

  **Implemented:**
  - Final plan SHA-256 `db3eccd0fecbbb6873f6aeee77141b2fe5ece3b044ea5afc8e14eae667bd4f99` привязан к exact integration base `dbf9bc3e6a019acd6c265cda72b76b4697a0cdf1`.
  - Повторный census зафиксировал 15 routes, 5 zero-caller commands, 31 tracked `backend/` path, 6 Python-schema tables, 12 SQLx migrations и один migration-only legacy locator.
  - Exact allowlist повторно утверждён как 53 path: 2 create, 18 modify, 33 delete и один gitlink.
  - План сохраняет SQLite/WAL snapshot, typed digest, crash journal, source immutability, совместимость старых баз, последовательный first-launch/recovery/init/import gate и запрет замены при live managed pool; provider/dev exceptions сохранены.
  - Независимые final SPEC и QUALITY reviews дали `PASS`, Critical 0 / Important 0 / Minor 0.
  - Product-файлы не изменялись; `FORK-057` остался открытым и `NOT_STARTED`; product branch/worktree не создавались; push, PR, package, release и публикация не выполнялись.

- [x] FORK-061 Перепривязать и заново утвердить план FORK-035 #ci #release #planning !high @blocked_by:FORK-057 #milestone @blocked_by:FORK-143 @blocked_by:FORK-144
  **Оставшаяся работа по декомпозиции 2026-09-22:** Перепривязать существующий CI-план к текущей базе; Проверить актуальный CI-план независимо. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Ожидаемый результат: план macOS-only CI/release привязан к integration tip после FORK-057 и готов к реализации.

  **Критерии приёмки:**
  - Повторены workflow, script, artifact и caller census на точном base SHA.
  - План проверяет Universal `x86_64` + `arm64` и minimum macOS 14.2.
  - Secretless build/verification отделены от signing/notarization environments.
  - Публикация допускается только как последний draft-этап после всех проверок.
  - Независимые reviews дают Critical 0 / Important 0.
  - Workflow dispatch, подпись, notarization и публикация не запускаются.

  **Implemented 2026-10-02:** Актуальные SPEC/QUALITY — PASS, Critical 0 / Important 0 / Minor 0; точные хеши и границы проверки в [отчёте](../reviews/cross-cutting/plan-review-2026-10-02.md). Исправлены ownership и последовательность CI GREEN; общий gate следует после transport. Workflow dispatch/signing/publication не запускались.

- [x] FORK-062 Перепривязать и завершить review плана FORK-039 #theme #planning #review @blocked_by:FORK-057 #milestone @blocked_by:FORK-150 @blocked_by:FORK-151
  **Оставшаяся работа по декомпозиции 2026-09-22:** Обновить theme-план и классифицировать baseline; Получить независимое заключение по theme-плану. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Ожидаемый результат: semantic-theme plan rev3 обновлён под актуальную базу и получает первый финальный независимый PASS.

  **Критерии приёмки:**
  - Повторены ThemeScript, layout, BlockNote и test-path census.
  - Проверяются initial/reload prepaint, hydration diagnostics и focus contrast не ниже 3:1.
  - Baseline-сбой `theme-foundation.spec.ts` воспроизводится и классифицируется отдельно от изменений FORK-039.
  - SPEC и QUALITY reviews дают Critical 0 / Important 0.
  - Product/UI-код не изменяется.

  **Implemented 2026-10-02:** Актуальные SPEC/QUALITY — PASS, Critical 0 / Important 0 / Minor 0; точные хеши и границы проверки в [отчёте](../reviews/cross-cutting/plan-review-2026-10-02.md). Baseline классифицирован отдельно; canonical closeout исправлен. Это приёмка плана, реализация theme и native acceptance остаются открытыми.

- [x] FORK-063 Перепривязать и завершить review плана FORK-043 #recording #planning #review !high @blocked_by:FORK-057 #milestone @blocked_by:FORK-126 @blocked_by:FORK-127
  **Оставшаяся работа по декомпозиции 2026-09-22:** Перепривязать folder-план к текущему storage; Получить независимое заключение по folder-плану. Итоговая задача закрывается только после дочерних шагов и всех исходных зависимостей.

  Ожидаемый результат: план выбора папки записей обновлён под актуальные recording/storage seams и получает финальный независимый PASS.

  **Критерии приёмки:**
  - Повторены caller census, path census и allowlist на точном base SHA.
  - План сохраняет durable root registry, относительные пути и revisions, crash journal и no-clobber publication.
  - Проверены scoped import cancellation, descriptor/inode checks, recovery и совместимость старых SQLite-данных.
  - SPEC и QUALITY reviews дают Critical 0 / Important 0.
  - Product-код не изменяется.

  Ранний статус 2026-09-23 относился к отдельному FORK-051-only `.app`: TCC baseline не удалось прочитать, приложение ещё не запускалось, а sandboxed audio inventory был пуст. Последующая проверка объединила lifecycle/source-control/privacy изменения в один точный candidate и запустила его обычным Launch Services способом. Полная актуализация и прошлые снимки приведены в [отчёте FORK-119](../fork/fork119-artifact-selection-2026-09-23.md).

## Короткие исполняемые шаги

### I01 — Надёжная запись на текущем Mac

  **Implemented 2026-10-02:** SPEC/QUALITY — PASS, Critical 0 / Important 0 / Minor 0 на точных план/census hashes. Определены immutable derived versions с SQLite accepted/published pointers, owner/CAS и export-pending; unsafe FFmpeg pathname fallback запрещён. Census учитывает reviewed FEAT-100 overlay и 62 пути. Продукт storage, реальные FS/FFmpeg crash tests и native/release не выполнены. [Отчёт](../reviews/cross-cutting/plan-review-2026-10-02.md).

- [ ] FORK-119 Зафиксировать exact .app и план native-проверок #exec #i01 @parent:FORK-051 @iter:I01 @estimate:0.5-1h @kind:plan @plan_key:FORK-051.a @blocked_by:FORK-034 @status:blocked #batch1
  Результат и приёмка: Выбран один проверяемый артефакт, записаны source/package hashes и исходные разрешения; подготовлен тестовый профиль. Не выполнять TCC reset или новую сборку без причины.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.
      Состояние 2026-09-23 после восстановления сборки: Tao 0.35.2 восстановлен из registry archive и сверён с checksum индекса; двухсайтовый `app_state.rs` patch восстановлен из локальной истории и прошёл независимый статический review без подтверждённых дефектов для обычного launch path. `pnpm --dir frontend exec tauri build --bundles app` завершился с кодом 0. Новый `.app` имеет bundle content fingerprint `44fe95dabe37837a36b6a60535e4e904502711944c63f2654c28cd062fec4b1d`, executable SHA-256 `7089846f63ef5cffb61a6acaa0802a83e0ab4843d95395872591df863eca9276`, Info.plist SHA-256 `7ae38d497e673442ef02c754acccb1c5f5201c7801fcc1aa4052db6a5c116a8a`, `com.meetily.ai` / `0.4.1`, `x86_64`, ad-hoc; проверка подписи PASS, notarization нет. Источник: HEAD `f35d864370cbdafd088447c16f78b24e3f07c06a`, combined dirty diff SHA-256 `71f55701468f2438300c13bbc7e163eb18b28e275c983e9d471d2ffdd446bde5`. Workaround и его гипотеза об ошибке определения main thread ещё не подтверждены запуском. Read-only TCC query для app bundle завершилась `unable to open database file`; TCC не менялся. В этом прогоне app не запускался, отдельный test profile не активирован, native audio/device scenarios не выполнялись. Сверка требований FEAT-082/083–086 установила, что этот app содержит PostHog и не содержит source-control UI/command. Ветка FEAT-071 содержит privacy removal и source controls, но не включает полный FORK-051 commit series и расходится с ним по нескольким грязным исходникам. Поэтому сейчас нет единого exact app для объединённых native checks; `FORK-119` остаётся `blocked`, а его прежний blocker отсутствующего Tao source устранён. Интеграция веток не выполнялась. Детали и тест-план — в [отчёте FORK-119](../fork/fork119-artifact-selection-2026-09-23.md).
      Актуализация combined candidate 2026-09-23: прежнее описание выше относится к FORK-051-only артефакту и предыдущей проверке branch alignment. Текущий `.app` — `.worktrees/feat071-fork051-integration-local/target/release/bundle/macos/meetily.app`, source HEAD `61e890f444f81718d8136beae6afc6f61a5f0e44`, MERGE_HEAD `f35d864370cbdafd088447c16f78b24e3f07c06a`, dirty diff SHA-256 `0609ae4c707c6d4e6df899a1cecfcf9c1bbc0b98d5f656e22d51e474c782df07`. Bundle fingerprint `85ee255367ba0102d53de5a02fdc7c15e04bd8b5459689a4cf9ec111ce26fb0e`; executable `b98032e71e38413892dfaed07175be83a7f713667bfefddfd925d0a97df27eb8`; Info.plist `865cd2da29adb93b575b4e9e3d7eb969f53faaae2fcc0cc97f1520353fb3233f`; `com.meetily.ai` / `0.4.1`, x86_64, ad-hoc, notarization отсутствует. `codesign --verify --deep --strict` и `hdiutil verify` прошли. `open -a` завершился кодом 0; PID `84731` подтверждён по точному пути bundle executable. Повторная read-only проверка `CGWindowListCopyWindowInfo` обнаружила окно процесса на экране (`meetily`, 1100×700); это подтверждает появление окна, но его содержимое и контролы не инспектировались. Аудиозапись не проверялась. В read-only system inventory вне sandbox видны PA248, USB Condenser Microphone и встроенные динамики Mac mini. TCC read-only query завершилась `authorization denied`; TCC не менялся, test profile не подготовлен, исходный permission baseline неизвестен. Native сценарии не запускались, поэтому статус `FORK-119` остаётся `blocked`. Детали, пакетные hashes и сценарии — в [отчёте FORK-119](../fork/fork119-artifact-selection-2026-09-23.md).
      Повторная проверка в текущей управляемой сессии 2026-09-23 подтвердила source diff и SHA-256 executable/Info.plist/DMG; подпись прошла проверку. Перечисление процессов запрещено, Accessibility/System Events не дали доступ к окну, TCC DB не открылась, а read-only список аудиоустройств оказался пуст. Это ограничения текущей сессии, а не доказательство отсутствия устройства или дефекта приложения. Отдельный тестовый профиль не подготовлен; TCC и устройство не менялись, запись не запускалась. FORK-119 остаётся `blocked`, зависимые native-шаги не проверены. Подробности — в [отчёте FORK-119](../fork/fork119-artifact-selection-2026-09-23.md).
      По прямому запросу пользователя тот же exact candidate установлен в `/Applications/Meetily.app`: установленный bundle побайтно совпадает с исходным, executable/Info.plist hashes и подпись совпадают с отчётом. Обычный `open -a` запустил именно установленный executable; PID `7648` оставался жив через 43 секунды. Проверка Gatekeeper завершилась внутренней ошибкой подсистемы подписи, а native Computer Use не показывает приложения и не предоставляет `getApp`/`listApps`; содержимое окна и запись не проверены. Переустановка не закрывает FORK-119: отдельного профиля и исходного TCC baseline всё ещё нет, зависимые native-сценарии не выполнялись. Данные пользователя не удалялись; детали в [отчёте FORK-119](../fork/fork119-artifact-selection-2026-09-23.md).

- [ ] FORK-120 Проверить TCC deny→Settings→recovery #exec #i01 @parent:FORK-051 @iter:I01 @estimate:1-2h @kind:native @plan_key:FORK-051.b @blocked_by:FORK-119 #batch1
  Результат и приёмка: Первый/повторный запуск, отказ, возвращение из Settings и запись подтверждены обычным запуском .app; нет лишних prompts. Контролируемый сценарий согласуется до изменения permissions.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-121 Проверить физическую смену и возврат устройства #exec #i01 @parent:FORK-051 @iter:I01 @estimate:1-2h @kind:native @plan_key:FORK-051.c @blocked_by:FORK-120 #batch1
  Результат и приёмка: Отключение/переподключение/смена микрофона и ошибка восстанавливаются без потери сохранённого результата и второй сессии.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-122 Подтвердить stop/Quit/relaunch и закрыть lifecycle #exec #i01 @parent:FORK-051 @iter:I01 @estimate:0.5-1.5h @kind:native @plan_key:FORK-051.d @blocked_by:FORK-121 #batch1
  Результат и приёмка: На том же artifact stop и Quit финализируют, reopen видит результат; сведены TCC/device evidence. System Track/mute/gain остаются отдельными FEAT-071 шагами и не создают обратный blocker на FEAT-071.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I02 — Пользовательская миграция и план хранения

- [ ] FORK-123 Подключить пользовательский WebView export/import к готовым командам #exec #i02 @parent:FORK-037 @iter:I02 @estimate:1-3h @kind:code @plan_key:FORK-037.a
  Результат и приёмка: Сначала сверены 1ae518a и уже зарегистрированные prepare/publish/status; оставшийся пользовательский путь связывает WebView adapter, operation identity и Rust journal. Готовые команды/lock/WAL/startup guard не реализуются повторно.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Source audit 2026-10-02 (реализация не завершена):**
  - `1ae518a1c90e3175539ac03ab286b5519d7e28fe` входит в текущий main; prepare/publish/status уже зарегистрированы и повторно не реализуются.
  - При незавершённом journal lib.rs прекращает setup, поэтому обычный UI не может продолжить миграцию. File-phase completed не подтверждает WebView import; нужен отдельно проверенный recovery bootstrap и связанный completion readback.
  - [Точный source-аудит и маршрут для ревью](../plans/webview-migration/2026-10-02-webview-migration-integration.md) сохраняют lock/WAL/startup guard, operation identity и marker. Восемь исторических source hashes относились к base main до FEAT-100. 4 октября выполнен [11-path recensus](../fork/evidence/webview-migration-rebind-corrected-2026-10-04.json); отличается lib.rs с сохранённой QA-регистрацией. [Recovery-contract](../plans/webview-migration/2026-10-04-webview-recovery-contract.md) прошёл bounded design review, но не допущен к реализации: требуется зафиксировать schemas/capabilities/resume/identity pair. [Реальный browser adapter probe](../fork/evidence/webview-migration-browser-marker-2026-10-04.json) подтверждает недостаточность marker для preference readback на явно введённом расхождении; native/user migration flow не проверен. Продукт миграции и пользовательские данные не менялись, FORK-123 остаётся открытой. [V3 contract](../plans/webview-migration/2026-10-04-webview-recovery-contract-v3.md) закрывает найденные restart-loop и недоступный SQLite path на уровне дизайна; bounded SPEC/QUALITY C0/I0/M0. [Точные границы](../fork/evidence/webview-recovery-contract-v3-progress-2026-10-04.json): fresh-only API и atomic admission/receipt конкретизированы в [bounded amendment v2](../plans/webview-migration/2026-10-04-webview-fresh-persistence-amendment-v2.md), исправление authority seam прошло SPEC/QUALITY C0/I0/M0. Full schemas/finite ownership, plugin/CSP, clean source и actual pair ещё не допущены; source review не подтверждает runtime fresh-resume.

- [ ] FORK-124 Проверить полный перенос контрольного профиля #exec #i02 @parent:FORK-037 @iter:I02 @estimate:1-2h @kind:verification @plan_key:FORK-037.b @blocked_by:FORK-123
  Результат и приёмка: В отдельном профиле перенесены встречи, модели, шаблоны/selection, говорящие, prefs, IndexedDB/localStorage и прерванная обработка; источник неизменён, readback совпал.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-125 Проверить crash/retry и отказ переноса #exec #i02 @parent:FORK-037 @iter:I02 @estimate:1-2h @kind:verification @plan_key:FORK-037.c @blocked_by:FORK-124
  Результат и приёмка: Сквозные filled-disk, несовместимая схема и прерывание не портят источник; повтор безопасен, незавершённый target не открывается.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [x] FORK-126 Перепривязать folder-план к текущему storage #exec #i02 @parent:FORK-063 @iter:I02 @estimate:1-2h @kind:plan @plan_key:FORK-063.a @blocked_by:FORK-057
  Результат и приёмка: Сверены callers/paths/allowlist; registry/revisions/journal/no-clobber/cancel/inode/recovery и совместимость SQLite сохранены.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Implemented:**
  - План перепривязан к main `53f4415`: сверены source hashes, 86 caller paths и 16 существующих миграций; reviewed оригинал сохранён.
  - Registry идентифицирует полный path/dev/inode tuple; сохранены revisions/journal/no-clobber/cancel/recovery и legacy no-move. Будущая миграция перенесена после актуального search prefix.
  - Первый census включал 60 путей вместо прежних 53; уточнённый контракт добавляет два FEAT-100 consumer/test пути, всего 62. Определены immutable derived versions, SQLite heads и export-pending; FFmpeg pathname fallback запрещён. Продукт storage не менялся. [План](../plans/recording-storage/2026-10-02-recording-storage.md).

- [x] FORK-127 Получить независимое заключение по folder-плану #exec #i02 @parent:FORK-063 @iter:I02 @estimate:0.5-1h @kind:review @plan_key:FORK-063.b @blocked_by:FORK-126
  Результат и приёмка: SPEC/QUALITY PASS на точных байтах; устаревший STOP на выполненную 034 не заставляет повторять её.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

### I03 — Папка записей и восстановление захвата

  **Implemented 2026-10-02:** SPEC/QUALITY — PASS, Critical 0 / Important 0 / Minor 0 на точных план/census hashes. Определены immutable derived versions с SQLite accepted/published pointers, owner/CAS и export-pending; unsafe FFmpeg pathname fallback запрещён. Census учитывает reviewed FEAT-100 overlay и 62 пути. Продукт storage, реальные FS/FFmpeg crash tests и native/release не выполнены. [Отчёт](../reviews/cross-cutting/plan-review-2026-10-02.md).

- [ ] FORK-128 Добавить выбор корня и относительные пути #exec #i03 @parent:FORK-043 @iter:I03 @estimate:1-3h @kind:code @plan_key:FORK-043.a @blocked_by:FORK-031 @blocked_by:FORK-055 @blocked_by:FORK-063
  Результат и приёмка: Registry и revisions согласованы с reviewed планом; старые пути читаются без потери данных.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-129 Проверить start/restart/import и атомарное сохранение #exec #i03 @parent:FORK-043 @iter:I03 @estimate:1-2h @kind:verification @plan_key:FORK-043.b @blocked_by:FORK-128
  Результат и приёмка: Новая запись, повтор и импорт используют выбранный корень; rename атомарный, дублей нет.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-130 Проверить конкуренцию, отмену и recovery #exec #i03 @parent:FORK-043 @iter:I03 @estimate:1-3h @kind:verification @plan_key:FORK-043.c @blocked_by:FORK-129
  Результат и приёмка: Конкурентное сохранение/отмена/crash сохраняют raw и результаты; descriptor/inode защита проверена.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-131 Проверить перенос каталога, нехватку места и старую схему #exec #i03 @parent:FORK-043 @iter:I03 @estimate:1-2h @kind:verification @plan_key:FORK-043.d @blocked_by:FORK-130
  Результат и приёмка: Старые встречи, templates, speakers и prefs сохранны; неизвестная схема отклонена явно, no-clobber соблюдён.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-132 Проверить crash/relaunch capture и transcribe #exec #i03 @parent:FORK-052 @iter:I03 @estimate:1-3h @kind:native @plan_key:FORK-052.a @blocked_by:FORK-051 @blocked_by:FORK-043 @blocked_by:FORK-057
  Результат и приёмка: Отдельный тестовый профиль и baseline зафиксированы; отмена/повтор не теряют raw и не создают дубли.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I04 — Модели, Intel benchmark и privacy

- [x] FORK-133 Сверить Gemma-путь и повторно использовать готовый downloader #exec #i04 @parent:FORK-040 @iter:I04 @estimate:1-2h @kind:verification @plan_key:FORK-040.a @blocked_by:FORK-031 @blocked_by:FORK-033 @blocked_by:FORK-057 @blocked_by:FEAT-070
  Результат и приёмка: Применимость источника и оставшиеся разрывы установлены; download/hash/atomic-install/выбор проходят через FEAT-070 без повторного написания загрузчика.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Implemented:**
  - Локальные Gemma selection/helper и macOS recommendation источники уже входят в main; дополнительный перенос не нужен.
  - Каталог, выбор, сохранение настройки и генерация используют существующий путь FEAT-070; повторный загрузчик не добавлен.
  - Проверка исходников и изолированных fixtures завершена; [evidence и оставшиеся разрывы](../fork/gemma-path-2026-10-02.md) зафиксированы. Реальные веса, native generation, reopen/offline и Apple Silicon остаются приёмкой FORK-134/167 и FORK-040.

- [ ] FORK-134 Проверить локальную генерацию и настройки на Intel #exec #i04 @parent:FORK-040 @iter:I04 @estimate:1-2h @kind:native @plan_key:FORK-040.b @blocked_by:FORK-133
  Результат и приёмка: Шаблон и RAM-рекомендации соответствуют модели; локальная генерация и offline работают на доступном Intel; архитектурные ограничения явно записаны.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-135 Зафиксировать корпус и эталоны benchmark #exec #i04 @parent:FORK-049 @iter:I04 @estimate:1-2h @kind:research @plan_key:FORK-049.a
  Результат и приёмка: Есть разрешённые реальные русские записи: тихая/шумная речь, длинные/короткие фразы, перекрытия и отдельные дорожки; исходники неизменны.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-136 Подготовить измерения и выполнить Intel benchmark #exec #i04 @parent:FORK-049 @iter:I04 @estimate:2-3h @kind:native @plan_key:FORK-049.b @blocked_by:FORK-135 @blocked_by:FORK-134
  Результат и приёмка: Повторяемо измерены текст/говорящие, задержка, CPU/RAM/размер и offline на исходном корпусе. Сравниваются существующие кандидаты, а не ещё не созданная diarization.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I05 — Говорящие и post-call управление

- [ ] FORK-137 Сравнить diarization-кандидатов и выбрать подход #exec #i05 @parent:FORK-041 @iter:I05 @estimate:1-2h @kind:research @plan_key:FORK-041.a @blocked_by:FEAT-071 @blocked_by:FORK-136 @blocked_by:FORK-031 @blocked_by:FORK-034 @blocked_by:FORK-057
  Результат и приёмка: Существующий sherpa и donor сравниваются на Intel-корпусе; решение учитывает перекрытия, разделение источника и личности.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-138 Проверить runtime и согласованность live/post-call #exec #i05 @parent:FORK-041 @iter:I05 @estimate:2-3h @kind:code @plan_key:FORK-041.b @blocked_by:FORK-137
  Результат и приёмка: Выбранный путь даёт согласованные назначения; перекрытие реплик покрыто целевым сценарием. Если изменение шире одного участка — разделить перед кодированием.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-139 Проверить имена и повторное открытие встречи #exec #i05 @parent:FORK-041 @iter:I05 @estimate:1-2h @kind:verification @plan_key:FORK-041.c @blocked_by:FORK-138
  Результат и приёмка: Ручные имена, неизвестные связи и назначения сохраняются после reopen; готовые primitives переиспользованы.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-140 Добавить видимую настройку post-call diarization #exec #i05 @parent:FORK-042 @iter:I05 @estimate:1-2h @kind:code @plan_key:FORK-042.a @blocked_by:FORK-139
  Результат и приёмка: Включение/выключение сохраняется; UI подключён к проверенному runtime. Можно реализовать после локальной 041.c; итоговая приёмка ждёт весь FORK-041.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-141 Добавить ручную связь говорящего с дорожкой #exec #i05 @parent:FORK-042 @iter:I05 @estimate:1-2h @kind:code @plan_key:FORK-042.b @blocked_by:FORK-140
  Результат и приёмка: Источник не приравнен человеку; You/Guest/Speaker N не становятся постоянными людьми автоматически.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-142 Проверить recovery diarize и summarize #exec #i05 @parent:FORK-052 @iter:I05 @estimate:1-2h @kind:native @plan_key:FORK-052.b @blocked_by:FORK-132 @blocked_by:FORK-141
  Результат и приёмка: Прерывание/повтор сохраняют уже завершённые результаты и ручные связи; Rust job state остаётся источником истины.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I06 — macOS-only CI и проверка поверхности

- [x] FORK-143 Перепривязать существующий CI-план к текущей базе #exec #i06 @parent:FORK-061 @iter:I06 @estimate:1-2h @kind:plan @plan_key:FORK-061.a @blocked_by:FORK-057
  Результат и приёмка: Сверены workflows/scripts/artifacts/callers; сняты только устаревшие STOP на уже закрытые 034/057; сохранены Universal, 14.2, secretless/signing/draft границы.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.
  **Implemented:**
  - Существующий reviewed план перепривязан к exact main `53f4415058fb01e38b8892a47ac036bfffa394ba` и его tree/parents; исходный план SHA-256 `a1d57ae7…` сохранён.
  - Повторены workflow/job/caller/artifact census и проверка build/config adapters; десять YAML зафиксированы с hashes. Уже имеющийся Tauri minimum 14.2 сохраняется.
  - Сняты исторические STOP на завершённые FORK-034/057; Universal, recursive Mach-O, secretless/signing/notarization, immutable transport и финальный draft-контракт сохранены. FORK-144 и внешние prerequisites остаются открытыми. [План](../plans/macos-ci/2026-10-02-macos-ci-release.md).


- [x] FORK-144 Проверить актуальный CI-план независимо #exec #i06 @parent:FORK-061 @iter:I06 @estimate:0.5-1h @kind:review @plan_key:FORK-061.b @blocked_by:FORK-143
  Результат и приёмка: SPEC/QUALITY не имеют Critical/Important на точных байтах плана; dispatch/signing/publication не запускаются.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Implemented 2026-10-02:** Актуальные SPEC/QUALITY — PASS, Critical 0 / Important 0 / Minor 0; точные хеши и границы проверки в [отчёте](../reviews/cross-cutting/plan-review-2026-10-02.md). Исправлены ownership и последовательность CI GREEN; общий gate следует после transport. Workflow dispatch/signing/publication не запускались.

- [ ] FORK-145 Внести macOS-only jobs по утверждённому плану #exec #i06 @parent:FORK-035 @iter:I06 @estimate:1-3h @kind:code @plan_key:FORK-035.a @blocked_by:FORK-061 @blocked_by:FORK-027 @blocked_by:FORK-031 @blocked_by:FORK-032 @blocked_by:FORK-033 @blocked_by:FORK-057
  **В работе, 2026-10-03:** поправка prerequisite-плана принята SPEC/QUALITY, оба actionlint-архива сверены с закреплёнными хешами; локальный syntax baseline трёх текущих workflows прошёл. Clean route, immutable producer/fresh acquisition, review action-кода и реальные dual-host проверки открыты; продуктовые workflows не изменялись.
  Результат и приёмка: Удалены чужие платформенные jobs; локально проверены синтаксис и связь build/verify/release, без удалённого запуска.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-146 Проверить secretless Universal build и package #exec #i06 @parent:FORK-035 @iter:I06 @estimate:1-3h @kind:verification @plan_key:FORK-035.b @blocked_by:FORK-145
  Результат и приёмка: Проверены x86_64/arm64, minimum 14.2, ресурсы и только .app/.dmg; это сборочное доказательство, не runtime на обеих архитектурах.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-147 Проверить отделение подписи и финального draft job #exec #i06 @parent:FORK-035 @iter:I06 @estimate:0.5-1.5h @kind:review @plan_key:FORK-035.c @blocked_by:FORK-146
  Результат и приёмка: Непроверенный пакет не обходит проверки; signing/notarization отделены, draft последний. Реальные credentials и dispatch требуют отдельного разрешения.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [x] FORK-148 Добавить ограниченный scanner платформенной поверхности #exec #i06 @parent:FORK-038 @iter:I06 @estimate:1-2h @kind:code @plan_key:FORK-038.a @blocked_by:FORK-032 @blocked_by:FORK-033 @blocked_by:FORK-034 @blocked_by:FORK-057
  Результат и приёмка: Охвачены targets/scripts/paths/docs/conditions и Python/FastAPI product runtime; разрешённые dev tools/providers явно отделены. Контрольное нарушение вызывает FAIL.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Implemented:**
  - Добавлен отдельный `check:platform` с PASS/FAIL/ERROR и JSON inventory; он не меняет release gate.
  - 30 адресных тестов, весь Node-набор 454/454, lint и typecheck прошли на checkout текущей базы с точными зависимостями.
  - Реальный scanner run дал exit 1 и 48 замечаний, включая девять Rust-файлов с чужими cfg. Состав и ограничения зафиксированы в [отчёте](../fork/platform-surface-2026-10-02.md); FORK-149 остаётся открытой.

- [ ] FORK-149 Проверить итоговый source и package #exec #i06 @parent:FORK-038 @iter:I06 @estimate:0.5-1h @kind:verification @plan_key:FORK-038.b @blocked_by:FORK-148 @blocked_by:FORK-035
  Результат и приёмка: После CI-очистки scanner проходит на согласованном снимке; прежние пути и исключения объяснены.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

### I07 — Тема и структура интерфейса

- [x] FORK-150 Обновить theme-план и классифицировать baseline #exec #i07 @parent:FORK-062 @iter:I07 @estimate:1-2h @kind:plan @plan_key:FORK-062.a @blocked_by:FORK-057
  Результат и приёмка: Сверены ThemeScript/layout/BlockNote; отдельно воспроизведён или честно отмечен непроверенным прежний baseline; критерии prepaint/hydration/focus актуальны.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Implemented:**
  - Сверены единственный ThemeScript/layout, runtime ConfigContext, оба BlockNote consumers и 18 путей; план и hashes привязаны к main `53f4415`.
  - Sandbox EPERM воспроизведён; разрешённый browser run выполнил baseline: 30 passed/3 failed. Static audit и 2 native-config Node checks прошли; actual native runtime не проверялся.
  - Диагностический probe подтвердил перезапись раннего dark-toggle наблюдения в старом тесте; визуально классифицирован sidebar content drift. Snapshots не изменены, prepaint/hydration/focus/реальный контраст сохраняются обязательными. [План и границы](../plans/theme/2026-10-02-semantic-theme.md); FORK-151 открыт.

- [x] FORK-151 Получить независимое заключение по theme-плану #exec #i07 @parent:FORK-062 @iter:I07 @estimate:0.5-1h @kind:review @plan_key:FORK-062.b @blocked_by:FORK-150
  Результат и приёмка: SPEC/QUALITY PASS без Critical/Important на точных байтах плана; продуктовый код не меняется.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

  **Implemented 2026-10-02:** Актуальные SPEC/QUALITY — PASS, Critical 0 / Important 0 / Minor 0; точные хеши и границы проверки в [отчёте](../reviews/cross-cutting/plan-review-2026-10-02.md). Baseline классифицирован отдельно; canonical closeout исправлен. Это приёмка плана, реализация theme и native acceptance остаются открытыми.

- [ ] FORK-152 Закрепить semantic theme по утверждённому плану #exec #i07 @parent:FORK-039 @iter:I07 @estimate:1-3h @kind:code @plan_key:FORK-039.a @blocked_by:FORK-031 @blocked_by:FORK-062
  Результат и приёмка: System/light/dark используют согласованные tokens; scope ограничен foundation темы.
  **В работе, 2026-10-03:** source census принят SPEC/QUALITY, продуктовые правки не начаты. До Task 1 требуется согласованный маршрут dev/feature и чистый checkout на точной базе; текущий dev отстаёт на 82 коммита. Диагностика порядка CSS/script сохранена отдельно, но не заменяет prepaint/hydration/contrast assertion suites.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-153 Проверить prepaint, hydration и контраст #exec #i07 @parent:FORK-039 @iter:I07 @estimate:1-2h @kind:verification @plan_key:FORK-039.b @blocked_by:FORK-152
  Результат и приёмка: Initial/reload не мигают неправильной темой; BlockNote включён; focus contrast ≥3:1; accessibility evidence указано отдельно от browser.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-154 Описать структуру окна и пользовательские состояния #exec #i07 @parent:FORK-046 @iter:I07 @estimate:1-2h @kind:design @plan_key:FORK-046.a @blocked_by:FORK-039
  Результат и приёмка: Окно/sidebar/transcript/summary inspector/панели описаны без зависимости от названия; зафиксирован ограниченный вопрос для Tauri/AppKit/SwiftUI прототипа, переписывание UI не обещано.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

### I08 — Финальная identity, миграция и updater

- [ ] FORK-155 Зафиксировать выбранные имя, bundle ID и repository slug #exec #i08 @parent:FORK-028 @iter:I08 @estimate:0.5-1h @kind:user @plan_key:FORK-028.a @blocked_by:FORK-026
  Результат и приёмка: Решение владельца записано одним набором значений; ранее отложенный выбор не решается агентом самостоятельно.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-156 Определить момент выхода из fork network #exec #i08 @parent:FORK-028 @iter:I08 @estimate:0.5-1h @kind:user @plan_key:FORK-028.b @blocked_by:FORK-155
  Результат и приёмка: Записаны условия handoff и ограничения identity; доступность имени/репозитория подтверждается только отдельной фактической проверкой.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-157 Подключить update channel и проверку пакета #exec #i08 @parent:FORK-036 @iter:I08 @estimate:1-3h @kind:code @plan_key:FORK-036.a @blocked_by:FORK-028 @blocked_by:FORK-056
  Результат и приёмка: Использована выбранная identity; подпись и архитектура проверяются до установки; до готовности канал остаётся выключенным.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-158 Добавить управление обновлением без конфликта с записью #exec #i08 @parent:FORK-036 @iter:I08 @estimate:1-2h @kind:code @plan_key:FORK-036.b @blocked_by:FORK-157
  Результат и приёмка: Отложить/пропустить, прогресс и ошибка работают; update при записи запрещён.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-159 Проверить rollback и недоверенный пакет #exec #i08 @parent:FORK-036 @iter:I08 @estimate:1-2h @kind:verification @plan_key:FORK-036.c @blocked_by:FORK-158
  Результат и приёмка: Повреждение, неверная подпись/архитектура отвергаются; rollback имеет проверенное ограничение совместимости.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-160 Подтвердить upgrade между старым и новым bundle ID #exec #i08 @parent:FORK-037 @iter:I08 @estimate:1-3h @kind:native @plan_key:FORK-037.d @blocked_by:FORK-125 @blocked_by:FORK-028
  Результат и приёмка: После финального выбора identity выполнен реальный old→new upgrade в тестовом профиле с readback; фактическая WebView изоляция проверена.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-161 Уточнить команды и accessibility, применить identity #exec #i08 @parent:FORK-046 @iter:I08 @estimate:1-2h @kind:design @plan_key:FORK-046.b @blocked_by:FORK-154 @blocked_by:FORK-028
  Результат и приёмка: Toolbar/menu/keyboard/Settings и VoiceOver/contrast/themes/reduced-motion имеют проверяемые критерии; финальная спецификация использует выбранную identity.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

### I09 — Системные команды, окна и основные экраны

- [ ] FORK-162 Подключить системное меню и keyboard shortcuts #exec #i09 @parent:FORK-047 @iter:I09 @estimate:1-2h @kind:code @plan_key:FORK-047.a @blocked_by:FORK-046 @blocked_by:FEAT-071
  Результат и приёмка: У каждой toolbar-команды есть menu/shortcut; команды используют существующую сессию.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-163 Реализовать окна и компактную панель #exec #i09 @parent:FORK-047 @iter:I09 @estimate:1-3h @kind:code @plan_key:FORK-047.b @blocked_by:FORK-162
  Результат и приёмка: Окна/панель следуют спецификации; timer/state общий, повторная команда не создаёт recorder.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-164 Проверить команды и окна в macOS #exec #i09 @parent:FORK-047 @iter:I09 @estimate:0.5-1.5h @kind:native @plan_key:FORK-047.c @blocked_by:FORK-163
  Результат и приёмка: Native shortcuts, start/pause/resume/stop, ошибки и переходы между окнами проходят на одном артефакте.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-165 Собрать recording/processing и список встреч #exec #i09 @parent:FORK-048 @iter:I09 @estimate:1-3h @kind:code @plan_key:FORK-048.a @blocked_by:FORK-141 @blocked_by:FORK-043 @blocked_by:FORK-047
  Результат и приёмка: Использованы готовые state/lifecycle; ручное название и начало встречи сохраняются. Начинать после локального speaker UI и folder/окон, финальный parent gate сохраняется.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-166 Собрать transcript/speakers/summary и существующий экспорт #exec #i09 @parent:FORK-048 @iter:I09 @estimate:1-3h @kind:code @plan_key:FORK-048.b @blocked_by:FORK-165
  Результат и приёмка: Результаты и фоновые состояния переживают переходы; новый PDF/DOCX из FEAT-075 не является незаявленным Alpha blocker.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

### I10 — Финальные аппаратные и сквозные проверки функций

- [ ] FORK-167 Подтвердить Gemma runtime на Apple Silicon #exec #i10 @parent:FORK-040 @iter:I10 @estimate:1-2h @kind:native @plan_key:FORK-040.c @blocked_by:FORK-134
  Результат и приёмка: На физическом arm64 проверены локальная генерация и допустимые ускорение/fallback; сборка arm64 не считается этим результатом.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-168 Повторить benchmark на arm64 и свести сравнение #exec #i10 @parent:FORK-049 @iter:I10 @estimate:1-3h @kind:native @plan_key:FORK-049.c @blocked_by:FORK-136 @blocked_by:FORK-040
  Результат и приёмка: Та же версия корпуса и параметры; различия Intel/arm64 и ограничения явно записаны. Запланировано к финальному аппаратному этапу.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-169 Сформировать выводы и roadmap моделей #exec #i10 @parent:FORK-050 @iter:I10 @estimate:1-2h @kind:plan @plan_key:FORK-050.a @blocked_by:FORK-049
  Результат и приёмка: Из benchmark следуют порядок Whisper/Parakeet и проверяемый эксперимент русского провайдера; эксперимент не выполняется в этой задаче.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-170 Проверить переименование и повторную обработку #exec #i10 @parent:FORK-042 @iter:I10 @estimate:0.5-1h @kind:verification @plan_key:FORK-042.c @blocked_by:FORK-141 @blocked_by:FORK-041
  Результат и приёмка: Имя локально одной встрече; reprocess не затирает подтверждённые назначения; окончательный sign-off после FORK-041.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-171 Проверить сквозной UX, recovery и перетаскивание #exec #i10 @parent:FORK-048 @iter:I10 @estimate:1-2h @kind:native @plan_key:FORK-048.c @blocked_by:FORK-166 @blocked_by:FORK-042
  Результат и приёмка: Native recording→processing→transcript→summary→export/recovery проходит; финальная визуальная проверка и сохранность ручных данных подтверждены.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-172 Проверить offline, filled disk, folder switch и upgrade вместе #exec #i10 @parent:FORK-052 @iter:I10 @estimate:2-3h @kind:native @plan_key:FORK-052.c @blocked_by:FORK-142 @blocked_by:FORK-037 @blocked_by:FORK-042
  Результат и приёмка: Переиспользованы данные отдельных 037/043 проверок, подтверждено сквозное взаимодействие; rollback ограничен совместимостью схемы. Если матрица не помещается в один слот, разделить по сценарию до запуска.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I11 — Подписанный update и финальный кандидат

- [ ] FORK-173 Подтвердить настоящий подписанный update #exec #i11 @parent:FORK-036 @iter:I11 @estimate:1-2h @kind:external @plan_key:FORK-036.d @blocked_by:FORK-159
  Результат и приёмка: Разрешённый update проходит на выбранной identity/канале; ключ updater и Apple Developer ID учитываются отдельно. Без ключей — blocked, не PASS.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [x] FORK-174 Собрать точную матрицу оставшихся Alpha-проверок #exec #i11 @parent:FORK-053 @iter:I11 @estimate:0.5-1h @kind:plan @plan_key:FORK-053.a
  Результат и приёмка: Каждое требование сопоставлено задаче/артефакту/устройству; все старые blocker edges сохранены. Нет требования заново писать или собирать уже проверенное без изменения входов.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.
  **Implemented:**
  - Все 13 исходных блокеров Alpha и этапы FORK-175–180 сопоставлены нужным артефактам и условиям проверки; dependency edges сохранены.
  - Все 25 строк release smoke привязаны к задачам, Intel/arm64 и конкретному readback; старые diagnostic packages не засчитываются новому main.
  - Отделены уже полученные интеграционные результаты от exact-commit полного gate, native smoke и distribution trust. [Матрица](../fork/alpha-acceptance-matrix-2026-10-02.md).


- [ ] FORK-175 Подготовить чистый проверенный source commit #exec #i11 @parent:FORK-053 @iter:I11 @estimate:1-3h @kind:git @plan_key:FORK-053.b @blocked_by:FORK-174 @blocked_by:FORK-035 @blocked_by:FORK-036 @blocked_by:FORK-038 @blocked_by:FORK-039 @blocked_by:FORK-040 @blocked_by:FORK-042 @blocked_by:FORK-043 @blocked_by:FORK-044 @blocked_by:FORK-047 @blocked_by:FORK-048 @blocked_by:FORK-050 @blocked_by:FORK-051 @blocked_by:FORK-052
  Результат и приёмка: Все исходные blockers FORK-053 закрыты; после отдельной авторизации создан clean reviewed commit и выполнен check.sh --full. Это не разрешение commit в текущей задаче.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-176 Собрать и проверить финальный Universal package #exec #i11 @parent:FORK-053 @iter:I11 @estimate:1-3h @kind:external @plan_key:FORK-053.c @blocked_by:FORK-175
  Результат и приёмка: Из clean exact commit release-candidate формирует app/dmg; проверены slices/resources/14.2 и provenance. При отсутствии Developer ID возможен только диагностический результат.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-177 Подтвердить подпись и доверие финального пакета #exec #i11 @parent:FORK-053 @iter:I11 @estimate:1-2h @kind:external @plan_key:FORK-053.d @blocked_by:FORK-176
  Результат и приёмка: С отдельным разрешением Developer ID/notarization/Gatekeeper подтверждены на candidate; ad-hoc не PASS. Финальные app/dmg hashes зафиксированы, последующий smoke использует эти байты.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I12 — Alpha на двух архитектурах и minimum macOS

- [ ] FORK-178 Пройти итоговый native smoke на Intel #exec #i12 @parent:FORK-053 @iter:I12 @estimate:1-3h @kind:native @plan_key:FORK-053.e @blocked_by:FORK-177
  Результат и приёмка: Exact signed artifact: capture/permissions/sources, offline/recovery/upgrade и пользовательские model-сценарии заполнены в smoke report. Затронутые данные только в тестовом профиле.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-179 Пройти итоговый native smoke на Apple Silicon #exec #i12 @parent:FORK-053 @iter:I12 @estimate:1-3h @kind:native @plan_key:FORK-053.f @blocked_by:FORK-177
  Результат и приёмка: Тот же Universal artifact; проверены capture/permissions/sources, модели/ускорение, offline/recovery/upgrade. Intel evidence не переносится на arm64.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

- [ ] FORK-180 Подтвердить минимальную macOS и вынести Alpha verdict #exec #i12 @parent:FORK-053 @iter:I12 @estimate:0.5-2h @kind:native @plan_key:FORK-053.g @blocked_by:FORK-178 @blocked_by:FORK-179
  Результат и приёмка: Есть runtime на macOS 14.2 (может быть тем же хостом из e/f); обе архитектуры, trust и smoke report полны; release-gate.sh --smoke-report возвращает 0. При недоступной среде — NOT VERIFIED.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I13 — Отдельно разрешённый handoff

- [ ] FORK-181 Подготовить read-only handoff с перечнем веток и PR #exec #i13 @parent:FORK-054 @iter:I13 @estimate:0.5-1.5h @kind:plan @plan_key:FORK-054.a @blocked_by:FORK-053
  Результат и приёмка: Каждый перенос подтверждён; сформированы точные действия и тексты комментариев. Ничего не архивировано и не опубликовано.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-182 Выполнить отдельно разрешённый handoff и readback #exec #i13 @parent:FORK-054 @iter:I13 @estimate:1-2h @kind:external @plan_key:FORK-054.b @blocked_by:FORK-181
  Результат и приёмка: Только после разрешения выполнены выбранные Git/GitHub действия и закрыты подтверждённые PR; результат перечитан.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.

### I19 — Ограниченные исследования streaming/provider

- [ ] FORK-183 Пересмотреть streaming-контракт и риски #exec #i19 @parent:FORK-045 @iter:I19 @estimate:1-2h @kind:research @plan_key:FORK-045.a @blocked_by:FORK-038 @blocked_by:FORK-044 @blocked_by:FORK-051 @blocked_by:FORK-057 @blocked_by:FEAT-064
  Результат и приёмка: Ограничены queues, sequence, session/job identity, ordered Channel; global events не transport аудио; записаны память/Intel/lifecycle риски.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-184 Проверить фразы, паузы и ресемплинг #exec #i19 @parent:FORK-045 @iter:I19 @estimate:1-2h @kind:verification @plan_key:FORK-045.b @blocked_by:FORK-183
  Результат и приёмка: На доступном кандидате отсутствуют обрезанные концы/дубли для коротких и длинных фраз; FORK-044 не переоткрывается.
  Граница: один указанный результат и целевая проверка; если нужен второй самостоятельный результат, разделить шаг до реализации. Полный интеграционный прогон не повторять без нового риска.

- [ ] FORK-185 Сравнить Intel/arm64 и принять решение #exec #i19 @parent:FORK-045 @iter:I19 @estimate:1-3h @kind:native @plan_key:FORK-045.c @blocked_by:FORK-184
  Результат и приёмка: Точность, задержка и нагрузка измерены; решение внедрять/отложить подкреплено фактами. Новая реализация и расширенные настройки не обещаны заранее.
  Граница: необходимое устройство, решение или разрешение проверяются до начала; отсутствие входа фиксируется как блокер, а не успешная проверка. Этот шаг не разрешает посторонние изменения.


## Ближайшая пачка — адресные проверки

- [x] FORK-189 Добавить адресные режимы docs/audio/summary в существующий check #exec #batch1 #validation !high @iter:B1 @estimate:2-4h @kind:code @review:R2
      Статус: выполнено и независимо проверено на `themaddoxed/feat071-audio-controls`, база `61e890f`. Маршруты `--docs`, `--audio`, `--summary` запускают только выбранные проверки; пустой выбор и неизвестные режимы отклоняются, состав `--quick/--full` сохранён. Проверки: docs 1 passed, audio 241 passed/5 ignored, summary 101 passed, маршрутизация 16 passed; независимое review: PASS. Полные `--quick/--full` не запускались.
  Поверхность: scripts/check.sh, frontend/package.json, тест маршрутизации и существующее руководство тестирования. Сначала подтвердить текущий состав --quick на продуктовой базе.
  Результат: docs не запускает browser/Rust/build; audio и summary запускают реальные выбранные модули/фильтры; неизвестный scope отклоняется; ноль запущенных тестов не считается PASS. --full сохраняет прежний обязательный состав.
  Приёмка: тест состава команд и фактический непустой запуск каждого затронутого маршрута, адресный diff review. Не добавлять CI-платформу, автоматический анализ влияния, DMG/signing или общий lint backlog. После двух неуспешных циклов по одной причине сохранить fail и blocker.
