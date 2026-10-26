# Замечания перед возвращением к разработке — 2026-10-26

База: `b4ec0ae1665b46a96af1da186bfd00a0c1a01c8f`, ветка
`feature/publish-reviewed-2026-10-25`. Семь тематических коммитов опубликованы,
удалённый SHA совпал. Это не merge в `main`/`dev` и не готовность релиза.

[Полный аудит и исходные журналы](completed-work-audit-2026-10-25.md).
Проверенный продуктовый код — `4dee822696361b39f735c0fcd3aa74608b82023b`;
последующий commit содержит только документацию/evidence. Полный автоматический
gate завершился exit 0, 11/11 PASS. Первичные диагностические FAIL сохранены.

## FORK-149: 48 замечаний platform-surface

Scanner завершился exit 1. Его 30 тестов проходят; это не означает отсутствия
замечаний в проекте. 48 совпадений требуют разбора применимости, это не
48 подтверждённых runtime-дефектов. FORK-149 остаётся открытой.

| Категория | Количество |
| --- | ---: |
| platform.docs | 5 |
| platform.workflow-runner | 11 |
| platform.workflow-condition | 17 |
| platform.target | 6 |
| platform.condition | 9 |

Список ниже перенесён из результата scanner, полученного при завершении аудита;
сегодня scanner повторно не запускался.

| Код | Путь | Наблюдение scanner |
| --- | --- | --- |
| `platform.docs` | `.github/workflows/ACCELERATION_GUIDE.md` | Current instructions advertise or install a foreign platform. |
| `platform.docs` | `.github/workflows/README_DEVTEST.md` | Current instructions advertise or install a foreign platform. |
| `platform.docs` | `.github/workflows/WORKFLOWS_OVERVIEW.md` | Current instructions advertise or install a foreign platform. |
| `platform.workflow-runner` | `.github/workflows/build-devtest.yml` | Job build-devtest has a non-macOS or unresolved runner. |
| `platform.workflow-condition` | `.github/workflows/build-devtest.yml` | Job build-devtest has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build-devtest.yml` | Job build-devtest has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build-devtest.yml` | Job build-devtest has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build-devtest.yml` | Job build-devtest has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build-devtest.yml` | Job build-devtest has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build-devtest.yml` | Job build-devtest has a foreign platform condition. |
| `platform.target` | `.github/workflows/build-devtest.yml` | Foreign platform compilation target. |
| `platform.workflow-runner` | `.github/workflows/build-linux.yml` | Job build-linux has a non-macOS or unresolved runner. |
| `platform.target` | `.github/workflows/build-linux.yml` | Foreign platform compilation target. |
| `platform.target` | `.github/workflows/build-test.yml` | Foreign platform compilation target. |
| `platform.workflow-runner` | `.github/workflows/build-windows.yml` | Job build-windows has a non-macOS or unresolved runner. |
| `platform.target` | `.github/workflows/build-windows.yml` | Foreign platform compilation target. |
| `platform.workflow-runner` | `.github/workflows/build.yml` | Job build has a non-macOS or unresolved runner. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.workflow-condition` | `.github/workflows/build.yml` | Job build has a foreign platform condition. |
| `platform.target` | `.github/workflows/build.yml` | Foreign platform compilation target. |
| `platform.workflow-runner` | `.github/workflows/pr-main-check.yml` | Job validation-check has a non-macOS or unresolved runner. |
| `platform.workflow-runner` | `.github/workflows/quality-gate.yml` | Job source-integrity has a non-macOS or unresolved runner. |
| `platform.workflow-runner` | `.github/workflows/quality-gate.yml` | Job frontend has a non-macOS or unresolved runner. |
| `platform.workflow-runner` | `.github/workflows/quality-gate.yml` | Job quality-gate has a non-macOS or unresolved runner. |
| `platform.workflow-runner` | `.github/workflows/release.yml` | Job manual-attestation has a non-macOS or unresolved runner. |
| `platform.workflow-runner` | `.github/workflows/release.yml` | Job create-release has a non-macOS or unresolved runner. |
| `platform.workflow-runner` | `.github/workflows/release.yml` | Job release-summary has a non-macOS or unresolved runner. |
| `platform.target` | `.github/workflows/release.yml` | Foreign platform compilation target. |
| `platform.docs` | `README.md` | Current instructions advertise or install a foreign platform. |
| `platform.docs` | `docs/building_in_linux.md` | Current instructions advertise or install a foreign platform. |
| `platform.condition` | `frontend/src-tauri/src/audio/decoder.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/device_detection.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/devices/discovery.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/devices/speakers.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/diagnostics.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/encode.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/incremental_saver.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/playback_monitor.rs` | Rust cfg contains a foreign platform condition. |
| `platform.condition` | `frontend/src-tauri/src/audio/recording_preferences.rs` | Rust cfg contains a foreign platform condition. |

Перед закрытием: повторить существующий scanner из `frontend/package.json` на
точном commit, сохранить команду/exit/полный JSON; разобрать каждое совпадение.
Отделить legacy-документы и допустимые служебные CI runners от продуктовой
платформенной поверхности. Исправления или исключения должны иметь явную причину;
сохранить обязательные CI/release gates, выполнить адресные тесты и применимый
полный gate, затем обновить PRD по фактическому результату.

В активных инструкциях README/workflow-документации остались Linux/Windows
направления при заявленной macOS-области. Это несогласованность документации
и поддерживаемой области, а не доказательство дефекта поведения. Часть кода
также сохраняет foreign-platform cfg; решение принимается отдельно по каждому
пути, без слепого удаления по совпадению scanner.

## Наблюдения, требующие внимания при повторении

- Cold dev compilation `/meeting-details` занимала 85,3 с при лимите 30 с.
  Два summary assertions затем ожидали 10 lazy JS-чанков BlockNote. Итоговые
  проверки прошли на подготовленном сервере с побайтно сверенными исходниками;
  assertions и таймауты не ослаблялись. Renderer не менялся относительно базы.
  Точная системная причина задержки ресурсов не установлена; единичный повтор
  не считать исправлением нестабильности.
- Первичный `rust.universal-helper-routing` policy-отказ на тех же байтах
  не воспроизведён, в том числе в 30 отдельных probes. Причина неизвестна;
  исходный FAIL сохранён. При повторении исследовать evaluator/среду.
- Исправление loopback Q&A proxy и Rust 1.99/Clippy описано в полном аудите;
  это закрытые адресные находки, их не нужно повторно записывать как открытые.

## Финальная native/release приёмка не выполнена

Новый `.app`/DMG для этой ревизии не собирался. `release-candidate.sh`, candidate
verifier, точный smoke-report и `release-gate.sh` не выполнялись. Из 11 ignored
Rust-тестов один FFmpeg-сценарий прошёл отдельной стадией; десять opt-in сценариев
не запускались. Исторические Gemma/native результаты не переносятся на новый
кандидат.

- Установленный Tauri IPC/UI, clean install/TCC, microphone/system audio,
  устройства и качество audio/STT.
- Real-time checkpoint, import/VAD, реальные summary/Parakeet/Gemma,
  двухголосая Sherpa и vocabulary runtime.
- Native/packaged offline, upgrade/reopen и сохранность пользовательских данных.
- arm64/Universal, минимальная macOS, Developer ID, Gatekeeper, notarization,
  updater.

После реализации пройти полный gate, собрать кандидат из чистого checkout,
зафиксировать commit и SHA-256 артефакта/verifier, выполнить приёмку именно этого
артефакта и `release-gate.sh --smoke-report <report>` с exit 0 по `docs/testing.md`.
Локальные воспроизводимые сборочные кэши могут удаляться; восстановить их обычной
сборкой. Исходный dirty main и исторические worktree сохраняются отдельно:
публикация проверенной ветки не означает фиксацию всего содержимого этих деревьев.

GitHub Issues на момент сохранения отключены (API 410), поэтому замечания
сохранены в репозитории этим документом. API GitHub показал `visibility: public`;
видимость не изменялась в рамках этой проверки.
