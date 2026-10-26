> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Срочные задачи: проверка 2026-09-19

Ни одна из пяти задач в этом отчёте не объявляется завершённой. Разделы до капсулы FEAT-071 описывают первоначальную проверку 19 сентября. Позднее продуктовый код менялся и выполнялись новые нативные проверки; актуальные результаты и открытые ошибки находятся в [журнале продолжения](audio-errors-2026-09-19.md). Коммиты, слияния и публикация в этом продолжении не выполнялись.

## Проверенный исходный код и приложение

Рабочая копия: `.worktrees/feat071-audio-controls`, ветка `themaddoxed/feat071-audio-controls`, HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c` с незакоммиченным переносом FORK-051. Состояние сохранено. Контрольная сумма исходников, полученная штатным `frontend/scripts/source-fingerprint.mjs`: `78376351ebf6698fa84f86e8757a58a1e5c5d6d5171b825470ae1d8264c59661`.

Приложение: `/Users/developer/Desktop/meeting-minutes/.worktrees/feat071-audio-controls/target/fork-051/release/bundle/macos/meetily.app`.

SHA-256 executable: `8411ee85cf18693540cb83246f3010edc137f4f3b1b5a4bbb5ec58c675c0228b`, совпадает с последней диагностической сборкой в [предыдущем отчёте](fork051-feat071-verification-2026-09-14.md). Повторная проверка `codesign --verify --deep --strict` завершилась с кодом 0. Приложение x86_64, подпись ad hoc, Team ID отсутствует. Это не Developer ID и не проверка готовности распространения. Новая сборка не выполнялась; совпадение executable само по себе не доказывает полный состав исходников старой сборки.

`RecordingControls.tsx` SHA-256: `567212de5edc365ea6cbd2091786900ad529f715dc893a99b4af8f1422e65f01`, совпадает с исправлением обратной связи старта из предыдущего отчёта.

## Свежие автоматические проверки

Команды выполнены в `feat071-audio-controls/frontend` на Intel macOS.

| Команда | Результат | SHA-256 журнала |
| --- | --- | --- |
| `pnpm typecheck` | PASS, exit 0 | `b52bae3417520b746d762c34a0befc14bc4e00f6bf7788a8921e2802bcbc5ae9` |
| `pnpm test:node` | PASS, 397/397, exit 0 | `fd4d3376c4200dca1415054db6b95cd340d2a8f94581b61931cca239da701289` |
| `pnpm test:unit` | PASS, 48/48, exit 0 | `359d91b17d271a0de63c8709053147f48a00a9ec71c0c16bd93e892e23fb16f4` |
| `pnpm check:policy` | PASS, exit 0 | `aa95c7e22d4f908d911e1b354178a7b850f402d9f97e695eea70d40f19d18689` |

Журналы: `/private/tmp/meetily-urgent-20260919/{typecheck,node,unit,policy}.log`. `git diff --check` также PASS. Rust, браузерные сценарии, полная release-проверка и новая упаковка в этом продолжении не запускались. Прошлые результаты этих проверок не переносятся на текущую дату как новые.

## FORK-051: остающаяся проверка на рабочем столе

Первоначально пользователю отправлен запрос открыть указанный `.app` через Finder. Затем по его просьбе выполнена самостоятельная проверка вне sandbox: процесс и окно запущены, но содержимое UI недоступно для надёжного управления; подробности ниже. Результатов проверки самой записи пока нет. Сброс разрешений не повторялся. Для первого запроса и отказа нужен отдельный тестовый профиль macOS либо явно согласованный сброс, а не изменение основного профиля без необходимости.

| Сценарий | Состояние | Что зафиксировать |
| --- | --- | --- |
| Запуск `.app` через LaunchServices | PASS только для запуска процесса и наличия окна | `/usr/bin/open` вне sandbox: exit 0; PID 60340, окно 182; содержимое интерфейса не проверено |
| Первый запрос микрофона и системного звука | NOT_RUN | Каждый запрос, выбранное действие, доступность источника после ответа |
| Отказ и возврат из настроек macOS | NOT_RUN | Понятная ошибка, отсутствие зацикленного запроса, успешный повтор после разрешения |
| Старт, stop, повторный старт | NOT_RUN | Немедленная индикация запуска, записанная контрольная фраза, воспроизведение сохранённого звука, освобождение микрофона |
| Pause/resume | NOT_RUN | Звук до/после паузы и соответствие времени расшифровки воспроизведению |
| Отключение и замена микрофона | NOT_RUN | Сообщение об ошибке, сохранение записанного, восстановление с выбранным устройством |
| Выход из приложения во время/после записи | NOT_RUN | Завершение потоков, сохранность встречи при повторном открытии |

Успех только start/stop не закрывает всю матрицу. FEAT-071 остаётся за зависимостью FORK-051; подготовленная ранее карта управления дорожками не считается реализацией.

## FORK-036, FORK-037 и FORK-053

FORK-028 содержит действующее решение отложить окончательные имя, bundle ID и репозиторий. Поэтому FORK-036 и FORK-037 остаются открыты. Для updater дополнительно нужны принадлежащие новому продукту адрес канала и открытый ключ проверки обновлений. Секретные ключи не запрашивались и не использовались. Текущая policy отвергает updater dependency, регистрацию, endpoints, pubkey и update artifacts; добавление нового updater требует целевого пересмотра этих проверок вместе с положительными и отрицательными тестами, а не удаления общего запрета без замены.

До выбора identity можно подготовить проект переноса и набор тестовых данных FORK-037; перенос реальных пользовательских данных не выполняется.

Подготовлен [проект FORK-037](fork037-migration-design-2026-09-19.md), SHA-256 `988a8de54a2a55a461b591fe25d452e104062140c18241cdd9edbaf23b81591d`. Независимый reviewer `review_migration_design` проверил именно этот хеш и исходный fingerprint выше. Его заключение (дословная выдержка): «**Status: safe draft.** Подтверждённых опасных указаний, требующих исправления перед сохранением этого черновика, не найдено. Это оценка предварительного плана, **не разрешение на реализацию и не подтверждение сохранности данных при реальном переносе**.»

Reviewer отдельно отметил: notification settings используют общий `dirs::config_dir()/meetily/notifications.json`, поэтому смена bundle ID сама по себе не разделяет владение этим файлом; значение `system_permission_granted` требует повторной проверки. Запрет запуска до завершения переноса должен охватывать все службы, которые записывают настройки или восстанавливают данные. Оба пункта относятся к уточнению окончательного проекта; FORK-037 остаётся открытым. Данный review не является проверкой кода FORK-051 или фактического переноса.

У FORK-053 открыты 12 прямых зависимостей: FORK-035, FORK-036, FORK-038, FORK-039, FORK-040, FORK-042, FORK-043, FORK-047, FORK-048, FORK-050, FORK-051 и FORK-052. FORK-044 отмечен выполненным. Для принятой Alpha обязательны одна чистая зафиксированная ревизия, полный `scripts/check.sh --full`, сборка и проверка кандидата, заполненный smoke report на том же артефакте и успешный `release-gate.sh`. Текущая грязная рабочая копия с диагностическим `.app` этим условиям не соответствует. Создание коммита требует отдельного разрешения согласно проектным инструкциям.

Beads: `bd prime` прочитан; `bd list` не смог открыть базу `/Users/developer/.beads-planning` из-за sandbox (`openat LOCK: operation not permitted`). Статусы верхнеуровневого графа не менялись. Детальные задачи проверены по актуальному PRD и `mdtask view FORK-053`.

## Самостоятельная нативная проверка после запроса пользователя

Пользователь попросил выполнить проверку самостоятельно. Запуск через `/usr/bin/open` внутри sandbox снова вернул `-10827`, но разрешённый запуск той же команды вне sandbox завершился с кодом 0. Процесс `meetily`, PID `60340`, запущен через `launchd`; путь executable совпадает с указанным выше `.app`. Это новое наблюдение снимает прежний блокер запуска, но не подтверждает запись.

CoreGraphics обнаружил окно `meetily` ID `182`, размер `1100×700`, и элемент меню ID `183`. Трёхсекундный `sample` подтверждает работающий AppKit event loop. Система: Intel macOS `15.8 (24H23)`; это свежая среда, отличная от версии macOS в старом crash report.

Управление UI проверено двумя способами. System Events при чтении `entire contents` не завершился в разумное время; следующие ограниченные запросы вернули timeout. Отдельный Swift-клиент показал `AX_TRUSTED=true`, но `AXWindows` содержит повторяющийся `AXApplication`, и обход дерева достигает только системного меню. Запросы `AXFocusedWindow`, `AXMainWindow`, `AXFocusedUIElement` вернули `-25204`; hit-test — `-25200`. Проверка `CGPreflightScreenCaptureAccess()` вернула `true`, однако `screencapture` не смог получить ни display image, ни изображение окна `182`. Отсутствие картинки поэтому не объявляется доказательством отсутствия разрешения. Причина недоступного содержимого окна не установлена; дефект продуктового кода из этого не выводится.

Кнопки записи не нажимались вслепую. Наблюдений start/pause/resume/stop и переключения устройства пока нет. Приложение не закрывалось принудительно, разрешения не сбрасывались.

Журналы в `/private/tmp/meetily-urgent-20260919/`:

- `ax-tree.txt`, SHA-256 `bb2b61428f0d9af8e84ee2643b023975f7e5df95c5fbe40f26a704e5ee1df306`.
- `ax-focus.txt`, SHA-256 `1d506970a8d57ebba8562d7555f59b87beed79326a6e6801c0177f954a4fb9a7`.
- `meetily-sample.txt`, SHA-256 `95fe307fec3f2168dd50c9d5c6e1ab5e0edbfd6ee2a5a445526ad26d8285e915`.

### Аппаратный CoreAudio

Вне sandbox выполнен штатный `scripts/smoke-macos-audio.sh` с `MEETILY_RUN_HARDWARE_TESTS=1`, `MEETILY_PACKAGED_APP` равным указанному `.app` и `CARGO_TARGET_DIR=target/fork-051`. Он локально собрал тестовый Rust executable (`--locked --offline`), сделал временную копию bundle, заменил executable только в ней и воспроизвёл локальный тон. Временная копия удалена самим скриптом. Rust/Cargo `1.98.1`.

Результат `audio::capture::core_audio::tests::test_core_audio_capture`: **1 passed**, 1.20s. По проверенному коду тест получает 48 000 отсчётов, ограничивает ожидание 15 секундами, требует RMS > 0.001 и peak > 0.01. Это свежее подтверждение ненулевого системного аудиозахвата в тестовом процессе на реальном Mac. Это не выполнение записи через UI исходного `.app` и не проверка микрофона.

Обнаружен ложноположительный вывод скрипта: фильтр `audio::system_audio_commands::tests::macos_system_audio_devices_have_valid_names` выполнил **0 tests, 400 filtered out**, но вслед за успешным кодом возврата скрипт напечатал `PASS: real macOS system audio device enumeration`. Такой PASS отвергнут: перечисление устройств не проверено. Соответствующий `system_audio_commands.rs` в текущих исходниках отсутствует. Следует отдельно исправить устаревший фильтр и требовать ненулевого числа выполненных тестов; в этом продолжении скрипт не изменён.

Общий exit скрипта: **2**, `OVERALL NOT_RUN`, микрофон не проверен. Журнал `native-audio.log`, SHA-256 `b0b6ce73ba9b2eb0d94f365dd44425204aa4bfd438ea0c2377ed5f29a2a1eaab`.

После проверки source fingerprint остался `78376351ebf6698fa84f86e8757a58a1e5c5d6d5171b825470ae1d8264c59661`, SHA executable исходного приложения остался `8411ee85cf18693540cb83246f3010edc137f4f3b1b5a4bbb5ec58c675c0228b`; `codesign --verify --deep --strict` и `git diff --check` завершились с кодом 0. FORK-051 остаётся открытым. Приложение оставлено запущенным; принудительное завершение не выполнялось.

## FEAT-071: принятая finite capsule этапа 1 — timeline/transport

Эта capsule относится только к рабочей копии `.worktrees/feat071-audio-controls` и не закрывает физические проверки FORK-051. Этап 1 меняет ровно семь Rust-файлов: `audio/capture/backend.rs`, `audio/capture/core_audio.rs`, `audio/capture/macos.rs`, `audio/pipeline.rs`, `audio/recording_state.rs`, `audio/incremental_saver.rs`, `audio/recording_session.rs`; focused tests находятся inline в этих файлах. Команды, frontend controls/UI и permissions остаются этапом 2.

### Передача блоков

- Callback публикует один полный bounded `TrackBlock` во входную очередь; обработчик вне callback последовательно готовит raw и downstream отправку. В callback запрещены allocate/free и две независимые публикации.
- Pool заранее резервирует `64` слота по `128 KiB` (`4096` native frames × `8` channels × `4` bytes), общий лимит `8 MiB`. Очередь входных блоков — `64` сообщений; существующие raw `4096` и downstream `64` сообщения сохраняются.
- Блок больше `4096` frames разбивается до публикации на последовательные блоки с сохранением clock remainder. Блок с числом каналов больше `8`, нулевым форматом или переполненной позицией отклоняется как terminal incomplete source; descriptor отдельно от samples не отправляется.
- Raw и downstream отправляются последовательно. Частичный отказ не называется атомарным успехом: если первая отправка прошла, а вторая отказала, source получает `incomplete` metadata, дальнейшее объединение прекращается, уже принятый prefix сохраняется.
- При full queue входной slot возвращается в pool после terminal gap/failure decision; при disconnect обработчик закрывает source и освобождает slot после завершения обоих receiver paths. Успешный block нельзя переиспользовать до возврата из всех путей.

### Время и состояния владельца

Каноническая шкала — целые кадры 48 kHz. `native_frame_count` считается как `samples.len() / channels`; resampling 44.1 kHz → 48 kHz использует постоянный rational remainder между блоками. Общий origin фиксируется до запуска microphone/system stream. CoreAudio использует валидный `host_time`/`sample_time`, CPAL relative timestamp помечается approximate.

Каждый active source имеет watermark `600 ms`: owner продвигает `EmptyUntil` отдельно от реального silence. Если данные не пришли до watermark, mixer записывает bounded gap и не перемещает поздние samples вперёд. `Disabled`/`Finished`/`Failed` source не блокирует mixer. Valid gap дольше 60 секунд никогда не сжимается и не re-anchor-ится; нули выдаются блоками по `4096` кадров. Невозможное время сохраняет `known_end`, создаёт degraded/incomplete event и не склеивает неизвестный хвост.

Pause выполняется как переход `prepare boundary → source confirmations → epoch close → ACK`. Boundary публикуется до backend pause; callback без данных тоже видит его. При отказе подтверждения переход становится terminal failure, старые blocks после boundary не пропускаются. Pause interval исключается из active timeline, resume открывает новый epoch, задержка первого callback после resume сохраняется как gap. Stop фиксирует `active_end_frame`, закрывает источники и только после drain передаёт диапазоны в encoding.

### Timeline journal и privacy

Mute/gap/epoch/failure metadata пишется в bounded journal рядом с source manifest:

- `MAX_TIMELINE_EVENT_BYTES = 4096` на событие;
- `MAX_TIMELINE_EVENTS = 4096` на source;
- `MAX_TIMELINE_BYTES = 4 MiB` на source.

Обычные contiguous spans coalesce-ятся; mute, pause epoch, format/device change и failure всегда получают отдельную запись. При переполнении journal source становится terminal incomplete, без silent truncation. Каждое событие содержит `seq`, `known_frame_prefix`, `start_frame`, `end_frame`, `epoch` и kind. Сначала fsync-ится journal event, затем checkpoint payload и manifest reference; recovery принимает только согласованный prefix и сохраняет mute metadata. Старые manifests без journal читаются как legacy contiguous track.

Mute descriptor содержит только span и не содержит исходных samples. Raw сохраняет только unmuted samples, на muted span materializes нули; downstream/VAD не получает muted source values. Gain применяется после raw capture. UI controls не входят в этап 1, но Rust session остаётся будущим source of truth.

Обязательные focused tests этапа 1: oversized/pool recycle, partial fan-out, pause confirmation failure, journal crash ordering, long valid gap, watermark lateness, native frame rational remainder, mute privacy и legacy recovery. Проверки выполняются на exact dirty snapshot с `CARGO_TARGET_DIR=.../target/fork-051`, `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`, `cargo --locked --offline`; native acceptance выполняется отдельно.
