# План дальнейших доработок Meetily — обновлён 2 октября 2026

Текущий порядок продолжения: [план от 18 октября](../development/execution-order.md).
Планы ниже сохраняют историю своих дат; навигация — в [INDEX.md](../../INDEX.md).

## Текущая интеграция и порядок продолжения — 2 октября

Локальное вливание завершено: `main@53f4415` содержит merge-кандидат
`integration/main-features-2026-10-02`, подготовленный от `main@952e3614`. В него перенесены macOS product line `61e890f4`, сохранённый
combined audio/privacy snapshot, summary FEAT-186–188, поиск FEAT-094–096 и
актуальные канонические PRD. Исходные dirty checkout сохранены. Владелец
2 октября разрешил локальный merge commit и обновление main с сохранением
его прежнего overlay в stash и salvage checkout. Push и публикация не входят
в это разрешение. Точный commit и результат обновления main записываются
в `.worktrees/integration-backups-2026-10-02/integration-report.json`.

TypeScript, lint, 424 Node и 61 Vitest тест, production frontend build и 29
критических браузерных сценариев прошли. Поиск проверен по названию, видимому
тексту итогов и реплике на второй странице расшифровки с реальной прокруткой.
Rust Clippy с `-D warnings` прошёл; workspace tests: 603 passed, 10 ignored.
Отдельный packaged FFmpeg recovery: 1 passed. Проверены все 16 префиксов SQLx
(26/26 data integrity), summary ownership (16/16) и поиск (5/5), включая
откат при ошибке индекса. Все автоматические этапы прошли после адресных
исправлений и повторных проверок; исходный полный запуск завершился с двумя
ошибками, устранёнными в Rust recheck. Эти результаты не подтверждают запись
звука или upgrade установленного приложения.

| Порядок | Канонические карточки | Результат и условие завершения |
|---|---|---|
| 1 | Текущая локальная интеграция | Завершено: merge commit `53f4415`, root checkout был чистым после интеграции; 175 прежних dirty файлов сверены в stash и salvage checkout. Точный итог — в integration-report.json. Сохранённые изменения старого main остаются в salvage checkout, повторно применяются только по смыслу. |
| 2 | FORK-119 → FORK-120 → FORK-121 → FORK-122; FEAT-084 → FEAT-085 → FEAT-086 | Подтвердить на точном артефакте разрешения, microphone/system capture, mute/gain, pause/resume, остановку и сохранность дорожек. Выбрать контрольный профиль; не закрывать по наличию окна. |
| 3 | FORK-123 → FORK-124 → FORK-125 | Связать пользовательский путь после [ревью source-аудита](../plans/webview-migration/2026-10-02-webview-migration-integration.md): incomplete journal сейчас прекращает запуск до UI, а file completion не подтверждает WebView import. Затем проверить реальный WebView/SQLite upgrade и recovery на контрольных данных. Тесты всех префиксов SQLx остаются обязательными; пользовательские данные не использовать как экспериментальный профиль. |
| Независимо от native | FEAT-101 → FEAT-102; FORK-145 → FORK-146 → FORK-147; FORK-149 (после FORK-035) | FEAT-078/100–102 приняты по source/integration и контролируемой локальной Gemma-проверке в рабочем main; installed native UI, сеть и release отдельно. CI-план принят SPEC/QUALITY; реализация macOS jobs и package остаётся отдельной. Scanner выявляет 48 замечаний. FEAT-089 сохраняет blocker FORK-042. |
| Затем | FORK-128 → FORK-129 → FORK-130 → FORK-131; FORK-152 → FORK-153 → FORK-154 | Storage/theme планы приняты SPEC/QUALITY без Critical/Important; [отчёт](../reviews/cross-cutting/plan-review-2026-10-02.md). Storage contract включает immutable derived versions и запрещает FFmpeg pathname fallback. Theme: 130/130 сценариев PASS в полном e2e; два Sidebar-эталона обновлены после допуска. Два устаревших локатора исправлены: targeted 2 PASS, полные spec 9 PASS, SPEC/QUALITY C0/I0/M0. Общий gate прошёл: 216 e2e PASS, canonical full 11/11 этапов PASS; native приёмка и интеграция открыты. |

Канонические статусы и зависимости остаются в
[macos-fork.md](../prd/macos-fork.md) и [fork-features.md](../prd/fork-features.md).
3 октября владелец выбрал обновление `dev` до `main@53f4415`. Предыдущий dev
сохранён в `themaddoxed/dev-before-main-2026-10-03`; новые `feature/*` начинаются
от проверенного dev. Для темы создан отдельный чистый checkout; текущий main
с незакоммиченными изменениями сохранён. В feature checkout prepaint
исправлен и проверен на 12 переходах; светлый контраст Settings исправлен,
persistence/OS-pressure дали 4 PASS. Focus/geometry дали 22 PASS, включая все
шесть Tab-измерений выше 3:1 и отсутствие overflow; policy-fixtures — 1 PASS,
регрессия текста/alpha — 8 PASS. 4 октября реальные Home/Settings/Meeting Details
дали 2 PASS и 18 текстовых измерений выше 4,5:1; механизм доказательства внешней
тени — 9 PASS и SPEC/QUALITY C0/I0/M0. Фоновая проверка принята SPEC/QUALITY
C0/I0/M0; 45 регрессионных сценариев PASS. Полный набор пяти theme-suite:
128 PASS / 2 FAIL; реальные фоны всех трёх страниц PASS. Два FAIL — старые
снимки Sidebar. Узкое обновление эталонов принято SPEC/QUALITY C0/I0/M0;
обычное сравнение двух снимков — 2 PASS. `check.sh --full`: exit 1, восемь
этапов PASS, три Rust-этапа FAIL из-за пустого offline-пути ONNX Runtime.
Повтор с локальной библиотекой: три FAIL exit 101 на отсутствующем helper.
Текущий Intel helper собран; три повторных Rust-этапа PASS exit0
(workspace 603 PASS, 10 ignored; отдельный FFmpeg recovery 1 PASS); native остаётся открытым; [текущий отчёт](evidence/theme-progress-2026-10-03.json).
Последующий обязательный полный `test:e2e`: exit 1, 210 PASS / 6 FAIL.
Все 130 theme-сценариев на текущих байтах PASS. Изолированный повтор шести
сценариев: 4 PASS / 2 FAIL. После независимого допуска исправлены два устаревших
локатора; targeted 2 PASS, полные два spec 9 PASS, SPEC/QUALITY C0/I0/M0.
Повтор обязательного полного e2e: 216 PASS, exit 0, без пропусков.
Канонический `check.sh --full` на том же diff: exit 0, все 11 этапов PASS,
пропущенных этапов нет, 416 секунд. Использованы локальная ONNX Runtime и
current-source Intel helper; Rust 603 PASS / 10 ignored, отдельный FFmpeg 1 PASS.
Прежние FAIL сохранены. Нативная матрица macOS и интеграция ещё открыты.
[Project evidence](evidence/theme-project-full-2026-10-04.json).
Сохранённые старые ветки и worktree не удалять до учёта перенесённых возможностей.

4 октября выполнена независимая часть CI-предпосылок: свежие ONNX Runtime
пакеты ARM64/Intel получены по URL из locked `ort-sys` 2.0.0-rc.10; SHA-256
пакетов и SHA-256/архитектуры извлечённых библиотек совпали с manifest.
В пакетах нет LICENSE/NOTICE; допуск build/license provenance, FFmpeg producers
и actual hosted jobs остаётся открытым.
[Свидетельства](evidence/macos-ci-onnx-source-census-2026-10-04.json).

Свежие FFmpeg-пакеты ARM64/Intel также совпали с release metadata и project
manifest; штатные проверки архитектур завершились exit 0 без запуска скачанных
бинарников. Независимые SPEC/QUALITY подтвердили только байты и привязки
источников, C0/I0/M0. Это не закрывает producer/source/license admission,
остальные native inputs или реальные CI jobs.
[FFmpeg evidence](evidence/macos-ci-ffmpeg-source-census-2026-10-04.json),
[границы ревью](evidence/macos-ci-native-acquisition-reviews-2026-10-04.json).
Intel-пакет дополнительно совпал целиком с историческим архивом Evermeet,
на который ведёт официальный FFmpeg download page. Это byte binding,
без проверки подписи и полноты source/license provenance.
[Publisher binding](evidence/macos-ci-ffmpeg-intel-publisher-binding-2026-10-04.json).
ARM-архив также совпал целиком с историческим пакетом OSXExperts; его
website-license не заменяет source/notice provenance бинарника с `--enable-gpl`.
[ARM binding](evidence/macos-ci-ffmpeg-arm-publisher-binding-2026-10-04.json).

Свежая загрузка теперь выполнена для всех восьми CI-входов. Sherpa/model
пакеты совпали по size/SHA, все 30 статических библиотек имеют ожидаемую thin
архитектуру; три root LICENSE прочитаны и сверены с bundled текстами.
Четыре missing и четыре same-size corrupt пробы отвергнуты, но идентичные
prior-run копии принимаются нынешним hash-only verifier. Обязательная проверка
same-run receipt/replay остаётся открытой; product CI changes и hosted jobs
не выполнялись. [Текущий сводный статус](evidence/macos-ci-acquisition-progress-2026-10-04.json),
[diarization evidence](evidence/macos-ci-diarization-source-census-2026-10-04.json).


Перед storage-реализацией сравнение 62 путей выявило четыре несовпадения
с чистым dev `53f4415`: API, регистрация команд, context repository и его тест.
План содержит незакоммиченный FEAT-100 overlay; новые QA-команды также требуют
перепривязки caller census. Чистый dev пока не допущен как storage source.
[Source comparison](evidence/recording-storage-source-comparison-2026-10-04.json).
[Caller drift](evidence/recording-storage-caller-drift-2026-10-04.json) фиксирует
четыре изменившихся пути прежнего caller census и два context-read вызова
нового QA-потребителя; расширенный lexical inventory пока не допущен.


Для переноса FEAT-100/101/102 в dev подготовлен изолированный QA-кандидат
из 15 файлов на base `53f4415`; root и чистый dev не переключались. Первый
полный gate завершился exit 1: 10 этапов PASS, frontend unit FAIL из-за
отсутствующего MeetingQuestions mock. Однострочное исправление harness
сохраняет assertions; пять затронутых auto-summary тестов проходят.
Повтор на прежних 14 файлах, ошибочно начатый до завершения подготовки,
остановлен (exit 143) и не является проверкой исправления. Исправленный
15-path full gate завершился exit 0: 11/11 этапов PASS за 438 секунд.
После прогона сверены все 15 source hashes и точный status кандидата.
Commit/FF dev ожидают отдельного допуска;
этот кандидат не подтверждает native или release readiness.
[Точный патч и свидетельства](evidence/qa-transfer-fixed-2026-10-04.json).

4 октября обновлён source-аудит FORK-123: 11 paths сверены с base53,
lib.rs содержит сохранённую QA-регистрацию. Независимые SPEC/QUALITY проверили
незавершённый recovery-contract; реализация ещё не допущена. Исправлена одна
Minor-ссылка на locked Tauri dispatch. Реальный Chromium/IDB/localStorage
probe с явно введённым preference fault после перезагрузки вернул
already_imported без восстановления preference, без дубля записи. Это
adapter/readback evidence, не пользовательский migration flow или crash PASS.
[Contract](../plans/webview-migration/2026-10-04-webview-recovery-contract.md),
[review boundaries](evidence/webview-recovery-contract-review-status-2026-10-04.json),
[browser diagnostic](evidence/webview-migration-browser-marker-2026-10-04.json).


Продолжение recovery design: независимые SPEC/QUALITY выявили в v2
недолговременный skip-переход (restart снова давал first_launch); SPEC также
выявил недоступность прежнего SQLite-import при новой изоляции. Отдельный
[v3 contract](../plans/webview-migration/2026-10-04-webview-recovery-contract-v3.md)
описывает owned pending/completed first-launch admission, fresh DB без обычного
AppState и scoped legacy discovery/select/import. Bounded SPEC/QUALITY v3: C0/I0/M0; это закрывает два design-замечания,
но не означает допуск полного плана или продукта. Fresh-only API ещё нужно
выделить явно: generic manager constructor может импортировать legacy DB.
[22-path census и границы](evidence/webview-recovery-contract-v3-progress-2026-10-04.json)
фиксируют 15 существующих и 7 новых предложенных путей; прежние 11 source hashes
сохранились. [Locked plugin source](evidence/webview-plugin-source-2026-10-04.json)
сверен с Cargo.lock без исполнения. Production implementation остаётся
NOT_ADMITTED: exact schemas, plugin/CSP, clean source и actual identity pair
не закрыты. Пользовательские данные и продукт этого этапа не менялись.


[Fresh/persistence amendment v2](../plans/webview-migration/2026-10-04-webview-fresh-persistence-amendment-v2.md)
уточняет isolated fresh-only pool API, строгие frozen defaults и immutable
admission/receipt publication. Первое ревью выявило невозможность напрямую
использовать cross-identity manifest для fresh той же identity. Исправленный
shared-mechanics seam имеет отдельные legacy/fresh authority types, сохраняет
legacy identity guard и disjoint sibling roots. Bounded SPEC/QUALITY fix:
C0/I0/M0. [23-path census и точные границы](evidence/webview-fresh-persistence-progress-2026-10-04.json)
сохраняют исходные FAIL-отчёты; manager.rs добавлен только в предложенный scope.
[Locked desktop plugin lifecycle](evidence/webview-plugin-lifecycle-source-2026-10-04.json)
подтверждает в исходниках возможность normal-only runtime регистрации плагинов;
actual sequencing, recovery CSP/IPC и native store continuity не проверены.
Продукт не менялся; canonical schemas/finite ownership, clean source и actual
pair продолжают блокировать implementation admission.


[Schema/first-ownership candidate](../plans/webview-migration/2026-10-04-webview-schema-ownership.md)
задаёт первый immutable intent без рекурсивной owner-зависимости, fail-closed
unbound setup до его durability, mapping auto-generated IDB transcript keys
в той же records/marker transaction и отдельную canonical encoding без изменения
browser package v1. Standalone Python/Node дали одинаковые bytes/SHA-256 для
10 [golden vectors](evidence/webview-canonical-golden-vectors-2026-10-04.json).
QUALITY independently повторил Node: 10/10, пять invalid inputs rejected;
это format evidence, не DTO parser или product/browser/native proof. SPEC выявил mixed-ID scheduling gap: ID-less запись могла занять последующий
явный id 1. [Schema v2](../plans/webview-migration/2026-10-04-webview-schema-ownership-v2.md)
фиксирует explicit-first ordering без изменения packageIndex/checksum; bounded
SPEC/QUALITY fix C0/I0/M0. [Реальный IDB probe](evidence/webview-mixed-id-browser-probe-2026-10-04.json)
подтвердил исходный ConstraintError/full rollback, контрольные ключи 1/2 и
сохранность соседней записи при настоящей коллизии. Это scheduling evidence,
не product fix или implemented versioned mapping; [точный статус](evidence/webview-schema-ownership-progress-2026-10-04.json).
Executable field types/limits/root-binding transitions, startup/CSP, clean source
и actual pair остаются implementation gates; FORK-123 не закрыта.

Ниже сохранён исторический план сентября: он объясняет источники и прежние
ограничения, но не возвращает поиск в очередь реализации.

## Историческая очередь 25 сентября при недоступном Computer Use

FEAT-186–188, FORK-189, FEAT-083 и FEAT-087/088 закрыты в канонических PRD. Выбранный combined candidate установлен в `/Applications/Meetily.app`; его executable SHA-256 `b98032e7…` и исходный diff `0609ae4c…` не изменились. Обычный запуск и наличие окна подтверждены, но frontend-контролы, запись и TCC-матрица не проверены. После обновления этой сессии Codex API Computer Use для нативных приложений отсутствует. AppleScript видит окно, но его верхний AX-слой пока не раскрывает кнопки WebView. Это ограничение инструмента и доказательств, не новый blocker для независимых code/docs задач.

**Историческая очередь от 24 сентября; текущая очередь указана выше. Следующим действием тогда был FEAT-094.** В параллельном, не затрагивающем запись потоке можно взять FORK-143 и FORK-148. Все изменения продукта выполнять в отдельно выбранном и заново проверенном product checkout; эта docs-ветка остаётся источником задач. Не переносить незакоммиченный combined candidate или пользовательские записи ради начала этих карточек.

| Порядок | Существующие карточки | Результат без нативного управления | Следующий gate |
|---|---|---|---|
| 1 | FEAT-094 → FEAT-095 → FEAT-096 | Индекс только видимого текста встречи, обновление/удаление и переход к источнику; адресные unit/integration и headless browser проверки | Поведение установленной `.app` не заявлять по одним кодовым тестам |
| Параллельно | FORK-143 → FORK-144 | Перепривязанный к текущей базе CI-план и независимое review; без изменения секретов и публикации | FORK-145 и дальнейшие CI-gates по исходному графу |
| Параллельно | FORK-148 | Узкий scanner platform surface с контрольным запрещённым случаем | FORK-149 ждёт также FORK-035 и проверку package |
| Затем/независимо | FORK-126 → FORK-127; FORK-150 → FORK-151 | Актуальные планы folder/storage и theme плюс независимое review | Реализация только после соответствующих plan/review и исходных зависимостей |
| Независимо | FORK-174 | Точная матрица Alpha: требование → артефакт → устройство → ещё недостающее доказательство | Не заменяет FORK-175–180 и release-проверки |
| Частично независимый code-поток | FORK-123 | Связать готовые WebView migration adapter и Rust journal на проверенном срезе, проверяя код на тестовых данных | До закрытия подтвердить пользовательский WebView-путь; FORK-124/125 требуют контрольного профиля и recovery-доказательств |

FEAT-076 принят по [исходным критериям поиска](meeting-search-acceptance-2026-10-02.md): все три области, переход ко второй странице, скрытые данные и удаление/reopen проверены; installed app остаётся отдельной приёмкой.

FORK-133 выполнен как [проверка исходников и изолированных fixtures](gemma-path-2026-10-02.md): общий downloader, каталог/шаблон и выбор проверены без скачивания реальных весов. FORK-134/167 остаются отдельной Intel/Apple Silicon приёмкой с реальными моделями. FORK-135 ждёт разрешённого корпуса реальных записей. FEAT-117 требует актуальных условий провайдера и продуктового решения; FORK-155 — выбора имени и bundle ID владельцем. Эти карточки не смешиваются с немедленной code-очередью только потому, что у них нет незакрытого `@blocked_by`.

**Нативная очередь остаётся отдельной:** FORK-119 → FORK-120 → FORK-121 → FORK-122 и FEAT-084 → FEAT-085 → FEAT-086. FORK-119 формально имеет закрытый task blocker, но остаётся `@status:blocked`: отдельный тестовый профиль и его исходное TCC-состояние не подтверждены, управляемый frontend-сценарий на exact app не выполнен. Не закрывать её по одному PID/окну или по исходникам. FEAT-105 → FEAT-106 остаются за FORK-051; их зависимость не снимается ради обхода Computer Use. Нативную матрицу можно исполнить любым доказуемым способом управления macOS, если доступны контроль окна, аудиофайлов, разрешений и физического устройства; конкретный инструмент не является критерием приёмки.

Новых task ID и dependency edges это обновление не создаёт. Канонические статусы и порядок зависимостей остаются в [macos-fork.md](../prd/macos-fork.md) и [fork-features.md](../prd/fork-features.md). Ниже — исходный план от 22 сентября для контекста.

## Откуда продолжать

| Роль | Проверенный контекст |
|---|---|
| Продуктовый код | `themaddoxed/meetily-macos`, `themaddoxed/feat071-audio-controls`, чистый `61e890f444f81718d8136beae6afc6f61a5f0e44` |
| Входной handoff | `themaddoxed/task-decomposition-plan`, `0074708f3f5bc3c3f37952b07c4f3dea071757a1` |
| Единственная редактируемая очередь | [macos-fork.md](../prd/macos-fork.md) и [fork-features.md](../prd/fork-features.md) в checkout этой docs-ветки |
| Локальный checkout плана | `.worktrees/task-decomposition-publish` относительно корня рабочего репозитория |
| Незавершённая работа | `fork-051-permissions-lifecycle` на `f35d864` с целевыми и временными диагностическими правками; не входит в чистую базу продукта |

Источники: [handoff](codex-handoff-2026-09-22.md), его раздел «Текст для стартовой сессии Codex», и [архитектурное обоснование](architecture-review-2026-09-22.md). Последнее уточнение пользователя ограничивает эту сессию законченным планом для mdtask. Выполнение одной продуктовой задачи переносится в следующую сессию.

Ранее canonical ledger был в `macos-fork-foundation`; опубликованная декомпозиция сохранила его задачи и историю. Продолжать план нужно здесь, а не редактировать одновременно root/foundation/product копии PRD. Product checkout даёт исходники, PRD этой ветки — очередь. Перед реализацией сверить свежий HEAD/dirty state; не использовать код docs-ветки или preservation main. Git-контекст уточнён в [существующей политике](git-policy.md#execution-context-2026-09-22); ветки, история и destination этим планом не перемещены.

## Первая пачка 22 сентября: один результат за проход (исполнена)

На момент составления исходного плана следующей задачей была **FEAT-186**. Сейчас FEAT-186–188 и FORK-189 закрыты; актуальный следующий шаг указан выше. Исходные оценки и критерии ниже сохранены для сверки истории.

| Приоритет | Карточка | Результат | Активная оценка / готовность |
|---|---|---|---|
| 1 | FEAT-186 (R3) | Отмена, ранние ошибки и cleanup относятся к своей попытке summary | 2–4 ч; ready |
| 2 | FEAT-187 (R4a) | Устаревшая попытка не публикует и не откатывает результат новой | 3–5 ч; после FEAT-186 |
| 3 | FEAT-188 (R4b) | Ручной текст, генерация и файл сходятся после ошибки/reopen | 3–5 ч; после FEAT-187 и его контракта версии |
| Параллельно независимому summary | FORK-189 (R2) | Узкие docs/audio/summary маршруты существующего check | 2–4 ч; ready; не blocker продукта |
| При готовом окружении | FORK-119–122 и FEAT-083–086 | Один exact-artifact native-пакет, отдельные результаты mic/system/TCC/device/pause/stop/Quit/reopen | 6–13 ч по существующим карточкам; аппаратная приёмка остаётся открытой |

R1 выполнен этим согласованием контекста и зависимостей; отдельная новая задача планирования не создана. Сохранены 160 существующих ID, включая 22 закрытых; добавлены только четыре задачи выше. Старые 103 шага не перепроектировались. Номера назначены `mdtask ids`.

На 61e890f узко проверены `summary/service.rs` (registry/cleanup, file/title до DB acceptance), `summary/commands.rs` (ранние ошибки и chunks после create/reset, отдельный manual save), summary/meeting repositories и persistence helpers. Отдельный read-only repo_scout подтвердил эти участки на том же чистом SHA. Это статическая проверка, не воспроизведение гонки и не свежий PASS тестов. Будущие карточки принимают результаты `confirmed`, `already-fixed`, `not-reproduced`, `needs-input`; закрытие требует доказательства выбранного исхода. Старые FEAT-067/068 остаются закрытыми.

`--quick` по коду всё ещё запускает почти весь автоматический gate, исключая production build. Поэтому R2 полезен, но до него использовать явные адресные команды. Общий check не является обязательным первым шагом каждого summary исправления.

### Native-ограничение и смысл зависимостей

FORK-122 ждёт FORK-121, **не FEAT-086**. Microphone lifecycle закрывается по собственным TCC/device/stop/Quit/reopen критериям. FEAT-071 ждёт FORK-051 и собственные FEAT-083–086. Общий артефакт не означает общую галочку приёмки.

Свежая локальная запись FORK-051 сообщает о падении последнего exact `.app` в `tao::did_finish_launching` на macOS 15.8; диагностический debug-запуск не закрывает native gate. Существуют предыдущие положительные mic-only/Quit/TCC наблюдения, но полного hot-unplug и текущей UI/TCC матрицы нет. Эти сведения сохранены в FORK-051 с границей доказательств; новые аппаратные прогоны здесь не выполнялись. FORK-119 сначала сверяет целевые/диагностические правки и выбирает работоспособный проверяемый артефакт. Пока его нет, продолжать FEAT-186, а не повторять упаковку без новой гипотезы.

Предположение R5 об unbounded teardown пока не подтверждено зависанием. В пределах FORK-122 при относящемся к нему симптоме — один controlled backend/watchdog тест, различающий ACK timeout и окончание owner; raw/checkpoints не удалять. Process isolation и отдельный проект recorder без воспроизведения не планируются как обязательный fix.

### Как исполнять и проверять граф

Из checkout плана:

```sh
mdtask --path docs/prd list --tag batch1
mdtask --path docs/prd view FEAT-186
mdtask --path docs/prd validate
```

`@parent` — метаданные, не автоматическое закрытие. Каждая открытая итоговая задача теперь явно зависит от своих уже назначенных детей через `@blocked_by`; сохранены её исходные зависимости. Закрытые строки и их статусы не менялись. Граф зависимостей редактируется только в PRD; таблицы здесь показывают приоритет, а не вторую копию графа. `ready` означает отсутствие task blockers; для native/user/external всё равно нужен указанный в карточке вход.

Один implementer на участок кода. Summary и scoped check могут выполняться независимо; запись и storage/migration одновременно не менять в общем checkout. Независимый review — по затронутому риску, без review-of-review. Одна реализация включает regression и критерии приёмки, а не новую цепочку задач «plan → review → test → review». После двух неуспешных циклов по одной причине сохранить исходный fail и конкретный blocker. Один полный integration gate выполняется на объединённом проверяемом снимке; публичные release gates сохраняются.

## Следующие пачки по исходному плану 22 сентября

**Пачка 2 — внутренний рабочий кандидат.** Это исходная формулировка; FEAT-087/088 уже закрыты по указанной в PRD source/behavior проверке, без сетевого native runtime. После первой пачки планировалось взять минимальную privacy-проверку FEAT-087/088 в FEAT-082 (включая содержимое логов, local/cloud/update/model запросы) и минимальный first-run путь FEAT-080. Проверку настоящей системной дорожки и долговечных requested/actual sources, предупреждений после reopen/export/reprocess держать внутри FEAT-071; после FEAT-083 уточнить минимальную карточку по фактическому gap, не открывать второй audio-проект. Не подменять отсутствие источника кратким toast или распознанным говорящим.

Capture без готового STT (R6) остаётся **предложением, требующим явного принятия продуктового контракта**. Пока не принято, оно не blocker внутреннего кандидата и не обязательный дефект. При принятии: raw только при разрешённом сохранении, статус transcription unavailable и поздняя обработка, без скрытого изменения `auto_save=false`.

Результат пачки 2 — один внутренний сценарий record → stop → reopen → локальная обработка → summary → существующий Markdown export. Дополнительно одна длительная встреча через несколько checkpoint с pause/reopen и заранее выбранными бюджетами RAM/CPU/очередей. Использовать существующие критерии FORK-052/FEAT-071 и evidence; не закрывать целиком FORK-052, пока его upgrade/offline blockers не выполнены. Это внутренний кандидат D, а не публичный Alpha.

**Пачка 3 — ежедневная работа.** Поиск FEAT-076, полный простой TXT/MD/JSON экспорт в FEAT-075 и вопросы по одной встрече FEAT-078; компактные системные команды — в FORK-047. При взятии конкретной карточки пересмотреть её scope и зависимости: global search не является необходимым входом для single-meeting QA, diarization — для plain export. Старые связи FEAT-100/FEAT-078 здесь не сняты без изменения их объёма; они не означают утверждённую архитектурную необходимость. PDF/DOCX, люди/говорящие и автоматизации остаются позднее.

Identity, bundle-ID migration, signed updater, доставка и FORK-053 идут отдельной release-линией. Intel + arm64, minimum macOS 14.2, exact-artifact smoke, signing/notarization и пользовательский upgrade не ослаблены. Тестирование другими людьми B требует понятного first-run и безопасной доставки; ни D, ни B не закрывают полный публичный Alpha R. Файлы raw/checkpoints сохраняются как файлы, SQLite — канонический принятый текст и метаданные, readable exports — восстанавливаемые копии. ActuallyFree/Amanu используются как источники ограниченных контрактов, без объединения баз кода или повторного общего аудита.

## Исходные оценки 22 сентября и сохранённые дальние итерации

У первой пачки 10–18 активных часов summary и scoped checks плюс 6–13 часов существующей native-матрицы: суммарно **16–31 ч**, без внешнего ожидания и неизвестного ремонта native-приложения. При условных 4 полезных часах в день это около 4–8 рабочих дней до резерва; календарный дедлайн не задан. Это начальные диапазоны, не измеренная скорость модели. Уточнить их после первой выполненной карточки; не добавлять искусственные даты `@due`.

Ниже сохранена прежняя группировка I01–I19 для ориентирования в остальных карточках. Она предварительная; номер не задаёт обязательный порядок и не требует закончить весь release до внутреннего кандидата. Ближайшие пачки выше имеют приоритет. Оценки дальних итераций не перепроверялись, их сумма не используется как новый срок Alpha.

| Итерация | Проверяемый результат | Шагов | Активная работа |
|---|---|---:|---:|
| I01 | Надёжная запись на текущем Mac | 8 | 6–13 ч |
| I02 | Пользовательская миграция и план хранения | 5 | 4,5–10 ч |
| I03 | Папка записей и восстановление захвата | 5 | 5–13 ч |
| I04 | Модели, Intel benchmark и privacy | 6 | 6,5–12 ч |
| I05 | Говорящие и post-call управление | 6 | 7–13 ч |
| I06 | macOS-only CI и проверка поверхности | 7 | 5,5–13,5 ч |
| I07 | Тема и структура интерфейса | 5 | 4,5–10 ч |
| I08 | Финальная identity, миграция и updater | 7 | 6–14 ч |
| I09 | Системные команды, окна и основные экраны | 5 | 4,5–12,5 ч |
| I10 | Финальные аппаратные и сквозные проверки функций | 6 | 6,5–13 ч |
| I11 | Подписанный update и финальный кандидат | 5 | 4,5–11 ч |
| I12 | Alpha на двух архитектурах и minimum macOS | 3 | 2,5–8 ч |
| I13 | Отдельно разрешённый handoff | 2 | 1,5–3,5 ч |
| I14 | Поиск и полный экспорт | 8 | 7,5–17,5 ч |
| I15 | Люди и вопросы по встрече | 6 | 6–12 ч |
| I16 | Несколько встреч и первый запуск | 5 | 5–10 ч |
| I17 | Обнаружение звонка и добровольная автозапись | 6 | 6–12 ч |
| I18 | Тихие участники и шумоподавление | 3 | 4–7 ч |
| I19 | Ограниченные исследования streaming/provider | 5 | 4,5–10,5 ч |

Фиксированные внешние входы: устройства/разрешения для native; окончательная identity для реального bundle-ID upgrade; Developer ID/notarization и выбранная доставка для публичного артефакта. Их отсутствие блокирует относящуюся приёмку, а не независимый summary fix. Commit, push/PR, публикация, ключи, TCC reset, миграция или удаление данных пользователя не входят в этот план.

## 4 октября: локальный кандидат исправления legacy IDB mixed-ID

В отдельном кандидате на чистом `dev@53f4415` исправлено столкновение с явными ID при автоматическом назначении ключей transcript. Первый вариант explicit-first отклонён: фактический публичный импорт с явным `2^53` выявил регрессию исчерпания генератора. Финальный вариант пропускает зарезервированные ключи внутри одной транзакции и удаляет только собственную только что добавленную временную строку; входной snapshot сохранён.

Проверки кандидата: четыре real-IDB сценария, public-import boundary probe, Node 424/Vitest 61, typecheck/lint, 29 critical browser tests и build — фактический exit 0. Два независимых ревью точных двух файлов: C0/I0/M0. Связанные исходники, патч, SHA и ограничения закреплены в `docs/fork/evidence/webview-mixed-id-fix-v2-2026-10-04.json`; историческая регрессия сохранена отдельно.

Это не коммит и не перенос в `main`/`dev`: оба ref сохраняют `53f4415`, исходный dev чист. FORK-123 и полный план остаются открытыми; combined coordinator, SQLite/FS/native user-flow и release gate не подтверждены. Следующая интеграция использует отдельное согласование QA commit/FF dev, уже ожидающее ответа; текущий этап не расширяет это разрешение.
