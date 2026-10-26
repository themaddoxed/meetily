# Meetily — индекс публичного снимка

Это независимый снимок исходников без прежней Git-истории. Начать с
[правил чистой публикации](docs/development/public-source-snapshot.md).
Исторические отчёты сохранены в обезличенном виде; старые SHA, ветки и результаты
не являются текущими refs или подтверждением готовности нового снимка.

## Быстрый вход для следующего агента

| Нужно | Источник |
| --- | --- |
| Понять правила работы | [AGENTS.md](AGENTS.md) |
| Найти код, назначение папок и автора работы | [Карта репозитория](docs/development/repository-map.md) |
| Продолжить задачи по порядку | [План: автоматические проверки сначала, ручные в конце](docs/development/execution-order.md) |
| Найти текущую карточку и её зависимости | [macOS PRD](docs/prd/macos-fork.md), [feature PRD](docs/prd/fork-features.md) |
| Проверить, где сохранилась незавершённая работа | [Реестр worktree](docs/development/worktree-inventory.md) |
| Найти план или ревью по направлению | [Планы](docs/plans/README.md), [ревью](docs/reviews/README.md) |
| Разобраться с прежним путём | [155 перемещений: путь → путь, SHA и provenance](docs/development/evidence/2026-10-18-structure-moves.json) |
| Проверить исходное состояние файлов и авторство коммитов | [Снимок до реорганизации](docs/development/evidence/2026-10-18-workspace-before.json) |
| Проверить результат реорганизации | [Отчёт проверок структуры и сохранности](docs/development/evidence/2026-10-18-structure-verification.json) |
| Выбрать автоматические проверки | [Testing guide](docs/testing.md), [check.sh](scripts/check.sh) |
| Проверить релиз | [Release candidate](scripts/release-candidate.sh), [smoke template](docs/release/smoke-report-template.md), [release gate](scripts/release-gate.sh) |

Статусы задач определяются PRD и свидетельствами конкретной версии. Исторические
планы и ревью не являются ещё одним backlog. `.mdtaskrc` ограничивает поиск
папкой `docs/prd`; явная команда: `mdtask validate --path docs/prd`.

## Устройство проекта

```text
INDEX.md                         полный вход и каталог файлов
AGENTS.md                        правила и обязательные release gates
CONTEXT.md                       язык предметной области
frontend/
  src/                           UI и сервисы WebView
  src-tauri/
    src/                         Rust/Tauri: запись, БД, модели, команды
    migrations/                  схема SQLite
    tests/                       Rust интеграционные проверки
    resources/, templates/       ресурсы приложения и шаблоны
  tests/                         Node, Vitest, Playwright, fixtures
  scripts/                       frontend и release policy tools
  public/                        статические ресурсы интерфейса
llama-helper/                    процесс локальной модели
scripts/                         общие проверки и выпуск артефактов
.github/                         CI и GitHub templates
docs/
  prd/                           канонические задачи
  plans/<topic>/                 планы и census по направлениям
  reviews/<topic>/               авторские заключения по направлениям
  development/                   навигация, execution order, Git, сборки
    evidence/                    provenance реорганизации
  adr/, architecture/            архитектурные решения и контракты
  migration/                     выборочный перенос legacy
  release/                       шаблоны финальной приёмки
  assets/                        иллюстрации документации
  fork/                          исторические отчёты и стабильные ссылки
    evidence/                    исходные свидетельства
    migration-snapshot/          замороженный снимок
  superpowers/                   исторические планы/specs со стабильными путями
outputs/                         сохранённый отчёт воспроизведения ошибки
.worktrees/                      отдельные checkout и backup; см. реестр
```

Служебные `.git/`, `.beads/`, `.pnpm-store/`, `node_modules/`, `target/`,
WebView/build output, модели и локальные логи описаны в
[карте репозитория](docs/development/repository-map.md). Их содержимое не является
каноническим исходным материалом индекса. Пользовательские данные не переносились.

## Код по поведению

| Область | Основные точки входа |
| --- | --- |
| Команды и жизненный цикл приложения | [lib.rs](frontend/src-tauri/src/lib.rs), [state.rs](frontend/src-tauri/src/state.rs), [tray.rs](frontend/src-tauri/src/tray.rs) |
| Запись и источники аудио | [audio](frontend/src-tauri/src/audio), [RecordingSession](frontend/src-tauri/src/audio/recording_session.rs) |
| SQLite и данные встречи | [database](frontend/src-tauri/src/database), [migrations](frontend/src-tauri/migrations) |
| Summary и providers | [summary](frontend/src-tauri/src/summary), [API](frontend/src-tauri/src/api) |
| Вопросы по встрече | [meeting_questions.rs](frontend/src-tauri/src/meeting_questions.rs), [MeetingQuestions.tsx](frontend/src/components/MeetingDetails/MeetingQuestions.tsx) — код включён в публичный снимок |
| Распознавание и модели | [whisper_engine](frontend/src-tauri/src/whisper_engine), [parakeet_engine](frontend/src-tauri/src/parakeet_engine), [model_download.rs](frontend/src-tauri/src/model_download.rs) |
| Диаризация | [diarization](frontend/src-tauri/src/diarization) |
| Экспорт | [export.rs](frontend/src-tauri/src/export.rs), [MeetingDetails](frontend/src/components/MeetingDetails) |
| Экраны и настройки | [app](frontend/src/app), [components](frontend/src/components), [i18n](frontend/src/lib/i18n) |
| Сервисы и сохранение WebView | [services](frontend/src/services), [hooks](frontend/src/hooks), [contexts](frontend/src/contexts) |
| Локальный LLM | [llama-helper](llama-helper), [summary_engine](frontend/src-tauri/src/summary/summary_engine) |

## Направления, источники и границы готовности

| Направление | Где продолжать | Что известно на момент организации |
| --- | --- | --- |
| Q&A / контекст | [Планы](docs/plans/meeting-questions), [ревью](docs/reviews/meeting-questions), [manifest](docs/fork/evidence/qa-transfer-fixed-2026-10-04.json) | Реализация находится в локальных правках корня; не считать отдельным интегрированным коммитом |
| Semantic theme | [Планы](docs/plans/theme), [ревью](docs/reviews/theme), [свидетельства](docs/fork/evidence/theme-project-full-2026-10-04.json) | Изменения находятся в отдельном `semantic-theme` worktree; исторические тесты не заменяют проверку нового кандидата |
| WebView migration | [Планы](docs/plans/webview-migration), [ревью](docs/reviews/webview-migration) | Ревью контракта и исправление отдельного случая не доказывают весь upgrade/recovery |
| Recording storage | [План и census](docs/plans/recording-storage), [общее ревью](docs/reviews/cross-cutting/plan-review-2026-10-02.md) | Отдельная реализация сохранности и recovery остаётся в очереди |
| macOS CI | [Планы](docs/plans/macos-ci), [ревью](docs/reviews/macos-ci) | Отчёты о prerequisites/acquisition не подтверждают выпуск подписанного приложения |
| Аудио и permissions | [PRD](docs/prd/macos-fork.md), [исторический отчёт](docs/fork/fork051-feat071-verification-2026-09-14.md) | Код и отдельные автоматические результаты есть; native-приёмка — в финале |
| Доноры и legacy | [Feature intake](docs/fork/feature-intake.md), [legacy registry](docs/migration/legacy-feature-registry.md), [исторический fork registry](docs/fork/migration-registry.md) | Применимость переноса проверяется по конкретному источнику; ветки целиком не служат единицей интеграции |

Авторов ревью искать в самих документах; зафиксированное Git-авторство — в снимке
или `git log`. Автор текущих незакоммиченных правок не выводится из автора HEAD.
Новые навигационные документы этой организации подготовлены Codex в текущем чате.

## Политики и справочники

- [README](README.md), [CONTRIBUTING](CONTRIBUTING.md), [LICENSE](LICENSE.md), [PRIVACY_POLICY](PRIVACY_POLICY.md).
- [Доменный контекст](CONTEXT.md), [ADR: Rust-owned monolith](docs/adr/0001-rust-owned-modular-monolith.md), [архитектурный контракт](docs/architecture/macos-modular-monolith-contract.md).
- [Git workflow](docs/development/git-workflow.md), [локальные Universal-артефакты](docs/development/universal-macos-local-artifacts.md).
- [Исторические fork-материалы](docs/fork/README.md), [исторические планы](docs/superpowers/README.md).
- [Финальная acceptance matrix](docs/fork/alpha-acceptance-matrix-2026-10-02.md), [test-plan smoke template](docs/release/test-plan-smoke-report-template.md).

## Полный каталог файлов

Обновление: `python3 scripts/project-index.py`. Проверка актуальности:
`python3 scripts/project-index.py --check`. Раздел группирует файлы по папкам;
описания выше обновляются отдельно. Каталог включает новые незакоммиченные файлы,
поэтому он шире списка файлов в HEAD.

<!-- project-files:start -->

Всего файлов: **1017**. Каталог включает существующие tracked и
неигнорируемые untracked-файлы; наличие в списке не означает коммит или готовность.
Служебные каталоги и локальные runtime-артефакты описаны выше, без обхода содержимого.

<details>
<summary>Корень — 14</summary>

- [.gitattributes](.gitattributes)
- [.gitignore](.gitignore)
- [.mdtaskrc](.mdtaskrc)
- [AGENTS.md](AGENTS.md)
- [BLUETOOTH_PLAYBACK_NOTICE.md](BLUETOOTH_PLAYBACK_NOTICE.md)
- [CLAUDE.md](CLAUDE.md)
- [CONTEXT.md](CONTEXT.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [Cargo.lock](Cargo.lock)
- [Cargo.toml](Cargo.toml)
- [INDEX.md](INDEX.md)
- [LICENSE.md](LICENSE.md)
- [PRIVACY_POLICY.md](PRIVACY_POLICY.md)
- [README.md](README.md)

</details>

<details>
<summary>.github — 2</summary>

- [issue_template.md](.github/issue_template.md)
- [pull_request_template.md](.github/pull_request_template.md)

</details>

<details>
<summary>.github/ISSUE_TEMPLATE — 3</summary>

- [bug_report.md](.github/ISSUE_TEMPLATE/bug_report.md)
- [documentation.md](.github/ISSUE_TEMPLATE/documentation.md)
- [feature_request.md](.github/ISSUE_TEMPLATE/feature_request.md)

</details>

<details>
<summary>.github/workflows — 13</summary>

- [ACCELERATION_GUIDE.md](.github/workflows/ACCELERATION_GUIDE.md)
- [README_DEVTEST.md](.github/workflows/README_DEVTEST.md)
- [WORKFLOWS_OVERVIEW.md](.github/workflows/WORKFLOWS_OVERVIEW.md)
- [build-devtest.yml](.github/workflows/build-devtest.yml)
- [build-linux.yml](.github/workflows/build-linux.yml)
- [build-macos.yml](.github/workflows/build-macos.yml)
- [build-test.yml](.github/workflows/build-test.yml)
- [build-windows.yml](.github/workflows/build-windows.yml)
- [build.yml](.github/workflows/build.yml)
- [ci.yml](.github/workflows/ci.yml)
- [pr-main-check.yml](.github/workflows/pr-main-check.yml)
- [quality-gate.yml](.github/workflows/quality-gate.yml)
- [release.yml](.github/workflows/release.yml)

</details>

<details>
<summary>docs — 5</summary>

- [BUILDING.md](docs/BUILDING.md)
- [GPU_ACCELERATION.md](docs/GPU_ACCELERATION.md)
- [architecture.md](docs/architecture.md)
- [building_in_linux.md](docs/building_in_linux.md)
- [testing.md](docs/testing.md)

</details>

<details>
<summary>docs/adr — 1</summary>

- [0001-rust-owned-modular-monolith.md](docs/adr/0001-rust-owned-modular-monolith.md)

</details>

<details>
<summary>docs/architecture — 1</summary>

- [macos-modular-monolith-contract.md](docs/architecture/macos-modular-monolith-contract.md)

</details>

<details>
<summary>docs/assets — 15</summary>

- [1.png](docs/assets/1.png)
- [2.png](docs/assets/2.png)
- [3.png](docs/assets/3.png)
- [4.png](docs/assets/4.png)
- [5.png](docs/assets/5.png)
- [6.png](docs/assets/6.png)
- [7.png](docs/assets/7.png)
- [8.png](docs/assets/8.png)
- [Diagram-High level architecture diagram.jpg](docs/assets/Diagram-High%20level%20architecture%20diagram.jpg)
- [HighLevel.jpg](docs/assets/HighLevel.jpg)
- [Meetily-6.png](docs/assets/Meetily-6.png)
- [README.md](docs/assets/README.md)
- [logo1.png](docs/assets/logo1.png)
- [logo2.png](docs/assets/logo2.png)
- [logo3.png](docs/assets/logo3.png)

</details>

<details>
<summary>docs/development — 6</summary>

- [execution-order.md](docs/development/execution-order.md)
- [git-workflow.md](docs/development/git-workflow.md)
- [public-source-snapshot.md](docs/development/public-source-snapshot.md)
- [repository-map.md](docs/development/repository-map.md)
- [universal-macos-local-artifacts.md](docs/development/universal-macos-local-artifacts.md)
- [worktree-inventory.md](docs/development/worktree-inventory.md)

</details>

<details>
<summary>docs/development/evidence — 7</summary>

- [2026-10-18-structure-moves.json](docs/development/evidence/2026-10-18-structure-moves.json)
- [2026-10-18-structure-verification.json](docs/development/evidence/2026-10-18-structure-verification.json)
- [2026-10-18-workspace-before.json](docs/development/evidence/2026-10-18-workspace-before.json)
- [2026-10-25-publish-browser-repeat.log.gz](docs/development/evidence/2026-10-25-publish-browser-repeat.log.gz)
- [2026-10-25-publish-browser.log.gz](docs/development/evidence/2026-10-25-publish-browser.log.gz)
- [2026-10-25-publish-discovery.log.gz](docs/development/evidence/2026-10-25-publish-discovery.log.gz)
- [2026-10-25-publish-final.log.gz](docs/development/evidence/2026-10-25-publish-final.log.gz)

</details>

<details>
<summary>docs/fork — 31</summary>

- [README.md](docs/fork/README.md)
- [alpha-acceptance-matrix-2026-10-02.md](docs/fork/alpha-acceptance-matrix-2026-10-02.md)
- [architecture-review-2026-09-22.md](docs/fork/architecture-review-2026-09-22.md)
- [audio-errors-2026-09-19.md](docs/fork/audio-errors-2026-09-19.md)
- [baseline.md](docs/fork/baseline.md)
- [claude-code-provider-decision-2026-10-02.md](docs/fork/claude-code-provider-decision-2026-10-02.md)
- [codex-handoff-2026-09-22.md](docs/fork/codex-handoff-2026-09-22.md)
- [execution-plan-2026-09-22.md](docs/fork/execution-plan-2026-09-22.md)
- [execution-plan-review-2026-09-22.md](docs/fork/execution-plan-review-2026-09-22.md)
- [feat065-verification-2026-09-08.md](docs/fork/feat065-verification-2026-09-08.md)
- [feat069-070-verification-2026-09-13.md](docs/fork/feat069-070-verification-2026-09-13.md)
- [feat082-zero-analytics-2026-09-23.md](docs/fork/feat082-zero-analytics-2026-09-23.md)
- [feat083-contract-review-2026-09-22.md](docs/fork/feat083-contract-review-2026-09-22.md)
- [feat083-contract-review-2026-09-23.md](docs/fork/feat083-contract-review-2026-09-23.md)
- [feat087-088-privacy-2026-09-23.md](docs/fork/feat087-088-privacy-2026-09-23.md)
- [feature-intake.md](docs/fork/feature-intake.md)
- [first-batch-verification-2026-09-08.md](docs/fork/first-batch-verification-2026-09-08.md)
- [fork037-migration-design-2026-09-19.md](docs/fork/fork037-migration-design-2026-09-19.md)
- [fork051-feat071-verification-2026-09-14.md](docs/fork/fork051-feat071-verification-2026-09-14.md)
- [fork119-artifact-selection-2026-09-23.md](docs/fork/fork119-artifact-selection-2026-09-23.md)
- [gemma-path-2026-10-02.md](docs/fork/gemma-path-2026-10-02.md)
- [git-policy.md](docs/fork/git-policy.md)
- [meeting-context-2026-10-02.md](docs/fork/meeting-context-2026-10-02.md)
- [meeting-questions-2026-10-02.md](docs/fork/meeting-questions-2026-10-02.md)
- [meeting-questions-local-model-2026-10-03.md](docs/fork/meeting-questions-local-model-2026-10-03.md)
- [meeting-search-acceptance-2026-10-02.md](docs/fork/meeting-search-acceptance-2026-10-02.md)
- [migration-registry.md](docs/fork/migration-registry.md)
- [platform-support.md](docs/fork/platform-support.md)
- [platform-surface-2026-10-02.md](docs/fork/platform-surface-2026-10-02.md)
- [urgent-tasks-status-2026-09-21.md](docs/fork/urgent-tasks-status-2026-09-21.md)
- [urgent-verification-2026-09-19.md](docs/fork/urgent-verification-2026-09-19.md)

</details>

<details>
<summary>docs/fork/evidence — 65</summary>

- [audio-timeline-repair-2026-09-20.json](docs/fork/evidence/audio-timeline-repair-2026-09-20.json)
- [full-check-2026-09-20-final10.json](docs/fork/evidence/full-check-2026-09-20-final10.json)
- [full-check-2026-09-21-universal.json](docs/fork/evidence/full-check-2026-09-21-universal.json)
- [gemma-path-2026-10-02.json](docs/fork/evidence/gemma-path-2026-10-02.json)
- [macos-ci-acquisition-progress-2026-10-04.json](docs/fork/evidence/macos-ci-acquisition-progress-2026-10-04.json)
- [macos-ci-action-pins-2026-10-03.json](docs/fork/evidence/macos-ci-action-pins-2026-10-03.json)
- [macos-ci-actionlint-2026-10-03.json](docs/fork/evidence/macos-ci-actionlint-2026-10-03.json)
- [macos-ci-diarization-source-census-2026-10-04.json](docs/fork/evidence/macos-ci-diarization-source-census-2026-10-04.json)
- [macos-ci-ffmpeg-arm-publisher-binding-2026-10-04.json](docs/fork/evidence/macos-ci-ffmpeg-arm-publisher-binding-2026-10-04.json)
- [macos-ci-ffmpeg-arm-publisher-observation-2026-10-04.json](docs/fork/evidence/macos-ci-ffmpeg-arm-publisher-observation-2026-10-04.json)
- [macos-ci-ffmpeg-intel-publisher-binding-2026-10-04.json](docs/fork/evidence/macos-ci-ffmpeg-intel-publisher-binding-2026-10-04.json)
- [macos-ci-ffmpeg-source-census-2026-10-04.json](docs/fork/evidence/macos-ci-ffmpeg-source-census-2026-10-04.json)
- [macos-ci-native-acquisition-reviews-2026-10-04.json](docs/fork/evidence/macos-ci-native-acquisition-reviews-2026-10-04.json)
- [macos-ci-onnx-source-census-2026-10-04.json](docs/fork/evidence/macos-ci-onnx-source-census-2026-10-04.json)
- [macos-ci-prerequisites-2026-10-03.json](docs/fork/evidence/macos-ci-prerequisites-2026-10-03.json)
- [macos-ci-syntax-baseline-2026-10-03.json](docs/fork/evidence/macos-ci-syntax-baseline-2026-10-03.json)
- [meeting-context-2026-10-02.json](docs/fork/evidence/meeting-context-2026-10-02.json)
- [meeting-questions-2026-10-02.json](docs/fork/evidence/meeting-questions-2026-10-02.json)
- [meeting-questions-local-model-2026-10-03.json](docs/fork/evidence/meeting-questions-local-model-2026-10-03.json)
- [meeting-search-acceptance-2026-10-02.json](docs/fork/evidence/meeting-search-acceptance-2026-10-02.json)
- [native-audio-2026-09-20-final.json](docs/fork/evidence/native-audio-2026-09-20-final.json)
- [native-audio-2026-09-20-final10.json](docs/fork/evidence/native-audio-2026-09-20-final10.json)
- [native-audio-2026-09-20-final4.json](docs/fork/evidence/native-audio-2026-09-20-final4.json)
- [native-audio-2026-09-20-final5.json](docs/fork/evidence/native-audio-2026-09-20-final5.json)
- [native-audio-2026-09-20-final6.json](docs/fork/evidence/native-audio-2026-09-20-final6.json)
- [native-audio-2026-09-20-final7.json](docs/fork/evidence/native-audio-2026-09-20-final7.json)
- [native-audio-2026-09-20-final8.json](docs/fork/evidence/native-audio-2026-09-20-final8.json)
- [native-audio-2026-09-20-final9.json](docs/fork/evidence/native-audio-2026-09-20-final9.json)
- [native-system-track-audit-2026-09-20.json](docs/fork/evidence/native-system-track-audit-2026-09-20.json)
- [native-universal-2026-09-21.json](docs/fork/evidence/native-universal-2026-09-21.json)
- [plan-blocked-audit-2026-10-03.json](docs/fork/evidence/plan-blocked-audit-2026-10-03.json)
- [plan-remaining-audit-2026-10-04.json](docs/fork/evidence/plan-remaining-audit-2026-10-04.json)
- [platform-surface-2026-10-02.json](docs/fork/evidence/platform-surface-2026-10-02.json)
- [qa-autosummary-harness-fix-2026-10-04.json](docs/fork/evidence/qa-autosummary-harness-fix-2026-10-04.json)
- [qa-transfer-candidate-2026-10-04.json](docs/fork/evidence/qa-transfer-candidate-2026-10-04.json)
- [qa-transfer-fixed-2026-10-04.json](docs/fork/evidence/qa-transfer-fixed-2026-10-04.json)
- [recording-storage-caller-drift-2026-10-04.json](docs/fork/evidence/recording-storage-caller-drift-2026-10-04.json)
- [recording-storage-source-comparison-2026-10-04.json](docs/fork/evidence/recording-storage-source-comparison-2026-10-04.json)
- [theme-admission-2026-10-03.json](docs/fork/evidence/theme-admission-2026-10-03.json)
- [theme-background-proof-2026-10-04.json](docs/fork/evidence/theme-background-proof-2026-10-04.json)
- [theme-clean-admission-2026-10-03.json](docs/fork/evidence/theme-clean-admission-2026-10-03.json)
- [theme-matrix-red-2026-10-03.json](docs/fork/evidence/theme-matrix-red-2026-10-03.json)
- [theme-oracle-round3-2026-10-03.json](docs/fork/evidence/theme-oracle-round3-2026-10-03.json)
- [theme-prepaint-adapter-2026-10-03.json](docs/fork/evidence/theme-prepaint-adapter-2026-10-03.json)
- [theme-prepaint-diagnostic-v2-2026-10-03.json](docs/fork/evidence/theme-prepaint-diagnostic-v2-2026-10-03.json)
- [theme-prepaint-raw-head-failure-2026-10-03.json](docs/fork/evidence/theme-prepaint-raw-head-failure-2026-10-03.json)
- [theme-progress-2026-10-03.json](docs/fork/evidence/theme-progress-2026-10-03.json)
- [theme-project-full-2026-10-04.json](docs/fork/evidence/theme-project-full-2026-10-04.json)
- [theme-shadow-proof-2026-10-04.json](docs/fork/evidence/theme-shadow-proof-2026-10-04.json)
- [theme-sidebar-snapshots-2026-10-04.json](docs/fork/evidence/theme-sidebar-snapshots-2026-10-04.json)
- [webview-canonical-golden-vectors-2026-10-04.json](docs/fork/evidence/webview-canonical-golden-vectors-2026-10-04.json)
- [webview-fresh-persistence-progress-2026-10-04.json](docs/fork/evidence/webview-fresh-persistence-progress-2026-10-04.json)
- [webview-migration-browser-marker-2026-10-04.json](docs/fork/evidence/webview-migration-browser-marker-2026-10-04.json)
- [webview-migration-rebind-2026-10-04.json](docs/fork/evidence/webview-migration-rebind-2026-10-04.json)
- [webview-migration-rebind-corrected-2026-10-04.json](docs/fork/evidence/webview-migration-rebind-corrected-2026-10-04.json)
- [webview-mixed-id-browser-probe-2026-10-04.json](docs/fork/evidence/webview-mixed-id-browser-probe-2026-10-04.json)
- [webview-mixed-id-fix-2026-10-04.json](docs/fork/evidence/webview-mixed-id-fix-2026-10-04.json)
- [webview-mixed-id-fix-v2-2026-10-04.json](docs/fork/evidence/webview-mixed-id-fix-v2-2026-10-04.json)
- [webview-plugin-lifecycle-source-2026-10-04.json](docs/fork/evidence/webview-plugin-lifecycle-source-2026-10-04.json)
- [webview-plugin-source-2026-10-04.json](docs/fork/evidence/webview-plugin-source-2026-10-04.json)
- [webview-recovery-contract-review-status-2026-10-04.json](docs/fork/evidence/webview-recovery-contract-review-status-2026-10-04.json)
- [webview-recovery-contract-v3-progress-2026-10-04.json](docs/fork/evidence/webview-recovery-contract-v3-progress-2026-10-04.json)
- [webview-recovery-transport-source-2026-10-04.json](docs/fork/evidence/webview-recovery-transport-source-2026-10-04.json)
- [webview-schema-ownership-progress-2026-10-04.json](docs/fork/evidence/webview-schema-ownership-progress-2026-10-04.json)
- [webview-startup-managed-state-source-2026-10-04.json](docs/fork/evidence/webview-startup-managed-state-source-2026-10-04.json)

</details>

<details>
<summary>docs/fork/evidence/native-audio-2026-09-19 — 146</summary>

- [audio-devices-20260920.json](docs/fork/evidence/native-audio-2026-09-19/audio-devices-20260920.json)
- [audio-final-source-after.txt](docs/fork/evidence/native-audio-2026-09-19/audio-final-source-after.txt)
- [audio-final-source-before.txt](docs/fork/evidence/native-audio-2026-09-19/audio-final-source-before.txt)
- [audio-recovery-fixed-provenance.json](docs/fork/evidence/native-audio-2026-09-19/audio-recovery-fixed-provenance.json)
- [audio-round2-full-checks-sha256.json](docs/fork/evidence/native-audio-2026-09-19/audio-round2-full-checks-sha256.json)
- [audio-round2-source-after.txt](docs/fork/evidence/native-audio-2026-09-19/audio-round2-source-after.txt)
- [audio-round2-source-before.txt](docs/fork/evidence/native-audio-2026-09-19/audio-round2-source-before.txt)
- [audio-smoke-awake-edit.json](docs/fork/evidence/native-audio-2026-09-19/audio-smoke-awake-edit.json)
- [ax-before-menu-quit.txt](docs/fork/evidence/native-audio-2026-09-19/ax-before-menu-quit.txt)
- [ax-launchservices-denied-attempt.txt](docs/fork/evidence/native-audio-2026-09-19/ax-launchservices-denied-attempt.txt)
- [ax-launchservices-denied-home.txt](docs/fork/evidence/native-audio-2026-09-19/ax-launchservices-denied-home.txt)
- [ax-long-checkpoint1.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-checkpoint1.txt)
- [ax-long-checkpoint2.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-checkpoint2.txt)
- [ax-long-completed.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-completed.txt)
- [ax-long-pause-held.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-pause-held.txt)
- [ax-long-paused.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-paused.txt)
- [ax-long-resumed.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-resumed.txt)
- [ax-long-retry.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-retry.txt)
- [ax-long-start.txt](docs/fork/evidence/native-audio-2026-09-19/ax-long-start.txt)
- [ax-permission-allow-recording.txt](docs/fork/evidence/native-audio-2026-09-19/ax-permission-allow-recording.txt)
- [ax-preflight-open.txt](docs/fork/evidence/native-audio-2026-09-19/ax-preflight-open.txt)
- [ax-preflight-paused-later.txt](docs/fork/evidence/native-audio-2026-09-19/ax-preflight-paused-later.txt)
- [ax-preflight-paused.txt](docs/fork/evidence/native-audio-2026-09-19/ax-preflight-paused.txt)
- [ax-preflight-resumed.txt](docs/fork/evidence/native-audio-2026-09-19/ax-preflight-resumed.txt)
- [ax-quit-recording.txt](docs/fork/evidence/native-audio-2026-09-19/ax-quit-recording.txt)
- [ax-recover-selected.txt](docs/fork/evidence/native-audio-2026-09-19/ax-recover-selected.txt)
- [ax-recovered.txt](docs/fork/evidence/native-audio-2026-09-19/ax-recovered.txt)
- [ax-reopened.txt](docs/fork/evidence/native-audio-2026-09-19/ax-reopened.txt)
- [ax-system-delay-settled.txt](docs/fork/evidence/native-audio-2026-09-19/ax-system-delay-settled.txt)
- [ax-timer-completed.txt](docs/fork/evidence/native-audio-2026-09-19/ax-timer-completed.txt)
- [ax-timer-paused1.txt](docs/fork/evidence/native-audio-2026-09-19/ax-timer-paused1.txt)
- [ax-timer-paused2.txt](docs/fork/evidence/native-audio-2026-09-19/ax-timer-paused2.txt)
- [ax-timer-recording0.txt](docs/fork/evidence/native-audio-2026-09-19/ax-timer-recording0.txt)
- [ax-timer-recording1.txt](docs/fork/evidence/native-audio-2026-09-19/ax-timer-recording1.txt)
- [ax-timer-resumed.txt](docs/fork/evidence/native-audio-2026-09-19/ax-timer-resumed.txt)
- [before-record.png](docs/fork/evidence/native-audio-2026-09-19/before-record.png)
- [capture-warning-cleanup.json](docs/fork/evidence/native-audio-2026-09-19/capture-warning-cleanup.json)
- [check-concat-markers.py](docs/fork/evidence/native-audio-2026-09-19/check-concat-markers.py)
- [check-concat-pcm.py](docs/fork/evidence/native-audio-2026-09-19/check-concat-pcm.py)
- [checkpoint-fixed-provenance.json](docs/fork/evidence/native-audio-2026-09-19/checkpoint-fixed-provenance.json)
- [checkpoint-sha256.json](docs/fork/evidence/native-audio-2026-09-19/checkpoint-sha256.json)
- [clippy-dead-test-cleanup.json](docs/fork/evidence/native-audio-2026-09-19/clippy-dead-test-cleanup.json)
- [continuation-sha256.json](docs/fork/evidence/native-audio-2026-09-19/continuation-sha256.json)
- [cycle2-artifacts.json](docs/fork/evidence/native-audio-2026-09-19/cycle2-artifacts.json)
- [cycle2-levels.json](docs/fork/evidence/native-audio-2026-09-19/cycle2-levels.json)
- [cycle2-pause-held.txt](docs/fork/evidence/native-audio-2026-09-19/cycle2-pause-held.txt)
- [cycle2-paused.png](docs/fork/evidence/native-audio-2026-09-19/cycle2-paused.png)
- [cycle2-paused.txt](docs/fork/evidence/native-audio-2026-09-19/cycle2-paused.txt)
- [cycle2-resumed.txt](docs/fork/evidence/native-audio-2026-09-19/cycle2-resumed.txt)
- [cycle2-start.txt](docs/fork/evidence/native-audio-2026-09-19/cycle2-start.txt)
- [cycle2-stop-action.txt](docs/fork/evidence/native-audio-2026-09-19/cycle2-stop-action.txt)
- [cycle2-stopped.png](docs/fork/evidence/native-audio-2026-09-19/cycle2-stopped.png)
- [cycle2-stopped.txt](docs/fork/evidence/native-audio-2026-09-19/cycle2-stopped.txt)
- [cycle3-minute.txt](docs/fork/evidence/native-audio-2026-09-19/cycle3-minute.txt)
- [cycle3-running.png](docs/fork/evidence/native-audio-2026-09-19/cycle3-running.png)
- [cycle3-running.txt](docs/fork/evidence/native-audio-2026-09-19/cycle3-running.txt)
- [cycle3-start.txt](docs/fork/evidence/native-audio-2026-09-19/cycle3-start.txt)
- [denied-and-dts-sha256.json](docs/fork/evidence/native-audio-2026-09-19/denied-and-dts-sha256.json)
- [dts-audit-concat.json](docs/fork/evidence/native-audio-2026-09-19/dts-audit-concat.json)
- [dts-audit-results.json](docs/fork/evidence/native-audio-2026-09-19/dts-audit-results.json)
- [dts-controlled-markers.json](docs/fork/evidence/native-audio-2026-09-19/dts-controlled-markers.json)
- [dts-controlled-result.json](docs/fork/evidence/native-audio-2026-09-19/dts-controlled-result.json)
- [dts-pcm-prototype-result.json](docs/fork/evidence/native-audio-2026-09-19/dts-pcm-prototype-result.json)
- [frontend-and-pcm-prototype-sha256.json](docs/fork/evidence/native-audio-2026-09-19/frontend-and-pcm-prototype-sha256.json)
- [journal-isolation-edit.json](docs/fork/evidence/native-audio-2026-09-19/journal-isolation-edit.json)
- [journal-isolation.patch](docs/fork/evidence/native-audio-2026-09-19/journal-isolation.patch)
- [launchservices-permission-timing-sha256.json](docs/fork/evidence/native-audio-2026-09-19/launchservices-permission-timing-sha256.json)
- [live-duration-checks-sha256.json](docs/fork/evidence/native-audio-2026-09-19/live-duration-checks-sha256.json)
- [live-duration-provenance.json](docs/fork/evidence/native-audio-2026-09-19/live-duration-provenance.json)
- [mechanical-format-lint.json](docs/fork/evidence/native-audio-2026-09-19/mechanical-format-lint.json)
- [mechanical-lint-small.json](docs/fork/evidence/native-audio-2026-09-19/mechanical-lint-small.json)
- [migration-schema-20260920.json](docs/fork/evidence/native-audio-2026-09-19/migration-schema-20260920.json)
- [msrv-macos-tests-result.json](docs/fork/evidence/native-audio-2026-09-19/msrv-macos-tests-result.json)
- [native-long-recording.json](docs/fork/evidence/native-audio-2026-09-19/native-long-recording.json)
- [native-long-sha256.json](docs/fork/evidence/native-audio-2026-09-19/native-long-sha256.json)
- [native-permission-exit-result.json](docs/fork/evidence/native-audio-2026-09-19/native-permission-exit-result.json)
- [native-permission-exit-sha256.json](docs/fork/evidence/native-audio-2026-09-19/native-permission-exit-sha256.json)
- [native-quit-failure.json](docs/fork/evidence/native-audio-2026-09-19/native-quit-failure.json)
- [native-recovery-sha256.json](docs/fork/evidence/native-audio-2026-09-19/native-recovery-sha256.json)
- [native-timer-recording.json](docs/fork/evidence/native-audio-2026-09-19/native-timer-recording.json)
- [native-timer-sha256.json](docs/fork/evidence/native-audio-2026-09-19/native-timer-sha256.json)
- [native-timing-markers.json](docs/fork/evidence/native-audio-2026-09-19/native-timing-markers.json)
- [partial-manifest-first-review-sha256.json](docs/fork/evidence/native-audio-2026-09-19/partial-manifest-first-review-sha256.json)
- [partial-manifest-round2-sha256.json](docs/fork/evidence/native-audio-2026-09-19/partial-manifest-round2-sha256.json)
- [permission-exit-build-sha256.json](docs/fork/evidence/native-audio-2026-09-19/permission-exit-build-sha256.json)
- [permission-exit-checks-sha256.json](docs/fork/evidence/native-audio-2026-09-19/permission-exit-checks-sha256.json)
- [permission-exit-provenance.json](docs/fork/evidence/native-audio-2026-09-19/permission-exit-provenance.json)
- [pool-checkpoint-full-after.txt](docs/fork/evidence/native-audio-2026-09-19/pool-checkpoint-full-after.txt)
- [pool-checkpoint-full-before.txt](docs/fork/evidence/native-audio-2026-09-19/pool-checkpoint-full-before.txt)
- [pool-checkpoint-full-unsandboxed-after.txt](docs/fork/evidence/native-audio-2026-09-19/pool-checkpoint-full-unsandboxed-after.txt)
- [pool-compilation-checkpoint-sha256.json](docs/fork/evidence/native-audio-2026-09-19/pool-compilation-checkpoint-sha256.json)
- [pool-consult-fingerprint.txt](docs/fork/evidence/native-audio-2026-09-19/pool-consult-fingerprint.txt)
- [pool-full-workspace-sha256.json](docs/fork/evidence/native-audio-2026-09-19/pool-full-workspace-sha256.json)
- [power-after-display-never.txt](docs/fork/evidence/native-audio-2026-09-19/power-after-display-never.txt)
- [power-display-reopened.txt](docs/fork/evidence/native-audio-2026-09-19/power-display-reopened.txt)
- [power-lockscreen-settings.txt](docs/fork/evidence/native-audio-2026-09-19/power-lockscreen-settings.txt)
- [pre-terminal-writer-checks-sha256.json](docs/fork/evidence/native-audio-2026-09-19/pre-terminal-writer-checks-sha256.json)
- [preflight-cancel-result.json](docs/fork/evidence/native-audio-2026-09-19/preflight-cancel-result.json)
- [ready-active.png](docs/fork/evidence/native-audio-2026-09-19/ready-active.png)
- [recording-controls-checks-sha256.json](docs/fork/evidence/native-audio-2026-09-19/recording-controls-checks-sha256.json)
- [recovery-native-after.json](docs/fork/evidence/native-audio-2026-09-19/recovery-native-after.json)
- [recovery-native-before.json](docs/fork/evidence/native-audio-2026-09-19/recovery-native-before.json)
- [recovery-sha256.json](docs/fork/evidence/native-audio-2026-09-19/recovery-sha256.json)
- [round2-denied-end.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-denied-end.ax.txt)
- [round2-denied-recording.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-denied-recording.ax.txt)
- [round2-denied-result.json](docs/fork/evidence/native-audio-2026-09-19/round2-denied-result.json)
- [round2-denied-settings.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-denied-settings.ax.txt)
- [round2-denied-sha256.json](docs/fork/evidence/native-audio-2026-09-19/round2-denied-sha256.json)
- [round2-identity-observation.json](docs/fork/evidence/native-audio-2026-09-19/round2-identity-observation.json)
- [round2-native-granted.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-granted.ax.txt)
- [round2-native-initial.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-initial.ax.txt)
- [round2-native-paused1.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-paused1.ax.txt)
- [round2-native-paused2.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-paused2.ax.txt)
- [round2-native-provenance.json](docs/fork/evidence/native-audio-2026-09-19/round2-native-provenance.json)
- [round2-native-recording.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-recording.ax.txt)
- [round2-native-result.json](docs/fork/evidence/native-audio-2026-09-19/round2-native-result.json)
- [round2-native-resumed.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-resumed.ax.txt)
- [round2-native-sha256.json](docs/fork/evidence/native-audio-2026-09-19/round2-native-sha256.json)
- [round2-native-stopped.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-native-stopped.ax.txt)
- [round2-restore-latest.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-restore-latest.ax.txt)
- [round2-restored-settings.ax.txt](docs/fork/evidence/native-audio-2026-09-19/round2-restored-settings.ax.txt)
- [settings-restart-legacy-crash.json](docs/fork/evidence/native-audio-2026-09-19/settings-restart-legacy-crash.json)
- [sha256.json](docs/fork/evidence/native-audio-2026-09-19/sha256.json)
- [stop-tail-sha256.json](docs/fork/evidence/native-audio-2026-09-19/stop-tail-sha256.json)
- [system-after-denial-new.txt](docs/fork/evidence/native-audio-2026-09-19/system-after-denial-new.txt)
- [system-before-denial-new.txt](docs/fork/evidence/native-audio-2026-09-19/system-before-denial-new.txt)
- [system-denied-app-initial.txt](docs/fork/evidence/native-audio-2026-09-19/system-denied-app-initial.txt)
- [system-denied-native-result.json](docs/fork/evidence/native-audio-2026-09-19/system-denied-native-result.json)
- [system-denied-start-settled.txt](docs/fork/evidence/native-audio-2026-09-19/system-denied-start-settled.txt)
- [system-unconfirmed-checks-sha256.json](docs/fork/evidence/native-audio-2026-09-19/system-unconfirmed-checks-sha256.json)
- [system-unconfirmed-edit.json](docs/fork/evidence/native-audio-2026-09-19/system-unconfirmed-edit.json)
- [transport-checkpoint-after.txt](docs/fork/evidence/native-audio-2026-09-19/transport-checkpoint-after.txt)
- [transport-checkpoint-before.txt](docs/fork/evidence/native-audio-2026-09-19/transport-checkpoint-before.txt)
- [transport-review-and-awake-sha256.json](docs/fork/evidence/native-audio-2026-09-19/transport-review-and-awake-sha256.json)
- [ui-error-native-result.json](docs/fork/evidence/native-audio-2026-09-19/ui-error-native-result.json)
- [ui-error-native-sha256.json](docs/fork/evidence/native-audio-2026-09-19/ui-error-native-sha256.json)
- [ui-fix-clippy-result.json](docs/fork/evidence/native-audio-2026-09-19/ui-fix-clippy-result.json)
- [ui-fix-full-check2-result.json](docs/fork/evidence/native-audio-2026-09-19/ui-fix-full-check2-result.json)
- [ui-new-denied-start.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-denied-start.txt)
- [ui-new-mic-denied.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-mic-denied.txt)
- [ui-new-paused-settled.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-paused-settled.txt)
- [ui-new-paused.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-paused.txt)
- [ui-new-resumed.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-resumed.txt)
- [ui-new-system-accepted.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-system-accepted.txt)
- [ui-new-system-pending.txt](docs/fork/evidence/native-audio-2026-09-19/ui-new-system-pending.txt)
- [verified-record-start.png](docs/fork/evidence/native-audio-2026-09-19/verified-record-start.png)

</details>

<details>
<summary>docs/fork/migration-snapshot — 10</summary>

- [branches.tsv](docs/fork/migration-snapshot/branches.tsv)
- [evidence.tsv](docs/fork/migration-snapshot/evidence.tsv)
- [gaps.tsv](docs/fork/migration-snapshot/gaps.tsv)
- [prs.tsv](docs/fork/migration-snapshot/prs.tsv)
- [session-claims.tsv](docs/fork/migration-snapshot/session-claims.tsv)
- [session-discovery.tsv](docs/fork/migration-snapshot/session-discovery.tsv)
- [sessions.tsv](docs/fork/migration-snapshot/sessions.tsv)
- [snapshot.tsv](docs/fork/migration-snapshot/snapshot.tsv)
- [worktrees-after.tsv](docs/fork/migration-snapshot/worktrees-after.tsv)
- [worktrees-before.tsv](docs/fork/migration-snapshot/worktrees-before.tsv)

</details>

<details>
<summary>docs/migration — 3</summary>

- [diarization-backend-foundation.md](docs/migration/diarization-backend-foundation.md)
- [interface-polish-visual-evidence.md](docs/migration/interface-polish-visual-evidence.md)
- [legacy-feature-registry.md](docs/migration/legacy-feature-registry.md)

</details>

<details>
<summary>docs/plans — 1</summary>

- [README.md](docs/plans/README.md)

</details>

<details>
<summary>docs/plans/macos-ci — 2</summary>

- [2026-10-02-macos-ci-census.json](docs/plans/macos-ci/2026-10-02-macos-ci-census.json)
- [2026-10-02-macos-ci-release.md](docs/plans/macos-ci/2026-10-02-macos-ci-release.md)

</details>

<details>
<summary>docs/plans/meeting-questions — 1</summary>

- [2026-10-02-meeting-questions.md](docs/plans/meeting-questions/2026-10-02-meeting-questions.md)

</details>

<details>
<summary>docs/plans/recording-storage — 2</summary>

- [2026-10-02-recording-storage-census.json](docs/plans/recording-storage/2026-10-02-recording-storage-census.json)
- [2026-10-02-recording-storage.md](docs/plans/recording-storage/2026-10-02-recording-storage.md)

</details>

<details>
<summary>docs/plans/theme — 8</summary>

- [2026-10-02-semantic-theme.md](docs/plans/theme/2026-10-02-semantic-theme.md)
- [2026-10-02-theme-census.json](docs/plans/theme/2026-10-02-theme-census.json)
- [2026-10-04-theme-background-proof-v2.md](docs/plans/theme/2026-10-04-theme-background-proof-v2.md)
- [2026-10-04-theme-background-proof-v3.md](docs/plans/theme/2026-10-04-theme-background-proof-v3.md)
- [2026-10-04-theme-background-proof-v4.md](docs/plans/theme/2026-10-04-theme-background-proof-v4.md)
- [2026-10-04-theme-background-proof.md](docs/plans/theme/2026-10-04-theme-background-proof.md)
- [2026-10-04-theme-e2e-locators.md](docs/plans/theme/2026-10-04-theme-e2e-locators.md)
- [2026-10-04-theme-sidebar-snapshots.md](docs/plans/theme/2026-10-04-theme-sidebar-snapshots.md)

</details>

<details>
<summary>docs/plans/webview-migration — 10</summary>

- [2026-10-02-webview-migration-census.json](docs/plans/webview-migration/2026-10-02-webview-migration-census.json)
- [2026-10-02-webview-migration-integration.md](docs/plans/webview-migration/2026-10-02-webview-migration-integration.md)
- [2026-10-04-webview-fresh-persistence-amendment-v2.md](docs/plans/webview-migration/2026-10-04-webview-fresh-persistence-amendment-v2.md)
- [2026-10-04-webview-fresh-persistence-amendment.md](docs/plans/webview-migration/2026-10-04-webview-fresh-persistence-amendment.md)
- [2026-10-04-webview-recovery-contract-v2.md](docs/plans/webview-migration/2026-10-04-webview-recovery-contract-v2.md)
- [2026-10-04-webview-recovery-contract-v3.md](docs/plans/webview-migration/2026-10-04-webview-recovery-contract-v3.md)
- [2026-10-04-webview-recovery-contract.md](docs/plans/webview-migration/2026-10-04-webview-recovery-contract.md)
- [2026-10-04-webview-recovery-transport.md](docs/plans/webview-migration/2026-10-04-webview-recovery-transport.md)
- [2026-10-04-webview-schema-ownership-v2.md](docs/plans/webview-migration/2026-10-04-webview-schema-ownership-v2.md)
- [2026-10-04-webview-schema-ownership.md](docs/plans/webview-migration/2026-10-04-webview-schema-ownership.md)

</details>

<details>
<summary>docs/prd — 2</summary>

- [fork-features.md](docs/prd/fork-features.md)
- [macos-fork.md](docs/prd/macos-fork.md)

</details>

<details>
<summary>docs/release — 2</summary>

- [smoke-report-template.md](docs/release/smoke-report-template.md)
- [test-plan-smoke-report-template.md](docs/release/test-plan-smoke-report-template.md)

</details>

<details>
<summary>docs/reviews — 1</summary>

- [README.md](docs/reviews/README.md)

</details>

<details>
<summary>docs/reviews/cross-cutting — 5</summary>

- [ci-docs-fix-2026-10-27.md](docs/reviews/cross-cutting/ci-docs-fix-2026-10-27.md)
- [completed-work-audit-2026-10-25.md](docs/reviews/cross-cutting/completed-work-audit-2026-10-25.md)
- [development-resume-notes-2026-10-26.md](docs/reviews/cross-cutting/development-resume-notes-2026-10-26.md)
- [plan-review-2026-10-02.md](docs/reviews/cross-cutting/plan-review-2026-10-02.md)
- [publication-privacy-audit-2026-10-27.md](docs/reviews/cross-cutting/publication-privacy-audit-2026-10-27.md)

</details>

<details>
<summary>docs/reviews/macos-ci — 4</summary>

- [macos-ci-native-acquisition-quality-review-2026-10-04.md](docs/reviews/macos-ci/macos-ci-native-acquisition-quality-review-2026-10-04.md)
- [macos-ci-native-acquisition-spec-review-2026-10-04.md](docs/reviews/macos-ci/macos-ci-native-acquisition-spec-review-2026-10-04.md)
- [macos-ci-prerequisite-quality-review-2026-10-03.md](docs/reviews/macos-ci/macos-ci-prerequisite-quality-review-2026-10-03.md)
- [macos-ci-prerequisite-spec-review-2026-10-03.md](docs/reviews/macos-ci/macos-ci-prerequisite-spec-review-2026-10-03.md)

</details>

<details>
<summary>docs/reviews/meeting-questions — 9</summary>

- [meeting-context-review-2026-10-02.md](docs/reviews/meeting-questions/meeting-context-review-2026-10-02.md)
- [meeting-questions-cold-cancel-quality-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-cold-cancel-quality-review-2026-10-03.md)
- [meeting-questions-cold-cancel-spec-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-cold-cancel-spec-review-2026-10-03.md)
- [meeting-questions-local-model-quality-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-local-model-quality-review-2026-10-03.md)
- [meeting-questions-local-model-spec-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-local-model-spec-review-2026-10-03.md)
- [meeting-questions-local-test-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-local-test-review-2026-10-03.md)
- [meeting-questions-plan-review-2026-10-02.md](docs/reviews/meeting-questions/meeting-questions-plan-review-2026-10-02.md)
- [meeting-questions-quality-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-quality-review-2026-10-03.md)
- [meeting-questions-spec-review-2026-10-03.md](docs/reviews/meeting-questions/meeting-questions-spec-review-2026-10-03.md)

</details>

<details>
<summary>docs/reviews/theme — 70</summary>

- [theme-admission-quality-review-2026-10-03.md](docs/reviews/theme/theme-admission-quality-review-2026-10-03.md)
- [theme-admission-spec-review-2026-10-03.md](docs/reviews/theme/theme-admission-spec-review-2026-10-03.md)
- [theme-background-proof-final-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-final-quality-review-2026-10-04.md)
- [theme-background-proof-final-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-final-spec-review-2026-10-04.md)
- [theme-background-proof-implementation-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-implementation-quality-review-2026-10-04.md)
- [theme-background-proof-implementation-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-implementation-spec-review-2026-10-04.md)
- [theme-background-proof-plan-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-quality-review-2026-10-04.md)
- [theme-background-proof-plan-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-spec-review-2026-10-04.md)
- [theme-background-proof-plan-v2-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-v2-quality-review-2026-10-04.md)
- [theme-background-proof-plan-v2-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-v2-spec-review-2026-10-04.md)
- [theme-background-proof-plan-v3-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-v3-quality-review-2026-10-04.md)
- [theme-background-proof-plan-v3-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-v3-spec-review-2026-10-04.md)
- [theme-background-proof-plan-v4-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-v4-quality-review-2026-10-04.md)
- [theme-background-proof-plan-v4-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-plan-v4-spec-review-2026-10-04.md)
- [theme-background-proof-review-fix-quality-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-review-fix-quality-review-2026-10-04.md)
- [theme-background-proof-review-fix-spec-review-2026-10-04.md](docs/reviews/theme/theme-background-proof-review-fix-spec-review-2026-10-04.md)
- [theme-clean-admission-quality-review-2026-10-03.md](docs/reviews/theme/theme-clean-admission-quality-review-2026-10-03.md)
- [theme-clean-admission-spec-review-2026-10-03.md](docs/reviews/theme/theme-clean-admission-spec-review-2026-10-03.md)
- [theme-contrast-helper-amendment-quality-review-2026-10-03.md](docs/reviews/theme/theme-contrast-helper-amendment-quality-review-2026-10-03.md)
- [theme-contrast-helper-amendment-spec-review-2026-10-03.md](docs/reviews/theme/theme-contrast-helper-amendment-spec-review-2026-10-03.md)
- [theme-diagnostic-v2-quality-review-2026-10-03.md](docs/reviews/theme/theme-diagnostic-v2-quality-review-2026-10-03.md)
- [theme-e2e-locators-implementation-quality-review-2026-10-04.md](docs/reviews/theme/theme-e2e-locators-implementation-quality-review-2026-10-04.md)
- [theme-e2e-locators-implementation-spec-review-2026-10-04.md](docs/reviews/theme/theme-e2e-locators-implementation-spec-review-2026-10-04.md)
- [theme-e2e-locators-plan-quality-review-2026-10-04.md](docs/reviews/theme/theme-e2e-locators-plan-quality-review-2026-10-04.md)
- [theme-e2e-locators-plan-spec-review-2026-10-04.md](docs/reviews/theme/theme-e2e-locators-plan-spec-review-2026-10-04.md)
- [theme-focus-gap-amendment-quality-review-2026-10-03.md](docs/reviews/theme/theme-focus-gap-amendment-quality-review-2026-10-03.md)
- [theme-focus-gap-amendment-spec-review-2026-10-03.md](docs/reviews/theme/theme-focus-gap-amendment-spec-review-2026-10-03.md)
- [theme-focus-geometry-amendment-quality-review-2026-10-03.md](docs/reviews/theme/theme-focus-geometry-amendment-quality-review-2026-10-03.md)
- [theme-focus-geometry-amendment-spec-review-2026-10-03.md](docs/reviews/theme/theme-focus-geometry-amendment-spec-review-2026-10-03.md)
- [theme-focus-implementation-quality-review-2026-10-03.md](docs/reviews/theme/theme-focus-implementation-quality-review-2026-10-03.md)
- [theme-focus-implementation-spec-review-2026-10-03.md](docs/reviews/theme/theme-focus-implementation-spec-review-2026-10-03.md)
- [theme-focus-review-fix-quality-review-2026-10-03.md](docs/reviews/theme/theme-focus-review-fix-quality-review-2026-10-03.md)
- [theme-focus-review-fix-spec-review-2026-10-03.md](docs/reviews/theme/theme-focus-review-fix-spec-review-2026-10-03.md)
- [theme-matrix-red-quality-review-2026-10-03.md](docs/reviews/theme/theme-matrix-red-quality-review-2026-10-03.md)
- [theme-matrix-red-spec-review-2026-10-03.md](docs/reviews/theme/theme-matrix-red-spec-review-2026-10-03.md)
- [theme-oracle-round3-quality-review-2026-10-03.md](docs/reviews/theme/theme-oracle-round3-quality-review-2026-10-03.md)
- [theme-oracle-round3-spec-review-2026-10-03.md](docs/reviews/theme/theme-oracle-round3-spec-review-2026-10-03.md)
- [theme-persistence-final-quality-review-2026-10-03.md](docs/reviews/theme/theme-persistence-final-quality-review-2026-10-03.md)
- [theme-persistence-final-spec-review-2026-10-03.md](docs/reviews/theme/theme-persistence-final-spec-review-2026-10-03.md)
- [theme-policy-fixtures-quality-review-2026-10-03.md](docs/reviews/theme/theme-policy-fixtures-quality-review-2026-10-03.md)
- [theme-policy-fixtures-spec-review-2026-10-03.md](docs/reviews/theme/theme-policy-fixtures-spec-review-2026-10-03.md)
- [theme-post-token-quality-review-2026-10-03.md](docs/reviews/theme/theme-post-token-quality-review-2026-10-03.md)
- [theme-post-token-spec-review-2026-10-03.md](docs/reviews/theme/theme-post-token-spec-review-2026-10-03.md)
- [theme-prepaint-adapter-quality-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-adapter-quality-review-2026-10-03.md)
- [theme-prepaint-adapter-spec-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-adapter-spec-review-2026-10-03.md)
- [theme-prepaint-amendment-quality-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-amendment-quality-review-2026-10-03.md)
- [theme-prepaint-hydration-amendment-quality-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-hydration-amendment-quality-review-2026-10-03.md)
- [theme-prepaint-hydration-amendment-spec-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-hydration-amendment-spec-review-2026-10-03.md)
- [theme-prepaint-implementation-quality-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-implementation-quality-review-2026-10-03.md)
- [theme-prepaint-implementation-spec-review-2026-10-03.md](docs/reviews/theme/theme-prepaint-implementation-spec-review-2026-10-03.md)
- [theme-rendered-contrast-quality-review-2026-10-03.md](docs/reviews/theme/theme-rendered-contrast-quality-review-2026-10-03.md)
- [theme-rendered-contrast-round3-quality-review-2026-10-03.md](docs/reviews/theme/theme-rendered-contrast-round3-quality-review-2026-10-03.md)
- [theme-rendered-contrast-round3-spec-review-2026-10-03.md](docs/reviews/theme/theme-rendered-contrast-round3-spec-review-2026-10-03.md)
- [theme-rendered-contrast-round4-quality-review-2026-10-03.md](docs/reviews/theme/theme-rendered-contrast-round4-quality-review-2026-10-03.md)
- [theme-rendered-contrast-round4-spec-review-2026-10-03.md](docs/reviews/theme/theme-rendered-contrast-round4-spec-review-2026-10-03.md)
- [theme-rendered-contrast-spec-review-2026-10-03.md](docs/reviews/theme/theme-rendered-contrast-spec-review-2026-10-03.md)
- [theme-shadow-proof-amendment-quality-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-amendment-quality-review-2026-10-03.md)
- [theme-shadow-proof-amendment-spec-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-amendment-spec-review-2026-10-03.md)
- [theme-shadow-proof-amendment-v2-quality-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-amendment-v2-quality-review-2026-10-03.md)
- [theme-shadow-proof-amendment-v2-spec-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-amendment-v2-spec-review-2026-10-03.md)
- [theme-shadow-proof-atomic-quality-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-atomic-quality-review-2026-10-03.md)
- [theme-shadow-proof-atomic-spec-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-atomic-spec-review-2026-10-03.md)
- [theme-shadow-proof-implementation-quality-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-implementation-quality-review-2026-10-03.md)
- [theme-shadow-proof-implementation-spec-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-implementation-spec-review-2026-10-03.md)
- [theme-shadow-proof-review-fix-quality-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-review-fix-quality-review-2026-10-03.md)
- [theme-shadow-proof-review-fix-spec-review-2026-10-03.md](docs/reviews/theme/theme-shadow-proof-review-fix-spec-review-2026-10-03.md)
- [theme-sidebar-snapshot-implementation-quality-review-2026-10-04.md](docs/reviews/theme/theme-sidebar-snapshot-implementation-quality-review-2026-10-04.md)
- [theme-sidebar-snapshot-implementation-spec-review-2026-10-04.md](docs/reviews/theme/theme-sidebar-snapshot-implementation-spec-review-2026-10-04.md)
- [theme-sidebar-snapshot-plan-quality-review-2026-10-04.md](docs/reviews/theme/theme-sidebar-snapshot-plan-quality-review-2026-10-04.md)
- [theme-sidebar-snapshot-plan-spec-review-2026-10-04.md](docs/reviews/theme/theme-sidebar-snapshot-plan-spec-review-2026-10-04.md)

</details>

<details>
<summary>docs/reviews/webview-migration — 19</summary>

- [webview-fresh-persistence-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-fresh-persistence-quality-review-2026-10-04.md)
- [webview-fresh-persistence-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-fresh-persistence-spec-review-2026-10-04.md)
- [webview-fresh-persistence-v2-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-fresh-persistence-v2-quality-review-2026-10-04.md)
- [webview-fresh-persistence-v2-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-fresh-persistence-v2-spec-review-2026-10-04.md)
- [webview-mixed-id-fix-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-mixed-id-fix-quality-review-2026-10-04.md)
- [webview-mixed-id-fix-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-mixed-id-fix-spec-review-2026-10-04.md)
- [webview-mixed-id-fix-v2-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-mixed-id-fix-v2-quality-review-2026-10-04.md)
- [webview-mixed-id-fix-v2-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-mixed-id-fix-v2-spec-review-2026-10-04.md)
- [webview-recovery-contract-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-contract-quality-review-2026-10-04.md)
- [webview-recovery-contract-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-contract-spec-review-2026-10-04.md)
- [webview-recovery-contract-v2-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-contract-v2-quality-review-2026-10-04.md)
- [webview-recovery-contract-v2-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-contract-v2-spec-review-2026-10-04.md)
- [webview-recovery-contract-v3-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-contract-v3-quality-review-2026-10-04.md)
- [webview-recovery-contract-v3-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-contract-v3-spec-review-2026-10-04.md)
- [webview-recovery-transport-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-recovery-transport-spec-review-2026-10-04.md)
- [webview-schema-ownership-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-schema-ownership-quality-review-2026-10-04.md)
- [webview-schema-ownership-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-schema-ownership-spec-review-2026-10-04.md)
- [webview-schema-ownership-v2-quality-review-2026-10-04.md](docs/reviews/webview-migration/webview-schema-ownership-v2-quality-review-2026-10-04.md)
- [webview-schema-ownership-v2-spec-review-2026-10-04.md](docs/reviews/webview-migration/webview-schema-ownership-v2-spec-review-2026-10-04.md)

</details>

<details>
<summary>docs/superpowers — 1</summary>

- [README.md](docs/superpowers/README.md)

</details>

<details>
<summary>docs/superpowers/plans — 13</summary>

- [2026-06-20-full-app-theme.md](docs/superpowers/plans/2026-06-20-full-app-theme.md)
- [2026-06-26-local-gemma4-summary.md](docs/superpowers/plans/2026-06-26-local-gemma4-summary.md)
- [2026-06-26-speaker-diarization-foundation.md](docs/superpowers/plans/2026-06-26-speaker-diarization-foundation.md)
- [2026-07-11-interface-polish.md](docs/superpowers/plans/2026-07-11-interface-polish.md)
- [2026-07-11-port-dark-theme-to-universal.md](docs/superpowers/plans/2026-07-11-port-dark-theme-to-universal.md)
- [2026-07-12-meetily-git-recovery-foundation.md](docs/superpowers/plans/2026-07-12-meetily-git-recovery-foundation.md)
- [2026-07-13-hidden-diarization-backend-foundation.md](docs/superpowers/plans/2026-07-13-hidden-diarization-backend-foundation.md)
- [2026-07-14-testing-release-gate.md](docs/superpowers/plans/2026-07-14-testing-release-gate.md)
- [2026-07-26-meeting-export-ui.md](docs/superpowers/plans/2026-07-26-meeting-export-ui.md)
- [2026-07-26-post-call-diarization-runtime.md](docs/superpowers/plans/2026-07-26-post-call-diarization-runtime.md)
- [2026-07-26-release-ready-qualification.md](docs/superpowers/plans/2026-07-26-release-ready-qualification.md)
- [2026-07-26-speaker-labels-rename.md](docs/superpowers/plans/2026-07-26-speaker-labels-rename.md)
- [2026-08-15-fork-051-macos-permissions-lifecycle.md](docs/superpowers/plans/2026-08-15-fork-051-macos-permissions-lifecycle.md)

</details>

<details>
<summary>docs/superpowers/specs — 11</summary>

- [2026-06-20-full-app-theme-design.md](docs/superpowers/specs/2026-06-20-full-app-theme-design.md)
- [2026-06-26-local-gemma4-summary-design.md](docs/superpowers/specs/2026-06-26-local-gemma4-summary-design.md)
- [2026-06-26-speaker-diarization-design.md](docs/superpowers/specs/2026-06-26-speaker-diarization-design.md)
- [2026-07-11-interface-polish-design.md](docs/superpowers/specs/2026-07-11-interface-polish-design.md)
- [2026-07-11-universal-macos-stable-build-design.md](docs/superpowers/specs/2026-07-11-universal-macos-stable-build-design.md)
- [2026-07-12-meetily-dual-track-git-workflow-design.md](docs/superpowers/specs/2026-07-12-meetily-dual-track-git-workflow-design.md)
- [2026-07-14-testing-release-gate-design.md](docs/superpowers/specs/2026-07-14-testing-release-gate-design.md)
- [2026-07-26-meeting-export-ui-design.md](docs/superpowers/specs/2026-07-26-meeting-export-ui-design.md)
- [2026-07-26-post-call-diarization-runtime-design.md](docs/superpowers/specs/2026-07-26-post-call-diarization-runtime-design.md)
- [2026-07-26-speaker-labels-rename-design.md](docs/superpowers/specs/2026-07-26-speaker-labels-rename-design.md)
- [2026-08-15-fork-051-macos-permissions-lifecycle-design.md](docs/superpowers/specs/2026-08-15-fork-051-macos-permissions-lifecycle-design.md)

</details>

<details>
<summary>frontend — 19</summary>

- [.gitignore](frontend/.gitignore)
- [API.md](frontend/API.md)
- [README.md](frontend/README.md)
- [clean_build.sh](frontend/clean_build.sh)
- [clean_run.sh](frontend/clean_run.sh)
- [components.json](frontend/components.json)
- [eslint.config.mjs](frontend/eslint.config.mjs)
- [next.config.js](frontend/next.config.js)
- [package-app.sh](frontend/package-app.sh)
- [package.json](frontend/package.json)
- [playwright.config.ts](frontend/playwright.config.ts)
- [playwright.static.config.ts](frontend/playwright.static.config.ts)
- [pnpm-lock.yaml](frontend/pnpm-lock.yaml)
- [postcss.config.js](frontend/postcss.config.js)
- [postcss.config.mjs](frontend/postcss.config.mjs)
- [tailwind.config.js](frontend/tailwind.config.js)
- [tailwind.config.ts](frontend/tailwind.config.ts)
- [tsconfig.json](frontend/tsconfig.json)
- [vitest.config.ts](frontend/vitest.config.ts)

</details>

<details>
<summary>frontend/public — 9</summary>

- [file.svg](frontend/public/file.svg)
- [globe.svg](frontend/public/globe.svg)
- [icon_128x128.png](frontend/public/icon_128x128.png)
- [icon_32x32@2x.png](frontend/public/icon_32x32%402x.png)
- [logo-collapsed.png](frontend/public/logo-collapsed.png)
- [logo.png](frontend/public/logo.png)
- [next.svg](frontend/public/next.svg)
- [vercel.svg](frontend/public/vercel.svg)
- [window.svg](frontend/public/window.svg)

</details>

<details>
<summary>frontend/scripts — 13</summary>

- [build-universal-macos.js](frontend/scripts/build-universal-macos.js)
- [check-publication-privacy.mjs](frontend/scripts/check-publication-privacy.mjs)
- [check-theme-tokens.mjs](frontend/scripts/check-theme-tokens.mjs)
- [diarization-artifacts.js](frontend/scripts/diarization-artifacts.js)
- [macos-sidecar-tools.js](frontend/scripts/macos-sidecar-tools.js)
- [platform-surface.mjs](frontend/scripts/platform-surface.mjs)
- [provision-universal-artifacts.js](frontend/scripts/provision-universal-artifacts.js)
- [release-policy.mjs](frontend/scripts/release-policy.mjs)
- [source-fingerprint.mjs](frontend/scripts/source-fingerprint.mjs)
- [universal-onnx-runtime.js](frontend/scripts/universal-onnx-runtime.js)
- [verify-release-candidate.mjs](frontend/scripts/verify-release-candidate.mjs)
- [verify-smoke-report.mjs](frontend/scripts/verify-smoke-report.mjs)
- [verify-universal-ffmpeg.js](frontend/scripts/verify-universal-ffmpeg.js)

</details>

<details>
<summary>frontend/src-tauri — 11</summary>

- [.gitignore](frontend/src-tauri/.gitignore)
- [CLEANUP_PLAN.md](frontend/src-tauri/CLEANUP_PLAN.md)
- [Cargo.toml](frontend/src-tauri/Cargo.toml)
- [Info.plist](frontend/src-tauri/Info.plist)
- [LOGGING_OPTIMIZATIONS.md](frontend/src-tauri/LOGGING_OPTIMIZATIONS.md)
- [NOTIFICATION_TESTING.md](frontend/src-tauri/NOTIFICATION_TESTING.md)
- [build.rs](frontend/src-tauri/build.rs)
- [check_screen_permission.swift](frontend/src-tauri/check_screen_permission.swift)
- [entitlements.plist](frontend/src-tauri/entitlements.plist)
- [tauri.conf.json](frontend/src-tauri/tauri.conf.json)
- [tauri.universal.conf.json](frontend/src-tauri/tauri.universal.conf.json)

</details>

<details>
<summary>frontend/src-tauri/.cargo — 1</summary>

- [config.toml](frontend/src-tauri/.cargo/config.toml)

</details>

<details>
<summary>frontend/src-tauri/build — 2</summary>

- [ffmpeg.rs](frontend/src-tauri/build/ffmpeg.rs)
- [ffmpeg_verify.rs](frontend/src-tauri/build/ffmpeg_verify.rs)

</details>

<details>
<summary>frontend/src-tauri/icons — 16</summary>

- [128x128.png](frontend/src-tauri/icons/128x128.png)
- [128x128@2x.png](frontend/src-tauri/icons/128x128%402x.png)
- [32x32.png](frontend/src-tauri/icons/32x32.png)
- [app_icon.icns](frontend/src-tauri/icons/app_icon.icns)
- [icon.icns](frontend/src-tauri/icons/icon.icns)
- [icon.png](frontend/src-tauri/icons/icon.png)
- [icon_128x128.png](frontend/src-tauri/icons/icon_128x128.png)
- [icon_128x128@2x.png](frontend/src-tauri/icons/icon_128x128%402x.png)
- [icon_16x16.png](frontend/src-tauri/icons/icon_16x16.png)
- [icon_16x16@2x.png](frontend/src-tauri/icons/icon_16x16%402x.png)
- [icon_256x256.png](frontend/src-tauri/icons/icon_256x256.png)
- [icon_256x256@2x.png](frontend/src-tauri/icons/icon_256x256%402x.png)
- [icon_32x32.png](frontend/src-tauri/icons/icon_32x32.png)
- [icon_32x32@2x.png](frontend/src-tauri/icons/icon_32x32%402x.png)
- [icon_512x512.png](frontend/src-tauri/icons/icon_512x512.png)
- [icon_512x512@2x.png](frontend/src-tauri/icons/icon_512x512%402x.png)

</details>

<details>
<summary>frontend/src-tauri/migrations — 16</summary>

- [20250916100000_initial_schema.sql](frontend/src-tauri/migrations/20250916100000_initial_schema.sql)
- [20250920155811_add_openrouter_api_key.sql](frontend/src-tauri/migrations/20250920155811_add_openrouter_api_key.sql)
- [20251006000000_add_audio_sync_fields.sql](frontend/src-tauri/migrations/20251006000000_add_audio_sync_fields.sql)
- [20251010153942_add_ollama_endpoint.sql](frontend/src-tauri/migrations/20251010153942_add_ollama_endpoint.sql)
- [20251101000000_add_summary_backup.sql](frontend/src-tauri/migrations/20251101000000_add_summary_backup.sql)
- [20251105120000_add_pro_license_custom_openai.sql](frontend/src-tauri/migrations/20251105120000_add_pro_license_custom_openai.sql)
- [20251110000000_add_grace_period_to_licensing.sql](frontend/src-tauri/migrations/20251110000000_add_grace_period_to_licensing.sql)
- [20251110000001_add_speaker_field.sql](frontend/src-tauri/migrations/20251110000001_add_speaker_field.sql)
- [20251223000000_add_meeting_notes.sql](frontend/src-tauri/migrations/20251223000000_add_meeting_notes.sql)
- [20251229000000_add_gemini_api_key.sql](frontend/src-tauri/migrations/20251229000000_add_gemini_api_key.sql)
- [20260626000100_add_diarization_foundation.sql](frontend/src-tauri/migrations/20260626000100_add_diarization_foundation.sql)
- [20260713000100_harden_diarization_foundation.sql](frontend/src-tauri/migrations/20260713000100_harden_diarization_foundation.sql)
- [20260908000100_add_transcription_vocabulary.sql](frontend/src-tauri/migrations/20260908000100_add_transcription_vocabulary.sql)
- [20260908000200_add_meeting_provenance.sql](frontend/src-tauri/migrations/20260908000200_add_meeting_provenance.sql)
- [20260913000100_add_meeting_lifecycle.sql](frontend/src-tauri/migrations/20260913000100_add_meeting_lifecycle.sql)
- [20260926000000_add_meeting_search_index.sql](frontend/src-tauri/migrations/20260926000000_add_meeting_search_index.sql)

</details>

<details>
<summary>frontend/src-tauri/resources/diarization — 1</summary>

- [NOTICE.md](frontend/src-tauri/resources/diarization/NOTICE.md)

</details>

<details>
<summary>frontend/src-tauri/resources/diarization/licenses — 2</summary>

- [Apache-2.0.txt](frontend/src-tauri/resources/diarization/licenses/Apache-2.0.txt)
- [MIT-pyannote.txt](frontend/src-tauri/resources/diarization/licenses/MIT-pyannote.txt)

</details>

<details>
<summary>frontend/src-tauri/src — 10</summary>

- [config.rs](frontend/src-tauri/src/config.rs)
- [export.rs](frontend/src-tauri/src/export.rs)
- [lib.rs](frontend/src-tauri/src/lib.rs)
- [main.rs](frontend/src-tauri/src/main.rs)
- [meeting_questions.rs](frontend/src-tauri/src/meeting_questions.rs)
- [model_download.rs](frontend/src-tauri/src/model_download.rs)
- [onboarding.rs](frontend/src-tauri/src/onboarding.rs)
- [state.rs](frontend/src-tauri/src/state.rs)
- [tray.rs](frontend/src-tauri/src/tray.rs)
- [utils.rs](frontend/src-tauri/src/utils.rs)

</details>

<details>
<summary>frontend/src-tauri/src/anthropic — 2</summary>

- [anthropic.rs](frontend/src-tauri/src/anthropic/anthropic.rs)
- [mod.rs](frontend/src-tauri/src/anthropic/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/api — 3</summary>

- [api.rs](frontend/src-tauri/src/api/api.rs)
- [commands.rs](frontend/src-tauri/src/api/commands.rs)
- [mod.rs](frontend/src-tauri/src/api/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/audio — 34</summary>

- [async_logger.rs](frontend/src-tauri/src/audio/async_logger.rs)
- [audio_processing.rs](frontend/src-tauri/src/audio/audio_processing.rs)
- [batch_processor.rs](frontend/src-tauri/src/audio/batch_processor.rs)
- [buffer_pool.rs](frontend/src-tauri/src/audio/buffer_pool.rs)
- [common.rs](frontend/src-tauri/src/audio/common.rs)
- [constants.rs](frontend/src-tauri/src/audio/constants.rs)
- [decoder.rs](frontend/src-tauri/src/audio/decoder.rs)
- [device_detection.rs](frontend/src-tauri/src/audio/device_detection.rs)
- [device_monitor.rs](frontend/src-tauri/src/audio/device_monitor.rs)
- [diagnostics.rs](frontend/src-tauri/src/audio/diagnostics.rs)
- [encode.rs](frontend/src-tauri/src/audio/encode.rs)
- [ffmpeg.rs](frontend/src-tauri/src/audio/ffmpeg.rs)
- [ffmpeg_mixer.rs](frontend/src-tauri/src/audio/ffmpeg_mixer.rs)
- [hardware_detector.rs](frontend/src-tauri/src/audio/hardware_detector.rs)
- [import.rs](frontend/src-tauri/src/audio/import.rs)
- [incremental_saver.rs](frontend/src-tauri/src/audio/incremental_saver.rs)
- [level_monitor.rs](frontend/src-tauri/src/audio/level_monitor.rs)
- [mod.rs](frontend/src-tauri/src/audio/mod.rs)
- [permissions.rs](frontend/src-tauri/src/audio/permissions.rs)
- [pipeline.rs](frontend/src-tauri/src/audio/pipeline.rs)
- [playback_monitor.rs](frontend/src-tauri/src/audio/playback_monitor.rs)
- [post_processor.rs](frontend/src-tauri/src/audio/post_processor.rs)
- [recording_commands.rs](frontend/src-tauri/src/audio/recording_commands.rs)
- [recording_preferences.rs](frontend/src-tauri/src/audio/recording_preferences.rs)
- [recording_saver.rs](frontend/src-tauri/src/audio/recording_saver.rs)
- [recording_session.rs](frontend/src-tauri/src/audio/recording_session.rs)
- [recording_session_state.rs](frontend/src-tauri/src/audio/recording_session_state.rs)
- [recording_state.rs](frontend/src-tauri/src/audio/recording_state.rs)
- [retranscription.rs](frontend/src-tauri/src/audio/retranscription.rs)
- [simple_level_monitor.rs](frontend/src-tauri/src/audio/simple_level_monitor.rs)
- [stt.rs](frontend/src-tauri/src/audio/stt.rs)
- [system_audio_types.ts](frontend/src-tauri/src/audio/system_audio_types.ts)
- [system_detector.rs](frontend/src-tauri/src/audio/system_detector.rs)
- [vad.rs](frontend/src-tauri/src/audio/vad.rs)

</details>

<details>
<summary>frontend/src-tauri/src/audio/capture — 4</summary>

- [backend.rs](frontend/src-tauri/src/audio/capture/backend.rs)
- [core_audio.rs](frontend/src-tauri/src/audio/capture/core_audio.rs)
- [macos.rs](frontend/src-tauri/src/audio/capture/macos.rs)
- [mod.rs](frontend/src-tauri/src/audio/capture/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/audio/devices — 6</summary>

- [configuration.rs](frontend/src-tauri/src/audio/devices/configuration.rs)
- [discovery.rs](frontend/src-tauri/src/audio/devices/discovery.rs)
- [fallback.rs](frontend/src-tauri/src/audio/devices/fallback.rs)
- [microphone.rs](frontend/src-tauri/src/audio/devices/microphone.rs)
- [mod.rs](frontend/src-tauri/src/audio/devices/mod.rs)
- [speakers.rs](frontend/src-tauri/src/audio/devices/speakers.rs)

</details>

<details>
<summary>frontend/src-tauri/src/audio/devices/platform — 2</summary>

- [macos.rs](frontend/src-tauri/src/audio/devices/platform/macos.rs)
- [mod.rs](frontend/src-tauri/src/audio/devices/platform/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/audio/transcription — 6</summary>

- [engine.rs](frontend/src-tauri/src/audio/transcription/engine.rs)
- [mod.rs](frontend/src-tauri/src/audio/transcription/mod.rs)
- [parakeet_provider.rs](frontend/src-tauri/src/audio/transcription/parakeet_provider.rs)
- [provider.rs](frontend/src-tauri/src/audio/transcription/provider.rs)
- [whisper_provider.rs](frontend/src-tauri/src/audio/transcription/whisper_provider.rs)
- [worker.rs](frontend/src-tauri/src/audio/transcription/worker.rs)

</details>

<details>
<summary>frontend/src-tauri/src/console_utils — 3</summary>

- [commands.rs](frontend/src-tauri/src/console_utils/commands.rs)
- [console_utils.rs](frontend/src-tauri/src/console_utils/console_utils.rs)
- [mod.rs](frontend/src-tauri/src/console_utils/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/database — 8</summary>

- [commands.rs](frontend/src-tauri/src/database/commands.rs)
- [legacy_fastapi.rs](frontend/src-tauri/src/database/legacy_fastapi.rs)
- [manager.rs](frontend/src-tauri/src/database/manager.rs)
- [migration.rs](frontend/src-tauri/src/database/migration.rs)
- [migration_commands.rs](frontend/src-tauri/src/database/migration_commands.rs)
- [mod.rs](frontend/src-tauri/src/database/mod.rs)
- [models.rs](frontend/src-tauri/src/database/models.rs)
- [setup.rs](frontend/src-tauri/src/database/setup.rs)

</details>

<details>
<summary>frontend/src-tauri/src/database/repositories — 9</summary>

- [diarization.rs](frontend/src-tauri/src/database/repositories/diarization.rs)
- [meeting.rs](frontend/src-tauri/src/database/repositories/meeting.rs)
- [meeting_context.rs](frontend/src-tauri/src/database/repositories/meeting_context.rs)
- [mod.rs](frontend/src-tauri/src/database/repositories/mod.rs)
- [search.rs](frontend/src-tauri/src/database/repositories/search.rs)
- [setting.rs](frontend/src-tauri/src/database/repositories/setting.rs)
- [summary.rs](frontend/src-tauri/src/database/repositories/summary.rs)
- [transcript.rs](frontend/src-tauri/src/database/repositories/transcript.rs)
- [transcript_chunk.rs](frontend/src-tauri/src/database/repositories/transcript_chunk.rs)

</details>

<details>
<summary>frontend/src-tauri/src/diarization — 7</summary>

- [alignment.rs](frontend/src-tauri/src/diarization/alignment.rs)
- [audio.rs](frontend/src-tauri/src/diarization/audio.rs)
- [commands.rs](frontend/src-tauri/src/diarization/commands.rs)
- [coordinator.rs](frontend/src-tauri/src/diarization/coordinator.rs)
- [mod.rs](frontend/src-tauri/src/diarization/mod.rs)
- [runtime.rs](frontend/src-tauri/src/diarization/runtime.rs)
- [types.rs](frontend/src-tauri/src/diarization/types.rs)

</details>

<details>
<summary>frontend/src-tauri/src/groq — 2</summary>

- [groq.rs](frontend/src-tauri/src/groq/groq.rs)
- [mod.rs](frontend/src-tauri/src/groq/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/meeting_questions/tests — 1</summary>

- [mod.rs](frontend/src-tauri/src/meeting_questions/tests/mod.rs)

</details>

<details>
<summary>frontend/src-tauri/src/notifications — 6</summary>

- [commands.rs](frontend/src-tauri/src/notifications/commands.rs)
- [manager.rs](frontend/src-tauri/src/notifications/manager.rs)
- [mod.rs](frontend/src-tauri/src/notifications/mod.rs)
- [settings.rs](frontend/src-tauri/src/notifications/settings.rs)
- [system.rs](frontend/src-tauri/src/notifications/system.rs)
- [types.rs](frontend/src-tauri/src/notifications/types.rs)

</details>

<details>
<summary>frontend/src-tauri/src/ollama — 4</summary>

- [commands.rs](frontend/src-tauri/src/ollama/commands.rs)
- [metadata.rs](frontend/src-tauri/src/ollama/metadata.rs)
- [mod.rs](frontend/src-tauri/src/ollama/mod.rs)
- [ollama.rs](frontend/src-tauri/src/ollama/ollama.rs)

</details>

<details>
<summary>frontend/src-tauri/src/openai — 2</summary>

- [mod.rs](frontend/src-tauri/src/openai/mod.rs)
- [openai.rs](frontend/src-tauri/src/openai/openai.rs)

</details>

<details>
<summary>frontend/src-tauri/src/openrouter — 3</summary>

- [commands.rs](frontend/src-tauri/src/openrouter/commands.rs)
- [mod.rs](frontend/src-tauri/src/openrouter/mod.rs)
- [openrouter.rs](frontend/src-tauri/src/openrouter/openrouter.rs)

</details>

<details>
<summary>frontend/src-tauri/src/parakeet_engine — 4</summary>

- [commands.rs](frontend/src-tauri/src/parakeet_engine/commands.rs)
- [mod.rs](frontend/src-tauri/src/parakeet_engine/mod.rs)
- [model.rs](frontend/src-tauri/src/parakeet_engine/model.rs)
- [parakeet_engine.rs](frontend/src-tauri/src/parakeet_engine/parakeet_engine.rs)

</details>

<details>
<summary>frontend/src-tauri/src/summary — 10</summary>

- [commands.rs](frontend/src-tauri/src/summary/commands.rs)
- [language_detection.rs](frontend/src-tauri/src/summary/language_detection.rs)
- [llm_client.rs](frontend/src-tauri/src/summary/llm_client.rs)
- [metadata.rs](frontend/src-tauri/src/summary/metadata.rs)
- [mod.rs](frontend/src-tauri/src/summary/mod.rs)
- [persistence.rs](frontend/src-tauri/src/summary/persistence.rs)
- [processor.rs](frontend/src-tauri/src/summary/processor.rs)
- [service.rs](frontend/src-tauri/src/summary/service.rs)
- [template_commands.rs](frontend/src-tauri/src/summary/template_commands.rs)
- [template_selection.rs](frontend/src-tauri/src/summary/template_selection.rs)

</details>

<details>
<summary>frontend/src-tauri/src/summary/summary_engine — 6</summary>

- [client.rs](frontend/src-tauri/src/summary/summary_engine/client.rs)
- [commands.rs](frontend/src-tauri/src/summary/summary_engine/commands.rs)
- [mod.rs](frontend/src-tauri/src/summary/summary_engine/mod.rs)
- [model_manager.rs](frontend/src-tauri/src/summary/summary_engine/model_manager.rs)
- [models.rs](frontend/src-tauri/src/summary/summary_engine/models.rs)
- [sidecar.rs](frontend/src-tauri/src/summary/summary_engine/sidecar.rs)

</details>

<details>
<summary>frontend/src-tauri/src/summary/templates — 4</summary>

- [defaults.rs](frontend/src-tauri/src/summary/templates/defaults.rs)
- [loader.rs](frontend/src-tauri/src/summary/templates/loader.rs)
- [mod.rs](frontend/src-tauri/src/summary/templates/mod.rs)
- [types.rs](frontend/src-tauri/src/summary/templates/types.rs)

</details>

<details>
<summary>frontend/src-tauri/src/whisper_engine — 8</summary>

- [_stderr_suppressor.rs](frontend/src-tauri/src/whisper_engine/_stderr_suppressor.rs)
- [acceleration.rs](frontend/src-tauri/src/whisper_engine/acceleration.rs)
- [commands.rs](frontend/src-tauri/src/whisper_engine/commands.rs)
- [mod.rs](frontend/src-tauri/src/whisper_engine/mod.rs)
- [parallel_commands.rs](frontend/src-tauri/src/whisper_engine/parallel_commands.rs)
- [parallel_processor.rs](frontend/src-tauri/src/whisper_engine/parallel_processor.rs)
- [system_monitor.rs](frontend/src-tauri/src/whisper_engine/system_monitor.rs)
- [whisper_engine.rs](frontend/src-tauri/src/whisper_engine/whisper_engine.rs)

</details>

<details>
<summary>frontend/src-tauri/templates — 8</summary>

- [README.md](frontend/src-tauri/templates/README.md)
- [daily_standup.json](frontend/src-tauri/templates/daily_standup.json)
- [project_sync.json](frontend/src-tauri/templates/project_sync.json)
- [psychatric_session.json](frontend/src-tauri/templates/psychatric_session.json)
- [retrospective.json](frontend/src-tauri/templates/retrospective.json)
- [sales_marketing_client_call.json](frontend/src-tauri/templates/sales_marketing_client_call.json)
- [standard_meeting.json](frontend/src-tauri/templates/standard_meeting.json)
- [tasks_only.json](frontend/src-tauri/templates/tasks_only.json)

</details>

<details>
<summary>frontend/src-tauri/tests — 15</summary>

- [data_integrity.rs](frontend/src-tauri/tests/data_integrity.rs)
- [database_recovery_backup.rs](frontend/src-tauri/tests/database_recovery_backup.rs)
- [diarization_apply.rs](frontend/src-tauri/tests/diarization_apply.rs)
- [diarization_migration.rs](frontend/src-tauri/tests/diarization_migration.rs)
- [diarization_runtime.rs](frontend/src-tauri/tests/diarization_runtime.rs)
- [export_integrity.rs](frontend/src-tauri/tests/export_integrity.rs)
- [local_model_runtime.rs](frontend/src-tauri/tests/local_model_runtime.rs)
- [meeting_context.rs](frontend/src-tauri/tests/meeting_context.rs)
- [meeting_provenance.rs](frontend/src-tauri/tests/meeting_provenance.rs)
- [meeting_questions.rs](frontend/src-tauri/tests/meeting_questions.rs)
- [meeting_questions_local_model.rs](frontend/src-tauri/tests/meeting_questions_local_model.rs)
- [meeting_search.rs](frontend/src-tauri/tests/meeting_search.rs)
- [recording_recovery.rs](frontend/src-tauri/tests/recording_recovery.rs)
- [speaker_rename.rs](frontend/src-tauri/tests/speaker_rename.rs)
- [vocabulary.rs](frontend/src-tauri/tests/vocabulary.rs)

</details>

<details>
<summary>frontend/src-tauri/tests/fixtures — 1</summary>

- [legacy_fastapi_schema.sql](frontend/src-tauri/tests/fixtures/legacy_fastapi_schema.sql)

</details>

<details>
<summary>frontend/src/app — 6</summary>

- [favicon.ico](frontend/src/app/favicon.ico)
- [globals.css](frontend/src/app/globals.css)
- [layout.tsx](frontend/src/app/layout.tsx)
- [metadata.ts](frontend/src/app/metadata.ts)
- [metadata.tsx](frontend/src/app/metadata.tsx)
- [page.tsx](frontend/src/app/page.tsx)

</details>

<details>
<summary>frontend/src/app/_components — 3</summary>

- [SettingsModal.tsx](frontend/src/app/_components/SettingsModal.tsx)
- [StatusOverlays.tsx](frontend/src/app/_components/StatusOverlays.tsx)
- [TranscriptPanel.tsx](frontend/src/app/_components/TranscriptPanel.tsx)

</details>

<details>
<summary>frontend/src/app/meeting-details — 2</summary>

- [page-content.tsx](frontend/src/app/meeting-details/page-content.tsx)
- [page.tsx](frontend/src/app/meeting-details/page.tsx)

</details>

<details>
<summary>frontend/src/app/notes/[id] — 1</summary>

- [page.tsx](frontend/src/app/notes/%5Bid%5D/page.tsx)

</details>

<details>
<summary>frontend/src/app/settings — 1</summary>

- [page.tsx](frontend/src/app/settings/page.tsx)

</details>

<details>
<summary>frontend/src/components — 35</summary>

- [About.tsx](frontend/src/components/About.tsx)
- [AudioLevelMeter.tsx](frontend/src/components/AudioLevelMeter.tsx)
- [AudioPlayer.tsx](frontend/src/components/AudioPlayer.tsx)
- [BetaSettings.tsx](frontend/src/components/BetaSettings.tsx)
- [BluetoothPlaybackWarning.tsx](frontend/src/components/BluetoothPlaybackWarning.tsx)
- [BuiltInModelManager.tsx](frontend/src/components/BuiltInModelManager.tsx)
- [ChunkProgressDisplay.tsx](frontend/src/components/ChunkProgressDisplay.tsx)
- [ComplianceNotification.tsx](frontend/src/components/ComplianceNotification.tsx)
- [ConfidenceIndicator.tsx](frontend/src/components/ConfidenceIndicator.tsx)
- [ConsoleToggle.tsx](frontend/src/components/ConsoleToggle.tsx)
- [CustomDialog.tsx](frontend/src/components/CustomDialog.tsx)
- [DeviceSelection.tsx](frontend/src/components/DeviceSelection.tsx)
- [EditableTitle.tsx](frontend/src/components/EditableTitle.tsx)
- [EmptyStateSummary.tsx](frontend/src/components/EmptyStateSummary.tsx)
- [Info.tsx](frontend/src/components/Info.tsx)
- [LanguagePickerPopover.tsx](frontend/src/components/LanguagePickerPopover.tsx)
- [LanguageSelection.tsx](frontend/src/components/LanguageSelection.tsx)
- [Logo.tsx](frontend/src/components/Logo.tsx)
- [MessageToast.tsx](frontend/src/components/MessageToast.tsx)
- [ModelDownloadProgress.tsx](frontend/src/components/ModelDownloadProgress.tsx)
- [ModelSettingsModal.tsx](frontend/src/components/ModelSettingsModal.tsx)
- [ParakeetModelManager.tsx](frontend/src/components/ParakeetModelManager.tsx)
- [PermissionWarning.tsx](frontend/src/components/PermissionWarning.tsx)
- [PreferenceSettings.tsx](frontend/src/components/PreferenceSettings.tsx)
- [RecordingControls.tsx](frontend/src/components/RecordingControls.tsx)
- [RecordingSettings.tsx](frontend/src/components/RecordingSettings.tsx)
- [RecordingStatusBar.tsx](frontend/src/components/RecordingStatusBar.tsx)
- [SettingTabs.tsx](frontend/src/components/SettingTabs.tsx)
- [SummaryLanguageSettings.tsx](frontend/src/components/SummaryLanguageSettings.tsx)
- [SummaryModelSettings.tsx](frontend/src/components/SummaryModelSettings.tsx)
- [ThemeScript.tsx](frontend/src/components/ThemeScript.tsx)
- [TranscriptSettings.tsx](frontend/src/components/TranscriptSettings.tsx)
- [TranscriptView.tsx](frontend/src/components/TranscriptView.tsx)
- [VirtualizedTranscriptView.tsx](frontend/src/components/VirtualizedTranscriptView.tsx)
- [WhisperModelManager.tsx](frontend/src/components/WhisperModelManager.tsx)

</details>

<details>
<summary>frontend/src/components/AISummary — 4</summary>

- [Block.tsx](frontend/src/components/AISummary/Block.tsx)
- [BlockNoteSummaryView.tsx](frontend/src/components/AISummary/BlockNoteSummaryView.tsx)
- [Section.tsx](frontend/src/components/AISummary/Section.tsx)
- [index.tsx](frontend/src/components/AISummary/index.tsx)

</details>

<details>
<summary>frontend/src/components/BlockNoteEditor — 2</summary>

- [BasicBlockNoteTest.tsx](frontend/src/components/BlockNoteEditor/BasicBlockNoteTest.tsx)
- [Editor.tsx](frontend/src/components/BlockNoteEditor/Editor.tsx)

</details>

<details>
<summary>frontend/src/components/ConfirmationModel — 1</summary>

- [confirmation-modal.tsx](frontend/src/components/ConfirmationModel/confirmation-modal.tsx)

</details>

<details>
<summary>frontend/src/components/DatabaseImport — 2</summary>

- [HomebrewDatabaseDetector.tsx](frontend/src/components/DatabaseImport/HomebrewDatabaseDetector.tsx)
- [LegacyDatabaseImport.tsx](frontend/src/components/DatabaseImport/LegacyDatabaseImport.tsx)

</details>

<details>
<summary>frontend/src/components/ImportAudio — 3</summary>

- [ImportAudioDialog.tsx](frontend/src/components/ImportAudio/ImportAudioDialog.tsx)
- [ImportDropOverlay.tsx](frontend/src/components/ImportAudio/ImportDropOverlay.tsx)
- [index.ts](frontend/src/components/ImportAudio/index.ts)

</details>

<details>
<summary>frontend/src/components/MainContent — 1</summary>

- [index.tsx](frontend/src/components/MainContent/index.tsx)

</details>

<details>
<summary>frontend/src/components/MainNav — 1</summary>

- [index.tsx](frontend/src/components/MainNav/index.tsx)

</details>

<details>
<summary>frontend/src/components/MeetingDetails — 9</summary>

- [MeetingQuestions.tsx](frontend/src/components/MeetingDetails/MeetingQuestions.tsx)
- [RetranscribeDialog.tsx](frontend/src/components/MeetingDetails/RetranscribeDialog.tsx)
- [SpeakerLabelEditor.tsx](frontend/src/components/MeetingDetails/SpeakerLabelEditor.tsx)
- [SummaryGeneratorButtonGroup.tsx](frontend/src/components/MeetingDetails/SummaryGeneratorButtonGroup.tsx)
- [SummaryPanel.tsx](frontend/src/components/MeetingDetails/SummaryPanel.tsx)
- [SummaryUpdaterButtonGroup.tsx](frontend/src/components/MeetingDetails/SummaryUpdaterButtonGroup.tsx)
- [TemplateEditorModal.tsx](frontend/src/components/MeetingDetails/TemplateEditorModal.tsx)
- [TranscriptButtonGroup.tsx](frontend/src/components/MeetingDetails/TranscriptButtonGroup.tsx)
- [TranscriptPanel.tsx](frontend/src/components/MeetingDetails/TranscriptPanel.tsx)

</details>

<details>
<summary>frontend/src/components/Sidebar — 2</summary>

- [SidebarProvider.tsx](frontend/src/components/Sidebar/SidebarProvider.tsx)
- [index.tsx](frontend/src/components/Sidebar/index.tsx)

</details>

<details>
<summary>frontend/src/components/TranscriptRecovery — 2</summary>

- [TranscriptRecovery.tsx](frontend/src/components/TranscriptRecovery/TranscriptRecovery.tsx)
- [index.ts](frontend/src/components/TranscriptRecovery/index.ts)

</details>

<details>
<summary>frontend/src/components/molecules/form-components — 3</summary>

- [form-input-item.tsx](frontend/src/components/molecules/form-components/form-input-item.tsx)
- [form-input-switch.tsx](frontend/src/components/molecules/form-components/form-input-switch.tsx)
- [form-select-item.tsx](frontend/src/components/molecules/form-components/form-select-item.tsx)

</details>

<details>
<summary>frontend/src/components/onboarding — 3</summary>

- [OnboardingContainer.tsx](frontend/src/components/onboarding/OnboardingContainer.tsx)
- [OnboardingFlow.tsx](frontend/src/components/onboarding/OnboardingFlow.tsx)
- [index.ts](frontend/src/components/onboarding/index.ts)

</details>

<details>
<summary>frontend/src/components/onboarding/shared — 4</summary>

- [PermissionRow.tsx](frontend/src/components/onboarding/shared/PermissionRow.tsx)
- [ProgressIndicator.tsx](frontend/src/components/onboarding/shared/ProgressIndicator.tsx)
- [StatusIndicator.tsx](frontend/src/components/onboarding/shared/StatusIndicator.tsx)
- [index.ts](frontend/src/components/onboarding/shared/index.ts)

</details>

<details>
<summary>frontend/src/components/onboarding/steps — 5</summary>

- [DownloadProgressStep.tsx](frontend/src/components/onboarding/steps/DownloadProgressStep.tsx)
- [PermissionsStep.tsx](frontend/src/components/onboarding/steps/PermissionsStep.tsx)
- [SetupOverviewStep.tsx](frontend/src/components/onboarding/steps/SetupOverviewStep.tsx)
- [WelcomeStep.tsx](frontend/src/components/onboarding/steps/WelcomeStep.tsx)
- [index.ts](frontend/src/components/onboarding/steps/index.ts)

</details>

<details>
<summary>frontend/src/components/shared — 1</summary>

- [DownloadProgressToast.tsx](frontend/src/components/shared/DownloadProgressToast.tsx)

</details>

<details>
<summary>frontend/src/components/ui — 23</summary>

- [accordion.tsx](frontend/src/components/ui/accordion.tsx)
- [alert.tsx](frontend/src/components/ui/alert.tsx)
- [button-group.tsx](frontend/src/components/ui/button-group.tsx)
- [button.tsx](frontend/src/components/ui/button.tsx)
- [command.tsx](frontend/src/components/ui/command.tsx)
- [contextual-icon.tsx](frontend/src/components/ui/contextual-icon.tsx)
- [dialog.tsx](frontend/src/components/ui/dialog.tsx)
- [dropdown-menu.tsx](frontend/src/components/ui/dropdown-menu.tsx)
- [form.tsx](frontend/src/components/ui/form.tsx)
- [input-group.tsx](frontend/src/components/ui/input-group.tsx)
- [input.tsx](frontend/src/components/ui/input.tsx)
- [label.tsx](frontend/src/components/ui/label.tsx)
- [popover.tsx](frontend/src/components/ui/popover.tsx)
- [progress.tsx](frontend/src/components/ui/progress.tsx)
- [scroll-area.tsx](frontend/src/components/ui/scroll-area.tsx)
- [select.tsx](frontend/src/components/ui/select.tsx)
- [separator.tsx](frontend/src/components/ui/separator.tsx)
- [sheet.tsx](frontend/src/components/ui/sheet.tsx)
- [switch.tsx](frontend/src/components/ui/switch.tsx)
- [tabs.tsx](frontend/src/components/ui/tabs.tsx)
- [textarea.tsx](frontend/src/components/ui/textarea.tsx)
- [tooltip.tsx](frontend/src/components/ui/tooltip.tsx)
- [visually-hidden.tsx](frontend/src/components/ui/visually-hidden.tsx)

</details>

<details>
<summary>frontend/src/config — 1</summary>

- [api.ts](frontend/src/config/api.ts)

</details>

<details>
<summary>frontend/src/constants — 3</summary>

- [audioFormats.ts](frontend/src/constants/audioFormats.ts)
- [languages.ts](frontend/src/constants/languages.ts)
- [modelDefaults.ts](frontend/src/constants/modelDefaults.ts)

</details>

<details>
<summary>frontend/src/contexts — 8</summary>

- [ConfigContext.tsx](frontend/src/contexts/ConfigContext.tsx)
- [I18nContext.tsx](frontend/src/contexts/I18nContext.tsx)
- [ImportDialogContext.tsx](frontend/src/contexts/ImportDialogContext.tsx)
- [OllamaDownloadContext.tsx](frontend/src/contexts/OllamaDownloadContext.tsx)
- [OnboardingContext.tsx](frontend/src/contexts/OnboardingContext.tsx)
- [RecordingPostProcessingProvider.tsx](frontend/src/contexts/RecordingPostProcessingProvider.tsx)
- [RecordingStateContext.tsx](frontend/src/contexts/RecordingStateContext.tsx)
- [TranscriptContext.tsx](frontend/src/contexts/TranscriptContext.tsx)

</details>

<details>
<summary>frontend/src/hooks — 17</summary>

- [use-live-reduced-motion.ts](frontend/src/hooks/use-live-reduced-motion.ts)
- [useAudioPlayer.ts](frontend/src/hooks/useAudioPlayer.ts)
- [useAutoScroll.ts](frontend/src/hooks/useAutoScroll.ts)
- [useImportAudio.ts](frontend/src/hooks/useImportAudio.ts)
- [useModalState.ts](frontend/src/hooks/useModalState.ts)
- [useNavigation.ts](frontend/src/hooks/useNavigation.ts)
- [usePaginatedTranscripts.ts](frontend/src/hooks/usePaginatedTranscripts.ts)
- [usePermissionCheck.ts](frontend/src/hooks/usePermissionCheck.ts)
- [usePlatform.ts](frontend/src/hooks/usePlatform.ts)
- [useProcessingProgress.ts](frontend/src/hooks/useProcessingProgress.ts)
- [useRecentLanguages.ts](frontend/src/hooks/useRecentLanguages.ts)
- [useRecordingStart.ts](frontend/src/hooks/useRecordingStart.ts)
- [useRecordingStateSync.ts](frontend/src/hooks/useRecordingStateSync.ts)
- [useRecordingStop.ts](frontend/src/hooks/useRecordingStop.ts)
- [useTranscriptRecovery.ts](frontend/src/hooks/useTranscriptRecovery.ts)
- [useTranscriptStreaming.ts](frontend/src/hooks/useTranscriptStreaming.ts)
- [useTranscriptionModels.ts](frontend/src/hooks/useTranscriptionModels.ts)

</details>

<details>
<summary>frontend/src/hooks/meeting-details — 8</summary>

- [useCopyOperations.ts](frontend/src/hooks/meeting-details/useCopyOperations.ts)
- [useDiarizationRun.ts](frontend/src/hooks/meeting-details/useDiarizationRun.ts)
- [useMeetingData.ts](frontend/src/hooks/meeting-details/useMeetingData.ts)
- [useMeetingExport.ts](frontend/src/hooks/meeting-details/useMeetingExport.ts)
- [useMeetingOperations.ts](frontend/src/hooks/meeting-details/useMeetingOperations.ts)
- [useModelConfiguration.ts](frontend/src/hooks/meeting-details/useModelConfiguration.ts)
- [useSummaryGeneration.ts](frontend/src/hooks/meeting-details/useSummaryGeneration.ts)
- [useTemplates.ts](frontend/src/hooks/meeting-details/useTemplates.ts)

</details>

<details>
<summary>frontend/src/lib — 20</summary>

- [audioPermissions.ts](frontend/src/lib/audioPermissions.ts)
- [blocknote-markdown.ts](frontend/src/lib/blocknote-markdown.ts)
- [builtin-ai-models.ts](frontend/src/lib/builtin-ai-models.ts)
- [builtin-ai.ts](frontend/src/lib/builtin-ai.ts)
- [meeting-export.ts](frontend/src/lib/meeting-export.ts)
- [meeting-search.ts](frontend/src/lib/meeting-search.ts)
- [meeting-summary.ts](frontend/src/lib/meeting-summary.ts)
- [meeting-transcripts.ts](frontend/src/lib/meeting-transcripts.ts)
- [motion.ts](frontend/src/lib/motion.ts)
- [onboarding-summary-model.ts](frontend/src/lib/onboarding-summary-model.ts)
- [parakeet.ts](frontend/src/lib/parakeet.ts)
- [recordingNotification.tsx](frontend/src/lib/recordingNotification.tsx)
- [retranscription-vocabulary.ts](frontend/src/lib/retranscription-vocabulary.ts)
- [summary-language-preferences.ts](frontend/src/lib/summary-language-preferences.ts)
- [summary-languages.ts](frontend/src/lib/summary-languages.ts)
- [summary-markdown.ts](frontend/src/lib/summary-markdown.ts)
- [theme.ts](frontend/src/lib/theme.ts)
- [transcript-pagination.ts](frontend/src/lib/transcript-pagination.ts)
- [utils.ts](frontend/src/lib/utils.ts)
- [whisper.ts](frontend/src/lib/whisper.ts)

</details>

<details>
<summary>frontend/src/lib/i18n — 1</summary>

- [catalog.ts](frontend/src/lib/i18n/catalog.ts)

</details>

<details>
<summary>frontend/src/services — 6</summary>

- [configService.ts](frontend/src/services/configService.ts)
- [indexedDBService.ts](frontend/src/services/indexedDBService.ts)
- [recordingService.ts](frontend/src/services/recordingService.ts)
- [storageService.ts](frontend/src/services/storageService.ts)
- [transcriptService.ts](frontend/src/services/transcriptService.ts)
- [webMigration.ts](frontend/src/services/webMigration.ts)

</details>

<details>
<summary>frontend/src/testing — 4</summary>

- [E2EBootstrap.tsx](frontend/src/testing/E2EBootstrap.tsx)
- [E2ENoop.tsx](frontend/src/testing/E2ENoop.tsx)
- [form-molecules.typecheck.tsx](frontend/src/testing/form-molecules.typecheck.tsx)
- [tauri-browser-mocks.ts](frontend/src/testing/tauri-browser-mocks.ts)

</details>

<details>
<summary>frontend/src/types — 5</summary>

- [betaFeatures.ts](frontend/src/types/betaFeatures.ts)
- [global.d.ts](frontend/src/types/global.d.ts)
- [index.ts](frontend/src/types/index.ts)
- [onboarding.ts](frontend/src/types/onboarding.ts)
- [summary.ts](frontend/src/types/summary.ts)

</details>

<details>
<summary>frontend/tests — 47</summary>

- [cargo-workspace-config.test.mjs](frontend/tests/cargo-workspace-config.test.mjs)
- [check-command.test.mjs](frontend/tests/check-command.test.mjs)
- [desktop-local-safety.spec.ts](frontend/tests/desktop-local-safety.spec.ts)
- [diarization-action-contract.test.mjs](frontend/tests/diarization-action-contract.test.mjs)
- [diarization-artifacts.test.mjs](frontend/tests/diarization-artifacts.test.mjs)
- [e2e-build-boundary.test.mjs](frontend/tests/e2e-build-boundary.test.mjs)
- [fixture-contracts.spec.ts](frontend/tests/fixture-contracts.spec.ts)
- [global-setup.ts](frontend/tests/global-setup.ts)
- [indexeddb-transcript-contract.spec.ts](frontend/tests/indexeddb-transcript-contract.spec.ts)
- [interface-language.spec.ts](frontend/tests/interface-language.spec.ts)
- [interface-polish-acceptance.spec.ts](frontend/tests/interface-polish-acceptance.spec.ts)
- [interface-polish-policy.spec.ts](frontend/tests/interface-polish-policy.spec.ts)
- [interface-polish-runtime.spec.ts](frontend/tests/interface-polish-runtime.spec.ts)
- [interrupted-recovery.spec.ts](frontend/tests/interrupted-recovery.spec.ts)
- [lint-tooling.test.mjs](frontend/tests/lint-tooling.test.mjs)
- [macos-audio-surface.test.mjs](frontend/tests/macos-audio-surface.test.mjs)
- [macos-permissions-lifecycle.spec.ts](frontend/tests/macos-permissions-lifecycle.spec.ts)
- [meeting-export-contract.test.mjs](frontend/tests/meeting-export-contract.test.mjs)
- [meeting-questions.spec.ts](frontend/tests/meeting-questions.spec.ts)
- [meeting-search.spec.ts](frontend/tests/meeting-search.spec.ts)
- [migration-source-integrity.test.mjs](frontend/tests/migration-source-integrity.test.mjs)
- [model-config-react-effects.test.mjs](frontend/tests/model-config-react-effects.test.mjs)
- [onboarding-react-effects.test.mjs](frontend/tests/onboarding-react-effects.test.mjs)
- [platform-surface.test.mjs](frontend/tests/platform-surface.test.mjs)
- [playwright-config.test.mjs](frontend/tests/playwright-config.test.mjs)
- [publication-privacy.test.mjs](frontend/tests/publication-privacy.test.mjs)
- [react-hooks-lifecycle.test.mjs](frontend/tests/react-hooks-lifecycle.test.mjs)
- [recording-controls.test.mjs](frontend/tests/recording-controls.test.mjs)
- [recording-folder-selection.spec.ts](frontend/tests/recording-folder-selection.spec.ts)
- [release-candidate.test.mjs](frontend/tests/release-candidate.test.mjs)
- [release-policy.test.mjs](frontend/tests/release-policy.test.mjs)
- [runtime-smoke-contract.test.mjs](frontend/tests/runtime-smoke-contract.test.mjs)
- [smoke-report.test.mjs](frontend/tests/smoke-report.test.mjs)
- [source-fingerprint.test.mjs](frontend/tests/source-fingerprint.test.mjs)
- [speaker-label-ui-contract.test.mjs](frontend/tests/speaker-label-ui-contract.test.mjs)
- [summary-template-selection.spec.ts](frontend/tests/summary-template-selection.spec.ts)
- [testing-guide.test.mjs](frontend/tests/testing-guide.test.mjs)
- [theme-dark-surfaces.spec.ts](frontend/tests/theme-dark-surfaces.spec.ts)
- [theme-foundation.spec.ts](frontend/tests/theme-foundation.spec.ts)
- [theme-primitives.spec.ts](frontend/tests/theme-primitives.spec.ts)
- [theme-secondary-surfaces.spec.ts](frontend/tests/theme-secondary-surfaces.spec.ts)
- [theme-shell.spec.ts](frontend/tests/theme-shell.spec.ts)
- [universal-build-config.spec.ts](frontend/tests/universal-build-config.spec.ts)
- [universal-feature-contract.spec.ts](frontend/tests/universal-feature-contract.spec.ts)
- [universal-onnx-runtime.test.mjs](frontend/tests/universal-onnx-runtime.test.mjs)
- [whitespace-command.test.mjs](frontend/tests/whitespace-command.test.mjs)
- [workflow-gate.test.mjs](frontend/tests/workflow-gate.test.mjs)

</details>

<details>
<summary>frontend/tests/fixtures — 1</summary>

- [tauri-fixtures.ts](frontend/tests/fixtures/tauri-fixtures.ts)

</details>

<details>
<summary>frontend/tests/interface-polish-acceptance.spec.ts-snapshots — 2</summary>

- [interface-polish-onboarding-step2-chromium-darwin.png](frontend/tests/interface-polish-acceptance.spec.ts-snapshots/interface-polish-onboarding-step2-chromium-darwin.png)
- [interface-polish-onboarding-step2-narrow-chromium-darwin.png](frontend/tests/interface-polish-acceptance.spec.ts-snapshots/interface-polish-onboarding-step2-narrow-chromium-darwin.png)

</details>

<details>
<summary>frontend/tests/lib — 21</summary>

- [audio-permissions.test.ts](frontend/tests/lib/audio-permissions.test.ts)
- [blocknote-markdown.test.ts](frontend/tests/lib/blocknote-markdown.test.ts)
- [builtin-ai-models.test.mjs](frontend/tests/lib/builtin-ai-models.test.mjs)
- [i18n.test.ts](frontend/tests/lib/i18n.test.ts)
- [import-title-provenance.test.mjs](frontend/tests/lib/import-title-provenance.test.mjs)
- [indexeddb-service.test.mjs](frontend/tests/lib/indexeddb-service.test.mjs)
- [meeting-export.test.ts](frontend/tests/lib/meeting-export.test.ts)
- [meeting-search.test.mjs](frontend/tests/lib/meeting-search.test.mjs)
- [meeting-summary.test.ts](frontend/tests/lib/meeting-summary.test.ts)
- [onboarding-summary-model.test.mjs](frontend/tests/lib/onboarding-summary-model.test.mjs)
- [product-analytics-absence.test.mjs](frontend/tests/lib/product-analytics-absence.test.mjs)
- [product-analytics-startup-behavior.test.mjs](frontend/tests/lib/product-analytics-startup-behavior.test.mjs)
- [retranscription-vocabulary.test.ts](frontend/tests/lib/retranscription-vocabulary.test.ts)
- [storage-service.test.mjs](frontend/tests/lib/storage-service.test.mjs)
- [summary-language-preferences.test.js](frontend/tests/lib/summary-language-preferences.test.js)
- [template-crud-contract.test.mjs](frontend/tests/lib/template-crud-contract.test.mjs)
- [template-serialization.test.ts](frontend/tests/lib/template-serialization.test.ts)
- [transcript-pagination.test.mjs](frontend/tests/lib/transcript-pagination.test.mjs)
- [transcript-recovery-hook.test.mjs](frontend/tests/lib/transcript-recovery-hook.test.mjs)
- [transcript-speakers.test.ts](frontend/tests/lib/transcript-speakers.test.ts)
- [web-migration.test.ts](frontend/tests/lib/web-migration.test.ts)

</details>

<details>
<summary>frontend/tests/theme-shell.spec.ts-snapshots — 2</summary>

- [home-dark-chromium-darwin.png](frontend/tests/theme-shell.spec.ts-snapshots/home-dark-chromium-darwin.png)
- [home-light-chromium-darwin.png](frontend/tests/theme-shell.spec.ts-snapshots/home-light-chromium-darwin.png)

</details>

<details>
<summary>llama-helper — 1</summary>

- [Cargo.toml](llama-helper/Cargo.toml)

</details>

<details>
<summary>llama-helper/src — 1</summary>

- [main.rs](llama-helper/src/main.rs)

</details>

<details>
<summary>outputs — 2</summary>

- [bug-reproducer-evidence.json](outputs/bug-reproducer-evidence.json)
- [bug-reproducer-report.md](outputs/bug-reproducer-report.md)

</details>

<details>
<summary>scripts — 12</summary>

- [check-diarization-backend-migration.sh](scripts/check-diarization-backend-migration.sh)
- [check-diarization-plan-migration.sh](scripts/check-diarization-plan-migration.sh)
- [check-whitespace.sh](scripts/check-whitespace.sh)
- [check.sh](scripts/check.sh)
- [generate-diarization-fixture.sh](scripts/generate-diarization-fixture.sh)
- [inject_transcript.py](scripts/inject_transcript.py)
- [project-index.py](scripts/project-index.py)
- [release-candidate.sh](scripts/release-candidate.sh)
- [release-gate.sh](scripts/release-gate.sh)
- [smoke-local-models.sh](scripts/smoke-local-models.sh)
- [smoke-macos-audio.sh](scripts/smoke-macos-audio.sh)
- [test-plan-gate.sh](scripts/test-plan-gate.sh)

</details>

<!-- project-files:end -->
