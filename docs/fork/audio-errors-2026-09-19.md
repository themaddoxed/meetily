> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Ошибки проверки записи — 2026-09-19

Продолжение FORK-051/FEAT-071 по запросу пользователя «Продолжай делать и собери ошибки». Это отчёт о проверках и последующих исправлениях, не новый список задач. Ниже сохранена история разных снимков; ранние утверждения о неизменённом коде относятся только к соответствующему этапу. Коммитов, push и публикации не выполнялось.

## Актуальное состояние — 2026-09-20T20:06+03:00

Проверен новый exact `.app` после исправлений жизненного цикла. Сборка относится к
HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`; source fingerprint до и после
упаковки совпал:
`992eb62889622e46722d54d0ef7366b28f60149cdbf5478aed78b809a0fc4a6a`.
Нативная запись сохранена в [final4 evidence](evidence/native-audio-2026-09-20-final4.json);
точный текущий пакет и его граница описаны в [final5 evidence](evidence/native-audio-2026-09-20-final5.json).
Путь приложения:
`target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260920-permission-fast-fallback-final4/meetily.app`.
Артефакт `UNGATED`: `x86_64`, ad-hoc signing, notarization `NOT_RUN`,
`releaseEligible=false`; рабочее дерево изменено. Build executable SHA-256
`e72e6cf5935f1427427b46d4ab47ad38ec9cb02730653dd49e859e9ef568248e`, packaged
executable SHA-256
`2b3158904851ec399b870cf374fdc8b4398d9ab5badfea3eb1439031eefc9fe8`.

Автоматические проверки этого снимка: macOS capture `22 passed`,
RecordingSession `48 passed, 1 ignored`, permissions `7 passed`, frontend policy
и audio surface `25/25`, Playwright permissions `12/12`, release build `PASS`,
`git diff --check` `PASS`. Строгий Clippy остаётся открытым: 13 замечаний
библиотеки и 16 тестовых замечаний.

В нативном прогоне две карточки TCC (микрофон и системный звук) были разрешены
один раз для этой ad-hoc identity. После этого разрешения не сбрасывались и
новый запуск для повторного запроса не делался. Exact `.app` выполнил
`Start → Pause → Resume`; затем stop/exit cleanup вернул управление и закрыл
процесс без зависания. В журнале зафиксировано, что CoreAudio system stream
стал доступен после асинхронного запуска, а затем не выдал ни одного кадра.

Встреча сохранена в
`/Users/developer/Movies/meetily-recordings/Meeting 20_09_26_20_02_13_2026-09-20_17-02`.
`metadata.json` имеет `status=completed`, `error=null`,
`duration_seconds=40.071158257`. Сохранены `audio.mp4`, `microphone.mp4`,
`transcripts.json` и `.tracks/microphone/track.json`; оба MP4 декодируются
FFmpeg. `system.mp4` отсутствует по ожидаемой причине: лог содержит
`System audio capture produced no frames; closing microphone-only`, после чего
финализация помечает System Track необязательным и сохраняет микрофонную запись.

Энергопитание не менялось постоянно: для проверки использовался только
временный `caffeinate`, `pmset` не изменялся. Ошибочные и неполные старые
каталоги сохранены для аудита и не удалялись.

FORK-051 получил реальную проверку `.app` для разрешений, start/pause/resume и
завершения с microphone-only fallback. Он остаётся `in-progress`, потому что
не проверены отключение/возврат устройства и полная матрица отказа TCC.
FEAT-071 продвинут до работающего общего lifecycle и безопасного fallback, но
не принят: системная дорожка не дала контрольных кадров, поэтому mute,
регулировка громкости и временная синхронность двух дорожек не доказаны.
FORK-036 по-прежнему ждёт финальную identity и ключ подписи. FORK-037 имеет
проект миграции, но не реальный upgrade/reopen. FORK-053 не закрыт: dirty source,
ad-hoc Intel bundle и отсутствие release signing/notarization не являются Alpha.

После этого прогона в тестовом helper заменён неограниченный канал на
`sync_channel(1)`, чтобы release-policy surface-check не принимал test-only код
за unbounded transport. Из-за этого изменился только source fingerprint:
`e645918f1723efe145348461a8850c1a7cd66a09d932346ac1067bbaea320998`; release
executable остался тем же SHA-256. Exact текущий пакет перепроверен и упакован
как `run-20260920-permission-fast-fallback-final5/meetily.app`, но нативно снова
не запускался: это сознательно исключило новый запрос TCC. Поэтому final4 —
нативное evidence того же release executable до test-only изменения, а final5 —
точный текущий package с явно отмеченной границей.

## Продолжение после final5 — FEAT-071 source controls

В `.worktrees/feat071-audio-controls` добавлен bounded-контур управления двумя
источниками во время активной записи. Tauri-команда передаёт `muted` и `gain`
для `microphone` или `system` владельцу capture-потока; UI показывает отдельные
кнопки mute и ползунки 0–200%. Значение применяется только к downstream-пути
(распознавание и смешанный live-поток), а raw Track и checkpoint остаются
неизменёнными для последующей обработки. Невалидный gain отклоняется в Rust,
а ответ команды остаётся привязан к текущей сессии.

Для exact текущего source fingerprint
`fc4c9f6d7e090104ffe6de2be1ebcf7be3c83a8c8681f74bbb5fab3ed72904ca` собран новый
пакет [final7 evidence](evidence/native-audio-2026-09-20-final7.json):
`run-20260920-source-controls-final7/meetily.app`. Release executable SHA-256
`0fce7a196a29bf61ac74951b000404a63e53f66dbf4508c0abd4c8586d8f49f5`, packaged
executable SHA-256 `c35ae0a9583e7b965da2baaf1cc388b1fb447bf3d39113152c220aaa97f1be63`.
Rust source-control test `23 passed`, RecordingSession `48 passed, 1 ignored`,
frontend control test `3 passed`, TypeScript, policy surface, release build,
codesign verify и `git diff --check` прошли. Strict Clippy остаётся красным
(13 замечаний библиотеки, 16 тестовых); отдельная команда release-policy по-прежнему
останавливается на существующем ложном `legacy.launch-surface` для FFmpeg в
`incremental_saver.rs`.

Пакет final7 нативно не запускался: это сознательная граница, чтобы не повторять
TCC-запросы. Нативное доказательство продолжает ссылаться на final4: оно
подтверждает Start → Pause → Resume → Stop и microphone-only fallback, но не
подтверждает system.mp4, native mute/volume или синхронность двух источников.
FEAT-071 поэтому продвинута до реализации и автоматических проверок, но не
закрыта без кадров системного источника.

После разбора единственного policy-сбоя найдено точное ложное срабатывание:
сканер не считал безопасной форму `Command::new(&ffmpeg)`, хотя значение
`ffmpeg` получено из `find_ffmpeg_path()`. В reviewed-список добавлено только
это выражение и добавлен регрессионный fixture; запуск Python/FastAPI/Docker
по-прежнему запрещён. Exact package final8 собран из fingerprint
`15dd5521362b372cdc37f79884c15272fefe11ec0685b9b9933a89348c630e39`:
[final8 evidence](evidence/native-audio-2026-09-20-final8.json),
`run-20260920-source-controls-final8/meetily.app`; executable SHA остался
`0fce7a196a29bf61ac74951b000404a63e53f66dbf4508c0abd4c8586d8f49f5`, packaged
SHA — `c35ae0a9583e7b965da2baaf1cc388b1fb447bf3d39113152c220aaa97f1be63`.
Агрегированный Node-набор — `225 passed`, `pnpm run check:policy` — `PASS`,
`git diff --check` — `PASS`. Strict Clippy остаётся красным: 13 библиотечных и
16 тестовых предупреждений. Пакет final8 нативно не запускался, поэтому
границы system.mp4, native mute/volume и физической смены устройства не
изменились.

После точечных Clippy-исправлений для существующих Tauri/API сигнатур,
`MacosCaptureBackend::Default`, сложного notifier-типа и двух тестовых
проверок строгий release Clippy прошёл `PASS`. Exact package final9 собран из
fingerprint `d69e40bde2fcbd6d9e78919a9626f9e45b47d0b52a381abfa4b1368e46a24bd3`:
[final9 evidence](evidence/native-audio-2026-09-20-final9.json),
`run-20260920-source-controls-final9/meetily.app`. Build executable SHA-256
`aa696bd5185da63aa9db124bba3a765b6e1cf802454baac9be22ee8a8b89e325`, packaged
SHA-256 `129825df4bd88d3fbd214ff05cbace0fac3d4688c86d62956ce8ffbe9a89300a`.
Capture `23 passed`, RecordingSession `48 passed, 1 ignored`, frontend unit
`49 passed`, aggregate Node `225 passed`, typecheck, release build, policy,
codesign and diff checks — `PASS`. Пакет final9 нативно не запускался, поэтому
system.mp4, native mute/volume, физическая смена устройства и полная TCC
deny/recovery матрица остаются открытыми.

После исправления гонки terminal-state в TrackWriter exact package final10 собран
из fingerprint `cbf0160ceba3644676ff6bdc2c017ce19901dc4f84d0170e8b792a803a41ff99`:
[final10 evidence](evidence/native-audio-2026-09-20-final10.json),
`run-20260920-source-controls-final10/meetily.app`. Build executable SHA-256
`c7fa712f589dc0a2e84f5cc022e962276cf38be8a5eeb8f63b25666470355324`, packaged
SHA-256 `11ccee268b774e60d5165a548ce61333fc3d19b39b67576e6d08f8b43e338dab`.
Полный release Rust library прогон завершился `433 passed, 0 failed, 5 ignored`;
strict Clippy, policy, typecheck, Vitest, агрегированный Node-набор `225`,
release build и codesign verification — `PASS`. Дополнительный полный
`./scripts/check.sh --full` вне sandbox также завершился `PASS`: frontend Node
`401 passed`, Vitest `49 passed`, critical Playwright `26 passed`, workspace
Clippy, workspace Rust tests и packaged FFmpeg recovery `1 passed` ([full gate
evidence](evidence/full-check-2026-09-20-final10.json)). Повторная проверка
`security find-identity -v -p codesigning` вернула `0 valid identities found`;
пакет всё ещё использует временную legacy identity `com.meetily.ai`. Новый
нативный запуск
не выполнялся, поэтому сохранено runtime-доказательство final4 без нового TCC
запроса; system.mp4, native mute/volume и физические матрицы остаются открытыми.

## Актуальное состояние — 2026-09-20T22:11+03:00

Последний exact `.app` — final10, собранный из рабочей копии
`.worktrees/feat071-audio-controls`, HEAD
`fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`, source fingerprint
`cbf0160ceba3644676ff6bdc2c017ce19901dc4f84d0170e8b792a803a41ff99`.
Машиночитаемая запись сохранена в [final10 evidence JSON](evidence/native-audio-2026-09-20-final10.json).
Рабочее дерево изменено, поэтому артефакт остаётся `UNGATED`: `releaseEligible=false`,
архитектура `x86_64`, ad-hoc signing, notarization не запускалась. Build executable
SHA-256 `c7fa712f589dc0a2e84f5cc022e962276cf38be8a5eeb8f63b25666470355324`,
упакованный executable SHA-256
`11ccee268b774e60d5165a548ce61333fc3d19b39b67576e6d08f8b43e338dab`.
Путь exact приложения:
`target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260920-source-controls-final10/meetily.app`.

Автоматические проверки final10: полный release Rust library `433 passed, 0 failed,
5 ignored`; capture `23 passed`; RecordingSession `48 passed, 1 ignored`; frontend
unit `49 passed`; агрегированный Node `225 passed`; TypeScript, release policy,
strict release Clippy, release build, codesign verify и `git diff --check` — `PASS`.
Пакет не запускался нативно повторно, чтобы не вызывать ещё один TCC prompt;
нативная граница остаётся зафиксированной в final4.

### Сохранённое нативное доказательство final4 exact `.app`

LaunchServices запустил exact `.app` на разблокированном рабочем столе. В
системном `UserNotificationCenter` выданы разрешения на системный звук и
микрофон; после рестарта предупреждение микрофона исчезло, а честное состояние
системного канала осталось `Unknown` с предложением продолжить микрофонную
запись. Нажатие `Start recording` в `15:57:46Z` дало AX-состояние `Recording`
к `15:57:48Z`, без прежнего 90-секундного permission preflight. Pause в AX дал
`Paused`, Resume — `Recording`, Stop вернул главный экран.

Встреча сохранена в
`/Users/developer/Movies/meetily-recordings/Meeting 20_09_26_18_57_50_2026-09-20_15-57`.
`metadata.json`: `status=completed`, `error=null`, duration
`46.501926199`. `audio.mp4` и `microphone.mp4` присутствуют; `ffprobe`
подтвердил AAC 48 kHz mono с длительностью mixed `40.221333s`. Системного
`system.mp4` нет: лог зафиксировал
`System audio capture produced no frames; closing microphone-only`, затем
`Optional System Track has no frames; retaining microphone-only recording`.
Это подтверждает безопасное завершение микрофонной записи при отсутствии
системных кадров, но не подтверждает полноценную вторую системную дорожку.

Энергопитание не переводилось в постоянный пользовательский режим. Для
длинного прогона оставлен временный `caffeinate` на 12 часов; на AC уже стоит
`sleep=0`, а `pmset` не изменялся. После проверки exact процесс Meetily закрыт и
его PID отсутствует.

FORK-051 теперь имеет свежую реальную `.app` проверку start/permission/
pause/resume/stop, но остаётся открытым до физической смены устройства и
полной матрицы отказа/восстановления. FEAT-071 остаётся открытым: fallback
микрофона работает, однако системный tap на этом Mac не дал кадров, поэтому
двухдорожечная native acceptance и временная синхронность не закрыты.
FORK-036 остаётся заблокирован выбором identity и ключом подписи; FORK-037
имеет проект миграции без реального upgrade; FORK-053 не закрыт из-за dirty
source, незакрытых зависимостей и `UNGATED` ad-hoc `.app`.

## Актуальное состояние — 2026-09-20T16:49+03:00

Последняя завершённая нативная проверка относится к диагностической `.app` с source fingerprint `52a49335bb2d883b18fd8e282917046d0525c3f331c26f1ca45c10f931fa0508` и executable SHA256 `bb5391819e7199db81d37af1890eb75d225ada51e01fd569e91bc36d00c5c75d`. HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`, рабочая копия изменена. Это ad-hoc диагностическое приложение, не Alpha. Последующая правка AAC, если выполнена, требует новых доказательств.

| Область | Свежий результат | Оставшаяся граница |
| --- | --- | --- |
| Микрофон, пауза, выход | Новая `.app` через LaunchServices: выдача доступа без перезапуска, pause/resume/Stop; второй запуск — Quit во время записи с сохранением до закрытия БД | Физическое отключение USB не проверено; эффективный отказ доступа в новой копии не подтверждён |
| Зависшее сообщение Recording Failed | В двух свежих нативных циклах не появилось; компонентная регрессия и независимое ревью ранее прошли | Все гонки повторного запуска не заявлены |
| Ложный Available системного звука | Честное Unknown видно до и во время записи; Try Again/Refresh отключены во время capture | Во втором запуске UI настроек показывает0, но system.mp4 содержит сигнал; применённый к этой копии отказ NOT VERIFIED, обход защиты не утверждается |
| Время raw system | На новом app маркеры system2.2/25.2s против mixed24.2/47.3s: расхождение около22s | Общая шкала не реализована; production mixer остаётся прежним |
| Стыки AAC checkpoints | Новый native снова даёт DTS warnings mixed/mic; временный PCM→один AAC encoder эксперимент проверен | Начат ограниченный перенос в normal/recovery; пока не принят |
| Автоматические проверки | Совместный Rust531 PASS/10ignored на52a49335; frontend400 Node+49 Vitest+38 browser PASS; production/release build PASS; scoped reviews PASS | Строгий Clippy FAIL13 library /16 lib-test (пересекаются); это не release gate |

FORK-051 открыт. Единое управление FEAT-071 ещё не готово. FORK-036 зависит от отложенного FORK-028 и ключа подписи; updater выключен, при последней проверке Developer ID identities0. FORK-037 имеет проверенный параметризованный проект, окончательные пути/upgrade не проверены. FORK-053 не имеет закрытой цепочки зависимостей и чистого source commit.

Защита от idle/display sleep активна через caffeinate (12h с10:28MSK); автоматический smoke также запускает собственную защиту на срок своего процесса. Постоянное изменение pmset не подтверждено. После второго native теста возврат системного разрешения потребовал пароль администратора. Свежий readback Settings после подтверждения показывает `meetily` в разделе системного звука с checkbox `1`; восстановление этого разрешения подтверждено. Meetily штатно закрыта. Readback сохранён в `round2-restore-latest.ax.txt` (SHA-256 `75a5322c655fa2c35789f34820529ddc5ace50a9d677c3c4f336e4966fda4886`).

### Свежая нативная проверка 52a49335

Доказательства: [первый цикл](evidence/native-audio-2026-09-19/round2-native-result.json), [контрольные суммы](evidence/native-audio-2026-09-19/round2-native-sha256.json), [второй цикл](evidence/native-audio-2026-09-19/round2-denied-result.json), [его контрольные суммы](evidence/native-audio-2026-09-19/round2-denied-sha256.json). Упаковка/подпись: `round2-native-provenance.json`, `audio-round2-package.log` в том же каталоге.

Первый цикл PID3630: микрофон/системный prompt Allow, после второго prompt микрофонное предупреждение исчезло. Sidebar Start Recording уже запустил запись; последующий поиск нижней кнопки вернул MATCHES0 и не был засчитан как нажатие. AX показывает00:16 → pause00:49 в двух отдельных чтениях → resume01:07 → Stop. Интервал двух paused снимков не встроен в файлы; длительность паузы по ним не утверждается. Сохранена встреча `0b3b690b-44ad-4175-a689-3584c0fd10b6`, session `4638991d-702b-462c-b493-385a1b0045d1`, metadata completed. Mixed69.09875s, microphone69.312s, system47.381375s после decode8k; metadata91.2869s включает паузу. Две DTS warnings mixed/mic сохраняются. Нормальный Quit после Stop, процесс отсутствует. Независимый reviewer сверил executable, JSON/AX/log и реальные MP4 SHA; bounded сценарий согласован, полного task acceptance нет.

Второй цикл PID4943: UI Settings meetily system-only0, после Start Unknown остаётся. Во время активной записи выполнен Quit; финализация MP4 предшествует DB cleanup. Встреча `716d1a6b-936a-4a2c-9d7e-b17c3c77a458`, session `b3bc46f8-69ff-483b-9c29-25ab5f268120`, completed62.9498s. Декодирование всех трёх MP4 exit0 без warnings; system56.576s, peak0.15315, rms0.02028 — сигнал есть. Поэтому название артефакта denied означает намерение сценария, **не доказанный системный отказ**. Причина установлена на уровне identity: диагностическая ad-hoc копия и `/Applications/Meetily.app` имеют разные code requirement/cdhash при одном bundle ID; Settings row не доказывает, какая копия получила изменение. Приватная TCC база не читалась и не сбрасывалась. После ручного системного подтверждения restore readback стал `1`.

## Исходный проверенный снимок — историческая часть

`.worktrees/feat071-audio-controls`, HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c` с существующими изменениями FORK-051. Source fingerprint до/после: `78376351ebf6698fa84f86e8757a58a1e5c5d6d5171b825470ae1d8264c59661`. Среда: Intel macOS 15.8, Rust/Cargo 1.98.1. Использован тестовый executable `target/fork-051/debug/deps/app_lib-6172355495115924`, собранный из этого снимка в предшествующем аппаратном прогоне.

Пути исходников ниже относятся к этой рабочей копии. Доказательства хранятся в `/private/tmp/meetily-urgent-20260919/`.

## Подтверждённые проблемы

### 1. Аудиоскрипт засчитывает отсутствующий тест

`scripts/smoke-macos-audio.sh:112–115` запускает точный фильтр `audio::system_audio_commands::tests::macos_system_audio_devices_have_valid_names`. В текущем тестовом executable такого теста нет. Rust возвращает exit 0 при `0 tests, 400 filtered out`, после чего скрипт печатает `PASS: real macOS system audio device enumeration` и повторяет общий PASS ниже.

**Воспроизведено:** реальный аппаратный запуск из предыдущего этапа; журнал `native-audio.log`, SHA-256 `b0b6ce73ba9b2eb0d94f365dd44425204aa4bfd438ea0c2377ed5f29a2a1eaab`. Список тестов дополнительно сохранён в `rust-test-list.txt`.

**Влияние:** ложное доказательство проверки устройств. Успех отдельного CoreAudio capture test остаётся действительным, но не исправляет эту ошибку. Исправление должно учитывать наличие и фактическое выполнение нужного теста, а не только код возврата процесса. Простое удаление проверки не подтверждает перечисление устройств.

### 2. Два теста остановки запускают неподготовленный канал обработки звука

Стабильно падают при общем и отдельных запусках:

- `wrong_session_stop_preserves_active_owner_then_a_stops_once`, `recording_session.rs:3903`;
- `shutdown_waits_for_an_in_flight_finalizer_before_reporting_safe_exit`, `recording_session.rs:3940`.

Оба получают `PersistenceFailed: Recording did not stop cleanly`, detail `microphone downstream drain: Audio pipeline not ready - no sender available`.

**Причина в тестовой подготовке:** `install_active_fake_session` создаёт `RecordingState` и вызывает `start_recording`, но не подключает sender (`recording_session.rs:3777–3780`). Затем запускает настоящий `DownstreamWriter` (`:3803`), а имитация устройства отправляет ему звук (`:2703`). `RecordingState::send_audio_chunk` возвращает указанную ошибку, когда sender отсутствует (`recording_state.rs:276–295`). В обычном коде sender назначается при старте pipeline (`pipeline.rs:1082`). Поэтому эти падения не доказывают аналогичный сбой реального stop в `.app`.

**Влияние:** тесты не могут подтвердить правильность нормальной остановки и ожидания финализации. Нельзя ослаблять продуктовую обработку ошибки ради их прохождения; нужно подготовить тестовый канал и сохранить проверки окончания записи.

### 3. Тест паузы ожидает событие, которое его имитация не регистрирует

`control_failure_stops_and_joins_before_returning_session_error` стабильно падает на `recording_session.rs:3976`. Проверяется последовательность `pause → stop → drop`, но `OrderedFakeBackend::pause` (`:2714–2720`) возвращает результат без записи `pause` в журнал событий. При этом `start`, `stop`, `drop` записываются.

**Воспроизведено:** общий прогон и отдельный запуск, оба exit 101. Исправление должно восстановить наблюдение события в имитации и сохранить проверку порядка. Это подтверждённый дефект тестовой проверки, не доказательство нарушения порядка в приложении.

### 4. Проверка качества кода останавливается на трёх неиспользуемых аргументах

`frontend/src/hooks/useRecordingStateSync.ts:11–13`: `_isRecording`, `_setIsRecording`, `_setIsMeetingActive` нарушают `@typescript-eslint/no-unused-vars`. Целевой ESLint завершился exit 1, три ошибки. Файл совпадает с HEAD — это существующая проблема, не изменение данного продолжения. `RecordingControls.tsx`, `usePermissionCheck.ts`, `PermissionWarning.tsx` проверены той же командой без дополнительных ошибок.

### 5. Тест независимости загрузок нестабилен в полном прогоне

`model_download::tests::same_model_is_serialized_but_two_models_are_independent` упал вне sandbox на `model_download.rs:2103`, assertion `one_result.is_ok() || two_result.is_ok()`. При отдельном повторе на том же executable и вне sandbox прошёл за 0.02s.

**Статус:** наблюдён сбой полного прогона, отдельно не воспроизведён. Причина — нагрузка, тестовая гонка или продуктовый дефект — не установлена. Отдельный успешный повтор не превращает полный прогон в успешный.

## Риски, найденные по исходникам, без runtime-воспроизведения

Независимый агент `audio_error_audit` прочитал audio/session/capture на том же HEAD и вернул source-level findings. Контроллер отделил их от подтверждённых падений.

**Время расшифровки после паузы.** `DownstreamWriter` использует `started.elapsed()` (`recording_session.rs:650`), хотя длительность сессии отдельно исключает паузы (`:2568–2578`). `transcription/worker.rs:206–275` использует эту временную метку для синхронизации с аудио. Возможный результат: после минутной паузы текст указывает примерно на минуту дальше, чем соответствующий фрагмент сохранённого аудио. Нужна проверка реального преобразования времени и детерминированный тест паузы; новый тест в этом продолжении не создан.

**Повторное подключение микрофона с тем же именем.** `capture/macos.rs:440–452` отбрасывает устройство с тем же отображаемым именем, что и потерянное. Поведение закреплено существующим `microphone_recovery_rejects_the_same_device_name`. Это может мешать повторному подключению физически работоспособного микрофона с прежним именем. Аппаратное переподключение не выполнено; требование отличить живой endpoint от нерабочего нельзя заменить одним сравнением имени.

**Не включено в подтверждённые дефекты:** агент также отметил, что `RecordingSession` не вызывает pause/resume на `RecordingState`. Само по себе прохождение уже записанных до паузы фрагментов через очередь после pause не доказывает ошибку: отбрасывание таких фрагментов может потерять звук. Сначала нужно определить границу паузы и сверить сохранённые дорожки с обработкой. Автоматическое добавление сброса очереди не предлагается.

## Результаты запусков

| Проверка | Результат |
| --- | --- |
| RecordingSession и его state, целевой запуск | 43 passed, 3 failed, exit 101 |
| Каждый из трёх упавших тестов отдельно | Каждый снова failed, exit 101 |
| Разрешения | 4 passed, exit 0 |
| Capture без opt-in аппаратных тестов | 43 passed, 1 ignored, exit 0 |
| Полная библиотека в sandbox | 381 passed, 16 failed, 3 ignored; часть падений — запрет локальных портов |
| Полная библиотека вне sandbox | 393 passed, 4 failed, 3 ignored, exit 101 |
| Упавший тест загрузки отдельно вне sandbox | 1 passed, exit 0 |
| Целевой ESLint четырёх файлов | 3 ошибки в одном файле, exit 1 |
| Source fingerprint и `git diff --check` после проверок | Не изменён; PASS |

Полные Rust-прогоны использовали существующий executable с `--nocapture`, целевые — его фильтры `audio::recording_session`, `audio::permissions`, `audio::capture`; одиночные — полные имена плюс `--exact --nocapture`. Аппаратные ignored-тесты повторно не включались. Паника искусственно падающего backend внутри успешно пройденного теста не включена в число дефектов.

## Нативное окно и микрофон

Повторная проверка после старта смогла получить AXWindow и дерево WebView, чего не удавалось на предыдущем этапе. Получен снимок только окна (`meetily-window-recheck.png`); на нём центральная область тёмная, тогда как AX-дерево содержит `Microphone Permission Required`, `Enable Microphone`, `Audio Capture`, `Try Again`. Причина расхождения картинки и дерева не установлена; оно не объявляется доказанной ошибкой отрисовки.

Найдена ровно одна кнопка `Enable Microphone`; `AXPress` вернул 0 (`enable-microphone.txt`). После нажатия чтение окна снова вернуло зацикленное дерево, а снимок полного экрана не получен. Принятие/отказ системного запроса не подтверждены. Сброс TCC, нажатие невидимых кнопок, принудительное завершение и правка пользовательских данных не выполнялись. Start/stop/pause/resume в реальном `.app` остаются NOT VERIFIED.

Последнее чтение `NSWorkspace.frontmostApplication` показало `loginwindow`; в списке окон также присутствует `UserNotificationCenter` (PID 95411, окно 274). Журнал `native-window-state.txt`. Это указывает на контекст входа/блокировки macOS и не позволяет считать недоступность UI дефектом Meetily. Содержимое системного окна не прочитано, попыток разблокировки или ввода учётных данных не было.

## Контрольные суммы ключевых журналов

- `recording-session.log`: `18a3364ff86fa80791fdd07c969f68927c55a0a9e0e4d954ec87ce8d0a38ccb3`.
- `rust-library-unrestricted.log`: `0b1b29b142b623ed70f04345f0b6374a580401c2f5826f243db2ec11399f5e5f`.
- `model-download-isolated.log`: `fe73159a2b683a5de81e73307fbbc92db5b6b510fa977c956f1aa6f0f7b1b2b3`.
- `audio-eslint.log`: `78c9ec0c6d147295277910a0461f4d3f80cfcb9f56c986651f2bd67565e9aaa6`.

FORK-051 не закрыт, FEAT-071 не начат, готовность Alpha не заявляется. Приоритет дальнейшего исправления: ложное доказательство аудиопроверки; неисправные тестовые сценарии остановки/паузы; затем воспроизведение расхождения времени после паузы и восстановления микрофона.

## Нативное продолжение после сообщения «Готово»

Рабочий стол снова доступен. Приложение то же: PID 60340, executable SHA-256 `8411ee85cf18693540cb83246f3010edc137f4f3b1b5a4bbb5ec58c675c0228b`, source fingerprint не изменён. После обновления статуса предупреждение микрофона отсутствовало; после `Try Again` исчезло предупреждение системного аудио. Это подтверждает состояние UI, но не воспроизводит первоначальный системный диалог отказа/выдачи разрешения.

Автоматический контроль безопасности отклонил первое предложение нажать кнопку с пустым названием. Команда не выполнена. После отдельного чтения AX-положений и снимка кнопка записи однозначно сопоставлена с красным микрофоном: (1087,724), 49×48; последующие разрешённые действия проверяли роль, точное положение, размер и единственность элемента. Кнопки паузы/продолжения: (949,732), 41×40; Stop: (997,732), 41×40. При несовпадении helper возвращал `MATCHES=0` без нажатия.

### Реальная ошибка завершения первого тестового цикла

Встреча `Meeting 19_09_26_21_44_27`, ID `3cd2a41a-022b-408d-8863-1ef6cb9fee38`, session `57e671b0-54b3-4c3f-85c2-f99f37580f6c`:

- Старт наблюдался в UI, контрольная синтезированная фраза распознана.
- До попытки паузы интерфейс уже вернулся к состоянию без записи. Helper паузы вернул `MATCHES=0`; значит, агент паузу в этом цикле не нажал.
- `metadata.json` содержит `status: interrupted`, duration `32.554550995`.
- Из отдельной копии SQLite/WAL/SHM прочитана только эта встреча: status `error`, lifecycle_error `Recording did not stop cleanly`, operation `recording`. Хеши файлов исходной БД до и после копирования совпали; оригинальная БД не открывалась SQLite-клиентом.
- `audio.mp4`, `microphone.mp4`, `system.mp4` и исходные checkpoints сохранились. Причина остановки и участие пользователя пока не установлены; задан уточняющий вопрос. Конкретный дефект автоматической остановки на 32-й секунде не объявляется воспроизведённым только на этом основании.

Ранее появлялось окно восстановления встречи `Meeting 19_09_26_21_41_58` с одной расшифровкой и надписью `No audio`. Она тоже имеет отдельный каталог и metadata со статусом `failed`. Наличие этой надписи не трактуется как доказательство уничтожения исходных дорожек. Встречи не удалялись и не восстанавливались вручную.

### Второй цикл: успешная пауза, продолжение и обычная остановка

Встреча `Meeting 19_09_26_21_46_57`, ID `fb6a0d93-9adb-4e68-84ad-6cd7eb7f88fc`, session `50df7543-9ec4-43b4-8b58-2d8ba704800e`:

1. Новая запись стартовала после предыдущей остановки: `Recording 00:00`.
2. Идентифицированная кнопка Pause нажата: `Paused 00:02`. При повторном чтении спустя паузу таймер остался `00:02`.
3. Та же кнопка в состоянии Play нажата: `Recording`. Воспроизведена фраза `Second section after the pause. Four five six.`
4. Идентифицированная кнопка Stop нажата, затем наблюдалась `Finalizing transcription...`.
5. Метаданные завершённой встречи содержат `status: completed`. Сохранились расшифровка, общий файл и обе исходные дорожки. UI показывает `[00:02] Second section after the pause. 456.`

Наблюдение `[00:02]` после паузы **не воспроизводит** ранее найденный по исходникам риск сдвига transcript timestamp. Риск остаётся для точечного теста, но не объявляется подтверждённым этим сценарием.

Артефакты: `/Users/developer/Movies/meetily-recordings/Meeting 19_09_26_21_46_57_2026-09-19_18-46/`.

| Файл | Длительность ffprobe | Средний / пиковый уровень |
| --- | --- | --- |
| `audio.mp4` | 7.821333s | -24.1 / -3.5 dB |
| `microphone.mp4` | 8.330000s | -54.1 / -34.8 dB |
| `system.mp4` | 5.642000s | -22.6 / -3.5 dB |

Все три файла декодируются без ошибки и не являются цифровой тишиной. Контрольная речь воспроизводилась через системный вывод; отдельное качество человеческой речи именно с микрофона этим не подтверждается. В метаданных `duration_seconds=40.63466147`: это полное время с паузой (`snapshot.recording_duration`), тогда как UI использует активное время. Расхождение подтверждено; перед исправлением нужно явно закрепить смысл поля длительности, а не менять его без проверки потребителей.

Проверенные изображения, AX-логи, измерения и SHA-256 сохранены в [каталоге доказательств](evidence/native-audio-2026-09-19/sha256.json). Исходные файлы аудио не редактировались. Подтверждён один успешный нативный цикл start/pause/resume/stop и повторный старт; FORK-051 целиком остаётся открыт из-за ошибки первого завершения, непроверенных отказа/возврата разрешений, переподключения устройств и выхода/повторного открытия.

## Повтор после разблокировки и первые исправления

Проверяемая прежняя `.app` штатно закрыта и снова открыта с локальным журналом. Процесс 29819, окно 382, тот же executable SHA-256 `8411ee85cf18693540cb83246f3010edc137f4f3b1b5a4bbb5ec58c675c0228b`. Кнопкой Cancel закрыт диалог восстановления без изменения встреч; Try Again восстановил доступ системного звука в UI.

Новая встреча `d748243f-5b6d-46fb-8d67-3b36f00defde`, session `6fd69091-7e58-4338-9304-39efc1ff5039`, начата через проверенную кнопку записи. Stop/Pause в этом цикле агент не нажимал. После двух синтезированных фраз запись снова завершилась: metadata содержит `interrupted`, длительность `32.597363178`. Журнал: старт capture около 20:34:16 UTC, закрытие CoreAudio 20:34:47 UTC, сохранение первого 30-секундного mixed checkpoint 20:34:48 UTC. Это воспроизводит прежний сценарий короткой прерванной записи. Конкретная причина ещё не выведена старой сборкой; связь с переполнением очереди при кодировании checkpoint проверяется, но пока не считается доказанной.

Дополнительно подтверждено по исходникам: окно восстановления проверяет только `.checkpoints/*.mp4`, не учитывает уже сохранённый `audio.mp4` и `.tracks`. Поэтому надпись No audio не доказывает отсутствие аудио. В новом тесте итоговые файлы сохранены. Отдельно журнал старта сообщает, что выбранная модель итогов Qwen3.5-4B имеет 2533066630 байт вместо ожидаемых 2740937888. Файл не изменялся.

В рабочей копии исправлен test fixture RecordingSession: реальный downstream передаёт захваченные фрагменты в saver и закрывается при stop; прежняя несвязанная подстановка audio удалена. Fake backend теперь фиксирует событие pause. Все 40 тестов `audio::recording_session::tests` прошли. Убраны неиспользуемые аргументы useRecordingStateSync и ненужное получение setter из Sidebar. ESLint этих файлов, typecheck, 397 Node-тестов, 48 Vitest-тестов и policy прошли.

Полная Rust-библиотека после этих исправлений: 396 passed, 1 failed, 3 ignored. Новое падение — `resumed_bundle_progress_includes_each_artifact_partial`: тестовый HTTP server получил неожиданный запрос. Ранее падавший same-model тест в этом прогоне прошёл. Повтор всей группы model_download (15 тестов) дал ещё два падения, поэтому проблема не объявляется устранённой одиночным зелёным повтором. В assertions добавлена диагностика фактической ошибки/запроса.

Сейчас собирается отдельная диагностическая версия с логом причины неожиданного завершения capture. Она не является release-кандидатом; прежняя `.app` и пользовательские записи не заменяются. FORK-051 остаётся открыт.

### Диагностическая сборка для точной причины остановки

Собран отдельный Intel `.app`: `target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260919-audio-diagnostics/meetily.app` в product worktree. Source fingerprint до/после сборки совпал: `3a2964f5925758f294b0e3a63311f3f4dc0bcad11e029f1867ed52baaefa758e`. SHA-256 executable: `46c9e106d3997a257fc1fced288eedc22c0a65281dc5469d8b84b088740c795f`. `codesign --verify --deep --strict` прошёл; подпись ad hoc, releaseEligible=false, notarization NOT_RUN. Прежняя сборка сохранена.

Воспроизводимая команда использовала `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` и **абсолютный** `CARGO_TARGET_DIR=<product worktree>/target/fork-051`, `cargo build -p meetily --bin meetily --release --features tauri/custom-protocol --locked --offline`, с разрешённым доступом к локальному кэшу Xcode. Относительный target путь ломал разрешение каталога в build-script Sherpa; активный CommandLineTools не подходил для cidre; sandbox блокировал запись Xcode DerivedData. Это ошибки запуска сборки, не доказанные дефекты Meetily.

Приложение запущено обычным `open` с stdout/stderr в локальные файлы `app-diagnostic.*.log`. PID 44395. В этот момент macOS снова перешла к loginwindow; новый цикл записи пока не начат, запрошена разблокировка. Временный `caffeinate -d -i -t 600` ограничен десятью минутами, системные настройки и пароль не менялись.

Независимый review `review_audio_repairs` принял repair 1 скрипта smoke и enumeration на fingerprint выше: output-only проверка добавлена, input-only список отклоняется; Node 4/4. Rust/hardware исполнение этой новой проверки пока отдельно ожидается. Браузерная матрица разрешений: 11/11 PASS. Новые журналы и снимок третьего цикла сохранены рядом с прежними доказательствами, контрольные суммы — `evidence/native-audio-2026-09-19/continuation-sha256.json`.

### Завершённые автоматические проверки после исправлений

Текущий source fingerprint: `1ad163fc2268361742e2b6b308150ff33940109fbc82398c1825d09c43d4f294`, HEAD по-прежнему `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`, рабочая копия содержит изменения. Это более новый тестовый снимок, чем диагностическая `.app` выше: изменена только fixture локального HTTP-сервера в `model_download.rs`.

Причина нестабильности загрузочных тестов установлена: принятый тестовым сервером TcpStream оставался неблокирующим. Чтение до прихода HTTP-заголовков попадало в Err → break; handler получал пустую строку. Это подтверждено журналами `model-download-recheck-12.log` и `-22.log` (точный request == ""). Добавлено `stream.set_nonblocking(false)` только в тестовый сервер; продуктовые SHA-256/size/resume-проверки сохранены. До исправления 6 из 30 повторов падали; после исправления focused group 15/15 и 10 последовательных повторов прошли. Main проверил узкий diff и хеши автора; отдельный вызов reviewer для этой строки не состоялся из-за лимита агентских задач. Независимый review остальных исправлений описан выше.

Финальные результаты этого этапа:

| Проверка | Результат |
| --- | --- |
| Полная Rust-библиотека вне sandbox | 398 passed, 0 failed, 4 ignored; exit 0, 130.32s |
| Полный Node-набор на обновлённых исходниках | 398 passed, 0 failed; exit 0 |
| Проверка input-only списка | 1 passed; exit 0 |
| Реальный ignored enumeration output-устройств | 1 passed, 0.14s |
| Реальный CoreAudio с контрольным тоном | 1 passed, 1.16s |
| Обновлённый аппаратный smoke целиком | exit 2, OVERALL NOT_RUN: отдельная микрофонная/UI-матрица ещё не выполнена |
| Browser permissions matrix | 11 passed |
| Frontend production build | PASS |

Новый smoke использует временную копию диагностического bundle с тестовым executable, поэтому оба аппаратных PASS — дополнительные доказательства системного звука, а не приемка интерфейса `.app`. SHA-256 Rust test binary: `26934224889872507e7237c163aa149073feb88a8e7571c6a1a08700a6cc9c69`.

Главная продуктовая ошибка остаётся открытой: реальная запись прерывается через ~32s. Диагностическая `.app` запущена, но её новый цикл пока не выполнен из-за loginwindow. Также остаются неверное No audio при наличии сохранённых дорожек и несовпадение размера выбранного файла модели итогов. FORK-051 не закрыт; FEAT-071 остаётся за его приёмкой. FORK-036/FORK-037 по-прежнему зависят от отложенного FORK-028, FORK-053 не имеет закрытой цепочки зависимостей и exact-commit release evidence. Коммиты, push и публикация не выполнялись.

### 20 сентября: воспроизведение переполнения и ограниченное исправление

Добавлена отдельная 35-секундная проверка реального TrackWriter с локальным FFmpeg и временными файлами: поток stereo 48 kHz, 480 frames каждые 10 ms. Микрофон и пользовательские записи тест не открывает. С прежней очередью на 64 сообщения тест завершился `TrackBackpressure(Microphone)` на 30.640313344s: во время синхронного кодирования первого 30-секундного checkpoint потребитель временно не освобождает очередь. Это подтверждённый дефект данного пути сохранения; привязка именно к прежнему нативному обрыву требует нового `.app`-цикла.

Для очередей исходных дорожек микрофона и системного звука добавлен отдельный предел 4096 сообщений. Ограничение полезных аудиоданных в очереди 8 MiB сохранено; producer остаётся неблокирующим и при реальной перегрузке возвращает явную ошибку. Очередь downstream остаётся на 64 сообщениях. Новый обычный тест проверяет накопление трёх секунд коротких callback без потери порядка и отказ при превышении byte budget. Это запас на временное кодирование, а не обещание выдержать любую задержку диска.

Снимок: HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`, dirty source fingerprint `16f0abdda7ffb646652605b1f69a677a8f7a9f529d41afd960932029698012a0`. `backend.rs` SHA-256 `01e7cbdc056d4d6eaed370edcbb0fc6a8b359956c59cf628e948da4f86ca61ba`, `recording_session.rs` SHA-256 `a72b00931553e460e3798d286f8bf88e22078f3761f711804c5ee0ba2dfdaef0`.

Проверки исправленного снимка: `audio::` — 203 passed, 0 failed, 5 ignored; явный запуск `audio::recording_session::tests::live_track_writer_survives_first_checkpoint --ignored --exact --nocapture` — 1 passed, 35.66s. Тест проверил, что сохранены все принятые samples. Журналы и SHA-256: `evidence/native-audio-2026-09-19/checkpoint-sha256.json`. Полный набор 398/398 выше относится к предыдущему снимку; не переносится на новый как свежий результат.

Попытка отдельного review после сверки существующих agent handles отклонена инфраструктурой: `agent thread limit reached`; независимый review этой новой правки не выполнен. Main прочитал ограничение памяти, обработку отказа и wiring обеих raw-дорожек. Собирается отдельный диагностический `.app`; при повторной проверке после сообщения о разблокировке frontmost всё ещё `loginwindow`, screencapture сообщает `could not create image from display`. Нативная приёмка исправления пока NOT VERIFIED, FORK-051 остаётся открытой.

Продолжение того же снимка: полная Rust-библиотека **399 passed, 0 failed, 5 ignored**, exit 0, 134.40s. Отдельный ignored realtime тест выше исполнен явно и прошёл. Release build PASS, 3m19s. Новый bundle: `target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260920-checkpoint-fixed/meetily.app`; executable SHA-256 `f272f3866349ef2f56d14c25c361c463fdc2dabace247fd8e887e307d03384a7`. Source fingerprint до/после совпал. `codesign --verify --deep --strict` PASS; ad hoc, notarization NOT_RUN, releaseEligible=false. Сохранена копия provenance и журналы сборки/полных тестов в checkpoint-sha256.json.

Предыдущая диагностическая копия штатно завершена через NSRunningApplication.terminate; отсутствие процесса подтверждено. Исправленная копия запущена обычным open, PID 51825, путь процесса совпадает с новым bundle. При свежей проверке frontmost всё ещё loginwindow. Запись на новой копии не запускалась. Старт повторно подтвердил предупреждение размера qwen3.5:4b (2533066630 вместо 2740937888); модель не изменялась. Нативная запись >65s, pause/resume/stop и повторное открытие остаются NOT VERIFIED.

### 20 сентября: готовое аудио после удаления checkpoints

Отдельный тест создал реальную запись через IncrementalAudioSaver, завершил её, подтвердил удаление `.checkpoints` и повторно запросил наличие аудио. До изменения `has_audio_checkpoints` вернул false: baseline упал на соответствующем assertion. Первый запуск fixture не создавал обязательный `.checkpoints` до saver.new; эта ошибка тестовой подготовки исправлена и не засчитывается как продуктовая. Точный product baseline сохранён отдельно в recovery-final-baseline-exact.log.

В `incremental_saver.rs` обе команды — проверка наличия аудио и восстановление — теперь используют общую проверку существующего `audio.mp4`: обычный непустой файл, потоковое декодирование с ограниченным буфером, ненулевой результат, одинаковый SHA-256 до/после чтения. Проверка вынесена с async-потока через spawn_blocking. Готовый файл не переписывается. При отсутствии legacy MP4-checkpoints возвращается success с тем же путём; при оставшихся checkpoints возвращается partial, чтобы существующий frontend не удалил фрагменты и не закрыл pending recovery. Пустой/повреждённый/необычный файл вызывает ошибку с сохранением исходных данных. Длительность не выдумывается из частоты, переданной вызывающей стороной: для этого legacy пути она остаётся неизвестной (0).

Три регрессии проверяют повторное открытие завершённого файла дважды без изменения байтов; сохранение отличающегося дополнительного checkpoint при partial; сохранность пустого/повреждённого final и пригодных checkpoints при отказе. Группа incremental_saver: 17 passed, 0 failed; frontend recovery hook: 16 passed, 0 failed. Форматирование затронуло только новые блоки. Текущий файл SHA-256 `a1e613ba6bc468c1b86ed419084a13ef04f4869c02c40d1d83dd16cd4cb37dd9`; source fingerprint `47f62c3f0ea6beda0af770b8a8d781c212212123fbd8781dd7ca8c8da90e8e33`. Полный Rust-прогон этого снимка ещё выполняется.

Ограничения: существующий final вместе с checkpoints пока не доказывается полным автоматически — остаётся partial; восстановление только из `.tracks` этой правкой не подключено. Декодируемость legacy final не доказывает точное исходное число samples без отдельного manifest. Исправление ещё не включено в запущенную checkpoint-fixed `.app`, независимая оценка текущего diff запрошена у работающего агента. FORK-051 и весь исходный список задач не закрываются этим частным исправлением.

Независимая read-only оценка агента fix_audio_smoke на SHA файла `a1e613ba...` подтвердила контракт основного исправления; агент отдельно отметил риск прежнего rename: файл мог появиться после начальной проверки и быть заменён. Main устранил эту гонку без ненадёжной пары exists+rename: новый helper публикует файл через hard_link в том же каталоге (ошибка при существующем destination), удаляет временное имя и синхронизирует каталог. При отсутствии поддержки hard link операция завершается явной ошибкой, без fallback на перезаписывающий rename. Добавлены тест отказа при уже появившемся destination с сохранением обоих файлов и полный обычный путь legacy checkpoint → recovery success → cleanup hook → reopen success.

Предыдущий снимок `47f62c3...` прошёл полный Rust-прогон: 402 passed, 0 failed, 5 ignored, 127.91s. Текущий дополнительный снимок: `incremental_saver.rs` SHA-256 `b7f3e4b12b563cb94c7ce99495075cb222c7a81dfe7a7e5dc259a6ef897141aa`, source fingerprint `4ec41cb8751e0ed2e6e90496fa00d1e1ea6c6f32335fa8aa45a4c420d24f1247`; расширенная группа сохранения 19/19 PASS. Его полный прогон и независимая повторная оценка ещё выполняются.

Финальный полный Rust-прогон снимка `4ec41cb...`: **404 passed, 0 failed, 5 ignored**, exit 0, 127.82s. Повторная read-only оценка fix_audio_smoke на точных SHA трёх изменённых файлов не нашла blocker: публикация с hard_link без overwrite fallback и обе raw-очереди с сохранённым пределом 8 MiB проверены. Агент отдельно прочитал baseline/fixed журналы checkpoint теста; native gates его оценкой не закрываются. В этой стадии source не менялся после проверки.

Дополнительно уточнена причина недоступности UI: свежий `CGSessionCopyCurrentDictionary` вернул `CGSSessionScreenIsLocked=1`, `kCGSessionLoginDoneKey=1`; `CGDisplayIsActive=0`, `CGPreflightScreenCaptureAccess=true`, frontmost=loginwindow. Это подтверждённая блокировка/неактивный экран при имеющемся разрешении снимков.

Проверка модели итогов стала точнее: локальный Qwen3.5-4B-Q4_K_M.gguf имеет SHA-256 `bfbdf9a0e304f9a40247954e0632f9c54142e34af9cc2bc8d6f5598f8467fd14`, отличный от закреплённого `00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4`. [Официальный pointer закреплённой ревизии](https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/raw/e87f176479d0855a907a41277aca2f8ee7a09523/Qwen3.5-4B-Q4_K_M.gguf) подтверждает именно ожидаемые SHA и размер 2740937888. Следовательно, менять manifest под имеющийся файл нельзя: локальный файл не соответствует закреплённому артефакту. Это не устанавливает причину расхождения (неполная загрузка или другой файл). Ни файл модели, ни пользовательские настройки не изменялись; полная модель из сети не скачивалась.

Собран отдельный общий диагностический bundle `target/diagnostic-dirty/fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c/run-20260920-audio-recovery-fixed/meetily.app`. Release build PASS, 2m59s; source fingerprint `4ec41cb8751e0ed2e6e90496fa00d1e1ea6c6f32335fa8aa45a4c420d24f1247` до/после совпал. Executable SHA-256 `012117516f6f225feb98f8a9df49a3eeb820196998721a2718475684a858b7b1`, codesign verify PASS, подпись ad hoc, notarization NOT_RUN, releaseEligible=false. Предыдущая копия PID 51825 штатно завершена, отсутствие процесса подтверждено; новая открыта через open с отдельными app-recovery-fixed журналами. Запись через её UI и отображение восстановления остаются NOT VERIFIED до разблокировки.

## Сверка оставшейся очереди 20 сентября

Product source fingerprint по-прежнему `4ec41cb8751e0ed2e6e90496fa00d1e1ea6c6f32335fa8aa45a4c420d24f1247`, HEAD `fcfa6e0...`, рабочая копия содержит изменения. Запущена общая диагностическая `.app`, PID 58404; свежая проверка возвращает locked=1 и displayActive=0. После сообщения пользователя о разблокировке этот внешний блокер подтверждён в трёх последовательных goal turns; в ожидании были закончены исправления очереди, восстановления и уточнение migration design. Выполняющихся сборок/тестов нет, последние запрошенные агентские проверки завершены.

| Задача | Достигнуто и граница | Что необходимо дальше |
| --- | --- | --- |
| FORK-051 | Исправления, 404 Rust PASS, focused recovery/queue evidence, отдельный `.app` и независимая оценка | Доступный рабочий стол; реальный >65s цикл, pause/resume/stop/relaunch, матрица разрешений и устройств на данном артефакте |
| FEAT-071 | Требования прочитаны; задача явно зависит от FORK-051 | Сначала завершить приёмку FORK-051; реализация unified source controls не объявляется выполненной |
| FORK-036 | Updater остаётся выключенным; FORK-028 явно блокирует новый канал | Выбранная окончательная identity и принадлежащие продукту канал/ключ подписи; не подставлять legacy identity как финальную |
| FORK-037 | Уточнены четыре Tauri Stores, WebView, отдельные templates/notifications; исправлены замечания независимого review | Окончательная identity/пути назначения; реализация согласованного экспорта, переноса и реальная upgrade/reopen матрица |
| FORK-053 | Диагностический x86_64 bundle собран и проверен только в указанном объёме | Закрытая цепочка зависимостей, clean exact-commit candidate, полная матрица конкретного артефакта и подпись выпуска; текущая dirty/ad-hoc сборка не подходит |

Дополнение FORK-037 прошло независимый review `review_migration_inventory`: исходный SHA `2afc4e...` имел два P2 (пропущенные TS Stores и отсутствие обязательной смены hardcoded template/notification путей); repair1 SHA `6a4a7c690e4e631f947f9acd9ce79fd0b9bd855b7e979a772e555dc35e46a48e` — оба замечания устранены. Reviewer сверил source fingerprint и все 15 SQL hashes в schema inventory, но не запускал перенос и не воспроизводил SQL runtime. Это принятие уточнений документа, не готовность миграции.

FORK-028 сохраняет прямое указание владельца отложить naming до конца и не задавать новых naming-вопросов сейчас. Разрешение на commit/hooks/push не получено; этих действий нет. Цель целиком не достигнута. Дальнейшая приёмка текущей очереди требует изменения внешних условий; прежние тесты не выдаются за нативную проверку новой `.app`.


## 20 сентября: рабочий стол доступен, длительная нативная запись

Проверен запущенный `run-20260920-audio-recovery-fixed/meetily.app`, PID 58404, executable SHA-256 `012117516f6f225feb98f8a9df49a3eeb820196998721a2718475684a858b7b1`, source fingerprint `4ec41cb8751e0ed2e6e90496fa00d1e1ea6c6f32335fa8aa45a4c420d24f1247`. AX и снимок рабочего стола доступны. В модальном окне восстановления прежняя тестовая встреча `Meeting 19_09_26_23_34_13` показывает `Audio available`; восстановление/удаление не нажималось.

Первый запуск `09:04:13` завершился `failed` / `capture_failed`, duration 0, примерно через 24 секунды. До завершения пришли фрагменты расшифровки с микрофона; системный журнал остановился на `Creating IO proc...`. Причина требует дополнительной диагностики, это не подтверждение прежнего переполнения на 30s. После штатной кнопки Try Again второй запуск `09:05:43` прошёл.

Длительный цикл второй записи: три checkpoint по 30s, затем Pause → Resume → Stop. После stop metadata `completed`, wall duration 161.223239556s; mixed 119.421333s, microphone 119.092333s, system 100.937667s. Все три MP4 полностью декодированы FFmpeg с exit 0, без ошибок. Системная дорожка короче остальных — полная синхронность дорожек этим тестом не доказана. Тестовая речь `The first checkpoint must not interrupt microphone or system audio. One, two, three.` присутствует в расшифровке. Это нативное подтверждение отсутствия прежнего обрыва при сохранении первых двух checkpoints на данном артефакте; остальные сценарии FORK-051 остаются открыты.

Найден отдельный дефект: счётчик UI 00:00 при продолжающейся записи; после Pause меняется на 01:30 и после Resume вновь не растёт. Чтение кода установило устаревший снимок в `RecordingSession::get`, тогда как `get_active` обновляет длительности. Исправление и отдельная проверка выполняются; текущий нативный результат относится к сборке до этой новой правки. После сохранения открылся выбор модели итогов, где Qwen отображается Corrupted; файл модели не менялся.

Доказательства: `evidence/native-audio-2026-09-19/native-long-sha256.json`, AX-снимки, журнал, контрольные суммы/декодирование трёх файлов. Коммитов, hooks, push и публикации нет.

Дополнительная read-only диагностика первого отказа: `create_io_proc_id` вернулся только через ~20s (06:04:16 → 06:04:36), тогда как startup reply ограничен 5s. Поток микрофона уже работал, а вызывающая сторона после timeout ждала штатное завершение capture owner. Поздно созданный CoreAudioStream сразу освобождён. Это объясняет `capture_failed` первого запуска; причина задержки самой macOS пока неизвестна. Увеличение timeout или изменение потоков без дополнительного воспроизведения не выполнялось. При штатном выходе 06:12:03Z журнал подтвердил WAL checkpoint и закрытие базы; затем отсутствие PID 58404 подтверждено.

После повторного запуска той же сборки (PID 70193) выполнен Recover для известной тестовой встречи `Meeting 19_09_26_23_34_13`. UI: `Meeting recovered successfully! / Transcripts and audio recovered`. Контрольные суммы всех 13 файлов каталога до/после совпали; ни один файл не удалён, audio/microphone/system.mp4 не переписаны. Журнал подтвердил сохранение 3 сегментов в базе. ID восстановленной SQLite-записи `bef0161c-b8bd-4fc6-8ec2-6a6dac5d6e2d` отличается от meeting_id прежнего metadata. Read-only проверка `useTranscriptRecovery` / `indexedDBService` установила, что отдельный sqliteMeetingId намеренно выделяется один раз и сохраняется до записи в базу; расхождение само по себе не является ошибкой. Нативный сценарий аварии между сохранением и отметкой восстановления этим прогоном не проверен. Manifest: `native-recovery-sha256.json`.


### Исправление живого счётчика

`RecordingSession::get` теперь обновляет длительности только совпадающей активной сессии под общей блокировкой и сохраняет свежий снимок. Завершённые и другие сессии сохраняют прежний путь чтения. Новый тест воспроизвёл старую ошибку (1 failed), после изменения прошёл (1 passed); вся группа RecordingSession: 41 passed, 0 failed, 1 ignored. Первый ошибочно отфильтрованный запуск с 0 тестов исключён из доказательств.

Итоговый файл `recording_session.rs` SHA-256 `8c4627770ab11828c2c7cbbfae8c9fa4d785edfd70e7102fc80ced3af5565675`, source fingerprint `7a15cff2102519ef4e34413cf374e319bcb94f0fed4e1c37b05ebb04303a18f3`. Main восстановил копию до этой узкой правки удалением нового теста и возвратом прежнего get; SHA-256 точно совпал с `a72b00931553e460e3798d286f8bf88e22078f3761f711804c5ee0ba2dfdaef0`, остальные изменения сохранены. Независимый `review_live_duration` подтвердил снимок и не нашёл блокирующих замечаний; самостоятельно запустил один тест.

`git diff --check` PASS. Проверка rustfmt всего изменённого файла остаётся FAIL из-за четырёх прежних участков вне новой правки; нового форматного замечания в get/test нет. Полный 404 Rust PASS выше относится к предыдущему снимку; на новом выполнена соответствующая группа из 41 теста. Нативная проверка новой сборки ожидается. Manifest: `live-duration-checks-sha256.json`.


### Нативная проверка исправленного счётчика

Release build PASS (3m35s), отдельная копия `run-20260920-live-duration-fixed/meetily.app`, executable SHA-256 `0d2630d6085aa35db1ecafb9cd2a86f94b546a550407dfb98e7b79dad30e80cb`. Source fingerprint до/после сборки и упаковки совпал с `7a15cff2102519ef4e34413cf374e319bcb94f0fed4e1c37b05ebb04303a18f3`; codesign --verify --deep --strict PASS, ad hoc, releaseEligible=false. PID 73653.

В новой копии macOS показала настоящие отдельные системные запросы микрофона и системного звука. Оба прочитаны через AX UserNotificationCenter и разрешены кнопкой «Разрешить», без сброса TCC. Это выявило причину зависания первого старта с большей точностью: системный запрос остаётся открытым дольше 5s, а capture owner уже расходует CAPTURE_REPLY_TIMEOUT. После ответа IO proc создаётся, но сессия уже завершается ошибкой. На этой копии путь воспроизведён; прежняя гипотеза о произвольной задержке macOS уточнена до ожидания диалога разрешения.

Повтор после штатной Try Again прошёл. Счётчик: Recording 00:01 → 00:34; Pause 00:45 → повторно 00:45; Resume 01:08. Stop завершился metadata completed, wall duration 90.9631456s, mixed 69.621333s, microphone 69.866s, system 40.245s; все MP4 полностью декодируются без ошибок. Контрольные фразы до и после паузы есть в сохранённой расшифровке. Системная дорожка короче — её непрерывная временная привязка этим тестом не принимается. CoreAudioStream dropped подтверждён при stop.

Manifest: `native-timer-sha256.json`; provenance: `live-duration-provenance.json`. Новое исправление счётчика имеет нативное доказательство. FORK-051 остаётся in-progress из-за дефекта первого запроса системного звука и невыполненной физической матрицы.


### Подтверждённый сбой выхода во время записи

На timer-fixed `.app` SHA `0d2630d…` выполнены два отдельных сценария без force kill: `NSRunningApplication.terminate` во время записи и пункт нативного меню `Quit meetily` через AX (уникальная точная подпись). В обоих случаях процесс завершился и RunEvent::Exit закрыл базу, но остановка capture/finalize не появилась в журнале. `metadata.status` остался recording, `completed_at` null, итоговые MP4 отсутствуют. Первый тест (~35s) сохранил по одному промежуточному файлу; второй (~16s) не достиг первого checkpoint. Это подтверждённый дефект сохранения при обычном Quit, не PASS lifecycle.

Доказательства: `native-quit-failure.json`, `native-system-quit-runtime.log`, `native-menu-quit-runtime.log`, AX до выхода. Простое ожидание shutdown_active внутри RunEvent::Exit пока не применяется: независимая предварительная проверка нашла риск взаимного ожидания с системным разрешением, которое требует главного потока. Исправление выхода требует согласовать отмену подготовки разрешений и завершающее сохранение.


### Совместное исправление подготовки разрешений и Quit

Снимок `bdfcf1421dad36bb6a56407de3618f358d7e1680ee34b8b5234fbba03f49f595`, HEAD по-прежнему `fcfa6e0…`, рабочая копия содержит изменения. Подготовка разрешений перенесена до создания capture owner, сессии и файлов; 5s timeout не увеличен. Отказ микрофону не создаёт запись; недоступный системный звук переводит запрос на микрофон с сохранением исходной идентичности запроса. При выходе постоянный флаг и сохраняемый сигнал прерывают ожидание разрешений. RunEvent::Exit затем дожидается завершения активной записи до закрытия базы.

Проверены permission group 4/4, session group 44/44 (1 существующий realtime diagnostic ignored); новые отдельные сценарии отказа, downgrade/retry и отмены без освобождения ожидающего разрешения прошли. Независимый reviewer подтвердил новые три SHA-256 и общий source fingerprint, воспроизвёл группы и не нашёл дефектов ограниченной правки. Нативная приёмка этих двух новых исправлений ещё ожидается; предыдущие доказательства относятся к прежней сборке.

Точные SHA-256: permissions.rs `22b4fe590f98043a0a0ed8f39fd9336712ee0ffc9ff83ce9f0f6aa9d35896e2d`; recording_session.rs `cd4122136bd568fc04e172192a0ae636b56e180ae2102b578764072328a7a019`; lib.rs `87624aa2a5becb28af6808b9b49a26f087167ea4d41c0d1bc4a64a7e702483bd`. Git diff --check PASS; полная rustfmt проверка по-прежнему находит прежние участки форматирования вне новых правок. Manifest: `permission-exit-checks-sha256.json`.


Итог автоматического прогона данного снимка: полная Rust-библиотека **408 passed, 0 failed, 5 ignored**, exit 0, 167.20s. Release build PASS, 3m34s. Новый bundle `run-20260920-permission-exit-fixed/meetily.app`, executable SHA-256 `fc63ecaaf2ba75551ccd2bed582b0eff1da5a5acc987242cfee1be8f0a266a75`, codesign verify PASS, ad hoc, releaseEligible=false. Source fingerprint до/после совпал с `bdfcf142…`. Manifest: `permission-exit-build-sha256.json`.

Новая копия открыта через LaunchServices, PID 81429. Перед нажатием каких-либо кнопок свежая проверка рабочего стола показала locked=1, displayActive=0, frontmost=loginwindow; снимок экрана недоступен. На новой копии запись/выход ещё не проверялись. Пользователю отправлен запрос повторно разблокировать Mac; автоуспех вместо нативного результата не ставится. Все старые успешные/неуспешные записи выше привязаны к своим прежним executable SHA.


### Сверка синхронности исходных дорожек при заблокированном рабочем столе

В следующем goal-продолжении свежая проверка вновь вернула locked=1/displayActive=0, PID 81429 той же permission-exit-fixed копии. Source fingerprint остался `bdfcf142…`; новой записи на ней нет. Предыдущий этап дал реальные исправления и нативные доказательства; текущая блокировка не является выполняющимся тестом.

Read-only проверка текущего исходника уточнила ограничение ранее записанных дорожек: `capture/backend.rs::TrackChunk` содержит kind/samples/sample_rate/channels, но не содержит временной позиции; CoreAudio callback не использует `_now` и `_input_time`; `SourceTrackSaver::add_chunk` добавляет samples подряд. Следовательно, если системный поток поставляет меньше samples между разнесёнными во времени событиями, исходная дорожка не может восстановить положение этих разрывов из собственного контракта. Код сам по себе не устанавливает, почему в двух нативных циклах system.mp4 короче; потерю звука, отсутствие callback в тишине и иной источник разницы необходимо различить контролируемым сигналом.

Для приёмки FEAT-071 («время не сдвигается после pause/resume») требуется сравнить разнесённые контрольные сигналы в микрофоне, системе, смешанном файле и расшифровке, включая тишину и паузу. Текущие decode PASS этого не доказывают. Продуктовые исходники не менялись: новая `.app` сохраняет точный проверенный снимок для ожидаемой нативной приёмки.

## 2026-09-20 10:34 MSK — unlocked desktop and native exit verification

User authorized power adjustments. Existing AC settings: system sleep 0, display sleep 30 minutes. Permanent `sudo -n pmset -c displaysleep 0` could not execute because an administrator password is required; no permanent setting changed. Started `caffeinate -di -t 43200` (PID 91390, session 77930) at about 10:28 MSK; readback confirms PreventUserIdleDisplaySleep=1 and PreventUserIdleSystemSleep=1. Protection expires after 12 hours; password lock unchanged.

Exact diagnostic bundle permission-exit-fixed: source fingerprint bdfcf1421dad36bb6a56407de3618f358d7e1680ee34b8b5234fbba03f49f595; executable SHA-256 fc63ecaaf2ba75551ccd2bed582b0eff1da5a5acc987242cfee1be8f0a266a75.

- PID 81429: Start Recording presented microphone permission; while pending, native Quit meetily completed without hanging. No recording folders added or removed versus preflight baseline.
- Relaunched identical bundle as PID 92016. Microphone prompt accepted after delay exceeding 5 seconds. Recording started on first attempt; capture_failed absent. No new system permission prompt shown, so delayed system permission acceptance remains unverified on this exact bundle.
- Timer 00:34, paused 01:00 twice, resumed 01:20. Native Quit during active recording at 07:33:07 UTC stopped capture and finalized audio by 07:33:09 before database cleanup at 07:33:10. Metadata completed.
- Full decode passed: mixed 81.621333 s, microphone 81.823333 s, system 44.831667 s. Track duration discrepancy persists; no synchronization acceptance inferred.
- Evidence: native-permission-exit-result.json and native-permission-exit-sha256.json in evidence/native-audio-2026-09-19, paired runtime logs and AX snapshots. No product edits or commits. FORK-051 remains open for remaining permission/device/error scenarios; FEAT-071 and Alpha are not accepted.

## 2026-09-20 12:58 MSK — LaunchServices permission correction and timed signals

Source fingerprint freshly unchanged: bdfcf1421dad36bb6a56407de3618f358d7e1680ee34b8b5234fbba03f49f595. Native tests now use `open -n <exact bundle path>`. Direct executable launch via the developer tool did not reliably use the app-specific microphone permission: it started capture while Settings showed Meetily microphone OFF. Prior direct-launch audio/quit observations remain observations of that launch environment; they do not establish standalone TCC acceptance.

- With microphone OFF, LaunchServices PID 23371 showed Microphone Permission Required and Open Microphone Settings. Sidebar start did not begin capture. Restored original microphone permission ON.
- macOS Settings Restart selected old /Applications/Meetily.app (PID 24002, version 0.4.0), which aborted. This is a different executable, not a crash attributed to the current candidate. Exact candidate reopened explicitly; no installed app or user data deleted/replaced.
- PID 24354 with restored mic permission displayed system-audio prompt. Accepted after more than 5 seconds. Capture eventually ran, but UI simultaneously displayed Recording Failed / Unable to start recording. New defect, not accepted.
- Completed native recording d53d7560-3135-4e4d-bf8c-af7b871f9195. A 40s WAV contains two 1s 880Hz markers at 2s and 25s. Mixed decoded 65.496s, markers 21.7–22.8 and 44.8–45.9; system decoded 46.24s, markers 2.2–3.3 and 25.2–26.3. Initial system timeline is displaced about 19.5s, while the 23s separation during continuous WAV playback is preserved. Cause remains under source diagnosis. Mic decoded 65.77075s; marker threshold not reached there.
- FFmpeg f32 decode exited 0 but reported nonmonotonic DTS near checkpoint joins for mixed/mic; do not claim error-free decode for this measurement.
- Evidence: launchservices-permission-timing-sha256.json and listed files. FORK-051/FEAT-071 remain open.

Follow-up verification: system-audio denial attempt is NOT VERIFIED. The exact Settings checkbox was changed 1→0 with Meetily closed, but after LaunchServices recording the readback was already 1; no main-agent action had enabled it in between. The guarded restore (expecting 0) refused to toggle. Therefore this does not prove denied-system behavior or a permission bypass. Original microphone/system settings both read ON at final checks. Last test meeting 941b8baf-3b2d-4d34-8636-f3f2419e716c completed; test app closed normally.

Independent frontend diagnosis by review_live_duration: RecordingControls local deviceError is cleared only by its close button; retry and successful recording state do not clear it. No five-second frontend start timeout found. Original triggering error in this native run is unknown. Bounded frontend fix is being implemented with behavioral regression tests.

Signing availability rechecked outside sandbox: `security find-identity -v -p codesigning` returned `0 valid identities found`. No credential/key creation or publication performed.

## UI stale-start-error repair — source fingerprint 6a807fdfaf663c0d2f40fd59f62cacad66e7468fc5ab1ded5ebf780db71989ab

RecordingControls now clears a previous failed-start alert on a new accepted attempt and on confirmed active state from props/context. New recording-controls.test.mjs executes handlers and renders the actual component through the existing style of a simplified hook runtime. Both scenarios failed before repair and pass afterwards; subsequent real failure remains visible. Author typecheck/scoped lint/diffcheck exit 0. Independent review by review_live_duration confirmed exact two file hashes and reran 2/2 tests, no confirmed findings. Real React DOM/native behavior not proven by this harness. Full check and packaged acceptance are still running; raw timing unchanged.

## Full check continuation — c7a099438522206b7a4bec029e10ea3caff3994f24f3df2f8eb4916cfaa0eb8e

Full check 1 was interrupted while Clippy was compiling into the default cache; it is INCOMPLETE, not PASS. Before interruption it found a pre-existing browser locator mismatch (Regenerate Summary versus actual Regenerate AI Summary), independently diagnosed. Main corrected that exact test selector and applied cargo fmt to existing code; formatting and whitespace now pass. UI fix file hashes remain as independently reviewed.

Full check 2 uses full Xcode, absolute target/fork-051 cache and CARGO_NET_OFFLINE=true. Node400/400, unit48/48, browser26/26, lint/typecheck, production frontend build and rustfmt passed. Clippy exited101 at ort-sys empty -L search path. Read-only diagnosis found ort-sys rc10 build.rs returns an empty path when CARGO_NET_OFFLINE=true before checking its cached archive. Existing ONNX static library is available locally; explicitly setting ORT_LIB_LOCATION is the planned retry, without downloads/deletion/source-policy changes. Remaining Rust stages still running at this note; no full PASS claimed.

Full check 2 completed exit 1: Rust workspace tests passed (518 across suites, 10 ignored), separately opted-in packaged FFmpeg recovery 1/1 passed. Only Clippy failed. Retry with explicit local ORT_LIB_LOCATION removed empty-link-path failure but Clippy exited101 on project diagnostics. Independent triage: 70 unique located diagnostics across15files; 69 problem sites already exist in HEAD by source comparison, not a HEAD Clippy run. One new MSRV issue in macos.rs used repeat_n (Rust1.82 versus declared1.77); main replaced it with equivalent repeat(...).take(...). New source fingerprint acd310dea937b537aaea3f40a3349145bfe4f1768c487579a726eedec5b24882; focused capture tests running. Remaining Clippy includes mechanical issues and nine long signatures, including externally invoked api_save_transcript; no blanket allowlist or interface rewrite performed. Gate remains FAIL; next packaging is diagnostic only.

### 2026-09-20: новая сборка с исправлением сообщения, отдельный запуск macOS

Проверена отдельная `run-20260920-ui-error-fixed/meetily.app` через LaunchServices (`open -n`), исходный отпечаток `acd310dea937b537aaea3f40a3349145bfe4f1768c487579a726eedec5b24882`, executable SHA256 `0855ffac44a6c0770f3b8f1dad15bf9c4f68353ea56ad89d6932cfaabe96a946`. Это dirty диагностическая сборка с локальной подписью, не Alpha.

Нативно подтверждены отказ микрофона с переходом к настройкам; восстановление разрешения с ручным перезапуском точной `.app`; разрешение системного звука после длительного ожидания; запись без ложного `Recording Failed`; пауза с неподвижным `00:49`, возобновление до `01:09`; штатный Quit во время записи с финализацией до database cleanup. Встреча `d540a7ed-e1d1-45ff-99a3-81843448a3f7`, сессия `30b95d6d-ba40-4afe-bd6a-d12b77103ed2`, metadata completed.

Все три финальных файла декодированы с exit 0. Длительности: mixed 76.90675s, microphone 77.16275s, system 56.896s; metadata wall duration 102.88956s включает паузу. Отставание/укорочение raw system остаётся открытым. При декодировании mixed/microphone получены предупреждения о повторном DTS (`480112`, `480152`); это не безошибочная проверка контейнера. Старое сообщение failure→success в том же процессе не было воспроизведено в этом прогоне; соответствующий компонентный тест отдельно зелёный. Физическое отключение USB-микрофона ещё не проверено.

Доказательства: `evidence/native-audio-2026-09-19/ui-error-native-result.json`, набор AX-состояний и `ui-error-native-sha256.json`. Тестовый процесс штатно завершён; разрешение микрофона восстановлено. Новые механические исправления Clippy после сборки не входят в этот проверенный бинарник. FORK-051/FEAT-071/FORK-053 остаются открытыми.

### 2026-09-20: подтверждены две дополнительные ошибки нативной записи

**Системный звук при запрещённом доступе.** Exact `.app`/source `acd310...`, LaunchServices PID 57010. Отдельный переключатель «Только запись системного звука» для meetily был выключен и остался `0` во время записи. До старта UI сообщает microphone-only, после старта предупреждение исчезает. При воспроизведении известного сигнала сохраняется `system.mp4` 35.904s с peak=0/rms=0; mixed и microphone ненулевые. Встреча `45392dd0-a24e-4748-9395-6dbdcc707487` completed, файлы декодируются. `probe_system_audio_access` принимает успешное создание stream после 250ms за Available, хотя комментарий CoreAudioCapture уже указывает, что запрет может возвращать нулевой звук. Это подтверждённая неверная индикация доступности, а не обход разрешения. Обычная тишина не должна становиться доказательством Denied. После Stop/Quit разрешение восстановлено `0→1`. Доказательства: `system-denied-native-result.json`, AX до/после и журнал; исправление ещё не выполнено.

**Метки времени AAC на границе checkpoints.** Независимая диагностика воспроизвела командой bundled FFmpeg 8.0.1 склейку сохранённых частей с `-c copy`: Non-monotonic DTS корректируется на границах 30/60s. Начальная AAC-посылка очередной части имеет DTS -1024 / Skip Samples 1024; финальные метки около границы различаются на один кадр48k, поэтому пересчёт8k даёт повтор. Это отдельный дефект от длинного пропуска system timeline. При исходной частоте decode clean; `aresample=8000,asetpts=N/SR/TB` устраняет предупреждение выходного диагностического потока, но не исправляет исходный MP4. Слышимый дефект/потеря звука не измерены. Исследование `dts-audit-results.json`/`dts-audit-concat.json`, checksum manifest `denied-and-dts-sha256.json`. Пользовательские файлы не переписывались. Для ремонта нужна проверка последовательности декодированных кадров и однократного финального кодирования, включая recovery.

### 2026-09-20: исправление ложного подтверждения системного звука

Успешное создание Core Audio stream теперь даёт `Unknown`, а не `Available`; повторная проверка не доверяет прежнему Available. Unknown допускает попытку системного захвата, явные Denied/Unavailable оставляют микрофон. Нулевые данные не объявляются отказом — они неотличимы от обычной тишины. Это исправляет ложное подтверждение, а не гарантирует распознавание скрытого отказа macOS.

RU/EN интерфейс отдельно объясняет неподтверждённый захват и предлагает настройки. Сообщение сохраняется во время записи; повторный probe и refresh-кнопки в этот момент недоступны. Onboarding с доступным микрофоном можно завершить. Browser mock больше не изображает успешный probe подтверждённым разрешением.

Typecheck/lint exit0, Vitest11/11, выбранные Node45/45. Browser первый запуск заблокирован sandbox EPERM localhost; разрешённый повтор:11PASS/1FAIL, единственный новый active сценарий остановился на recovery dialog до assertions. После явного Cancel тестового dialog focused повтор1PASS; это два прогона, не один общий12PASS. Новый тест hash `66338bb9849e3cd7a1f3d9f8a0193581064c7cbbaf0e656b2ec7f3b0c641aa6c`. Манифест исходников `system-unconfirmed-edit.json`; Rust-компиляция и новая `.app` для этой правки пока не проверены. FORK-051 остаётся открытым.

### 2026-09-20: общая проверка интерфейса и временный эксперимент AAC

После правки Unknown: полный frontend400 Node +49 Vitest PASS, production build PASS. Совместный набор browser critical26 + permissions12 =38PASS одним запуском (1.4min), включая окончательный тест с закрытием recovery dialog. Это закрывает прежнюю границу раздельных11+1 прогонов для frontend, но не заменяет Rust/native проверку. Механические Rust-правки журналирования, повторных ссылок, неиспользуемых обёрток и размещения тестов прошли независимое read-only ревью; свежий общий Clippy пока не выполнен.

AAC prototype только в `/private/tmp`: каждая сохранённая часть декодируется отдельно, служебное хвостовое заполнение отсекается по сохранённому `input_sample_count`, байты передаются блоками64KiB в единственный финальный AAC encoder. Исходные checkpoint hashes проверены до чтения. Microphone expected77.077333s, container77.077000s; system expected/container56.832000s. Decode8k exit0 без предупреждений для обеих новых временных копий. Это проверка кандидата алгоритма, не его интеграция: production source не менялся этой проверкой, mixed/recovery/legacy metadata/ошибки процессов ещё не покрыты. Оригинальные записи не изменялись. Доказательства `dts-pcm-prototype-result.json`, `check-concat-pcm.py`, `frontend-and-pcm-prototype-sha256.json`.

### 2026-09-20: промежуточная граница передачи аудиоблоков

Первая широкая стадия изменений семи Rust-файлов остановлена для проверки. Снимок `b8f1403d79d6ad8eadfaa2ce594eb382e79d0a4d46b805b2a02d6a039b2255f4` неизменен до/после root compilation. `cargo test -p meetily --lib audio::permissions::tests:: --locked --offline` с правильным Xcode/ORT/cache окружением завершился exit101: две ссылки `super::capture` внутри тестового модуля pipeline и два ActiveSession без нового поля timeline. Тесты не выполнялись. Исполнителю возвращена ограниченная правка этих четырёх ошибок и scoped formatting; дальнейшее расширение до первого проверенного снимка не принимается.

Уже написаны pool/очередь блоков, частичный fan-out, положения частей, черновой mixer/watermark/journal и SessionTimeline. Но общая начальная отметка времени, полное подключение pause/resume и native19.5s criterion ещё не реализованы/проверены; callback-relative cursor не является общей шкалой разных устройств. Подробная капсула с численными пределами находится в `urgent-verification-2026-09-19.md`. Это промежуточный код, не принятый FEAT-071.

Дополнительный AAC эксперимент: синтетические mono/stereo последовательности из48013/48029/24117 кадров, четыре30ms маркера вокруг двух стыков. Все4 маркера найдены во всех3 выходах в пределах10ms (измерение5ms окнами); никакое submillisecond accuracy не заявляется. Последнее AAC-кодирование добавляет673 хвостовых кадра к декодированному потоку — это отдельно указано в `dts-controlled-markers.json`, не скрыто. Производственный mixed/recovery путь этим не проверен.

### 2026-09-20: проверка промежуточной версии и защита тестов от сна

Снимок `d8c156e3821ee60584d8666807947520d5b6cf018a54cec3751e71c1d55cd9bc`: четыре первоначальные ошибки компиляции исправлены; permissions7, backend22, отдельные mixer1 и journal1 PASS. Общий workspace-прогон, однако, завершился до тестов: в `tests/recording_recovery.rs:143` не добавлено поле span. Строгий Clippy также FAIL: lib20/libtest22 сообщений (пересекаются, складывать нельзя), включая незадействованный прототип timeline и прежние длинные сигнатуры.

Независимое ревью отклонило промежуточный аудиотранспорт: необоснованная готовность mixer при пустом втором источнике и неограниченное накопление; смешение исходных кадров микрофона с пересчитанными48k; блокирующий Mutex в звуковом вызове; drain до остановки источника; неполное сохранение признака частичной доставки; journal без проверки согласованного префикса при recovery. Общая шкала и pause/resume также ещё не подключены. В `.app` этот вариант не упакован. Назначена ограниченная правка: оставить прежний mixer активным, изолировать непроверенные прототипы и устранить ошибки владения буферами/остановки/признака неполной записи перед повторным ревью.

В `scripts/smoke-macos-audio.sh` добавлен `caffeinate -di -w "$$"`; очистка завершает только собственный сохранённый PID. Скрипт SHA256 `d126881db0726e590604e1865f51e12e38dc39dc939d67006e968138c0b78150` прошёл независимое ревью, bash syntax, harness и4 contract tests. Настоящий длительный hardware run с этой обёрткой ещё не выполнен.

Настройки macOS: после выбора «Никогда» для выключения дисплея UI сохраняет это значение, но `pmset` по-прежнему показывает displaysleep30. Поэтому постоянное изменение не считается подтверждённым. Пароль/блокировка не изменялись. Ранее запущенный временный caffeinate удерживает дисплей и систему на время работ; тестовый скрипт теперь создаёт собственное удержание для следующих запусков. Доказательства перечислены в `transport-review-and-awake-sha256.json`.

Узкая консультация по последующей замене pool: `backend.rs` SHA256 `6bd268a53fc39955dd6aa3cb91b2fe4a0a52615701f3ab5e18019593f77bcb36`, `macos.rs` SHA256 `90121e356a4b0339e87b0246372b7bef33178003ad4704235184428dc4384199`. Новый собственный UnsafeCell-пул заменён на имеющуюся crossbeam ArrayQueue. Read-only reviewer не нашёл нового блокера владения: SourceSinks удерживает сильную ссылку вне callback до уничтожения stream, также при setup/recovery; принятые блоки удерживают собственные ссылки. При изменении stop этот порядок должен сохраниться. Это не полное повторное ревью транспорта, не проверка гарантий нативного уничтожения stream и не доказательство отсутствия ожиданий/выделений во всех callback-путях. Тесты этой консультацией не запускались.

Root compilation текущего pool checkpoint `b3d12075a791c47ab12660a6631042f92cb0cf1417e9cfa1def4b4088b0825b7` завершилась exit101 до тестов: `macos.rs:700` ещё использует прежние7 аргументов вместо нового CapturePublishContext. Автор подтвердил незавершённые stop-tail/partial-manifest/journal и получил ограниченный checkpoint только API-связности/pool/fixture, без дальнейшего расширения timeline.

Ограниченная компиляционная правка завершена: macos publish_samples_at использует CapturePublishContext, recovery fixture получил span:None, комментарий pool исправлен. Backend23/23 PASS (включая contention), весь workspace --no-run exit0. Это только компиляция всех тестовых программ, не их выполнение. Реальный macos.rs SHA256 `c469df1d144d02a1696ab2260e02dcfdafdfad114e8b583bbf06e8e0838c62bd` проверен root; опечатка сокращённого hash в авторском сообщении исправлена перед независимым review. Backend SHA256 `a514d9746a9a2da5522d4b487cd0f463a4056d70f93729238a8965f5750c8a69`, recovery test SHA256 `680588badde7274ed00f60dd69c6f1c202aa92a825810318e8a7cf1c0bbf2ed0`. Остальные исходные findings пока открыты.

Независимое review этих трёх точных хешей: PASS только pool/API; ошибок владения/возврата слотов/передачи прежних параметров не найдено. Все три хеша совпали до/после. При этом ArrayQueue допускает повторные попытки при конкуренции, а std SyncSender::try_send не является обещанием отсутствия внутренних ожиданий (путь уведомления ожидающего receiver может использовать mutex; активный drain здесь использует try_recv). Поэтому устранён явный Mutex пула, но жёсткий предел длительности callback не доказан. Stop-tail/partial manifest/journal и общая шкала остаются открытыми. Новые runtime-проверки reviewer не запускал.

Полный workspace после pool/API: source fingerprint `c400e2b2879138ab5d160f1013932a5f99b180a731b119f0af05466e708ae242` совпал до/после обоих прогонов. Первый в sandbox: lib403PASS/13FAIL/5ignored, все13FAIL — `Operation not permitted` при открытии локального тестового HTTP-сервера в model_download.rs1530. Повтор той же команды вне sandbox exit0: суммарно526PASS/0FAIL/10ignored, включая интеграционные наборы и doc tests. Это не Clippy, не native и не закрытие ранее найденных логических ошибок. Журналы с хешами: `pool-full-workspace-sha256.json`. Следующая ограниченная правка относится только к порядку stop/quiesce/drain в macos.rs/core_audio.rs.

Изоляция непроверенного журнала реализована отдельно root: `incremental_saver.rs` before `bc0237c66739afcb5d0ddef92d0d38bbc6c5111612b7b31d2ed5a4e4c8a694ad`, after `e34c200e6db85eaa578557107b8a5a3c8ede0cd61429f25a6f433db1b5136750`. Production add_chunk больше не пишет journal; прототип test-only; обе сборки manifest публикуют None/нулевые timeline-поля. Checkpoint hashes/counts/recovery код не менялся. Independent static review PASS, точный diff сохранён в `journal-isolation.patch`; выполнение тестов пока ожидает освобождения Cargo. Расширенный тест проверяет legacy manifest через уже существующий финальный файл, не повторную сборку файла из checkpoint. Признак incomplete сохранён для отдельного ремонта partial delivery.

Изолированная правка журнала подтверждена выполнением: `cargo test -p meetily --lib audio::incremental_saver::tests:: --locked --offline` exit0, 20/20 PASS, включая расширенный legacy manifest сценарий. SHA saver после теста неизменен `e34c200e6db85eaa578557107b8a5a3c8ede0cd61429f25a6f433db1b5136750`. Журнал `journal-isolation-tests.log`, SHA256 `417fd883bb807f7d986b9ec2a22b271a36cebc9d67c2e9ab109d581568a39151`. В рабочей копии к этому моменту отдельно добавлены stop-регрессии исполнителя; этот запуск фильтрует только saver и не принимает stop-код.

Stop-tail исправление автора готово к независимому review: macos.rs `69a029f9341d7882e84e53bff8b8ec1055d751ad72cc5f33456e685b8662a252`, core_audio.rs `e9ee5770ddc98880557da22798abb925ddd2ab2a01f90736f227dc1c066593b3`. До правки два regression теста теряли блок (0 вместо1), paused-stop уже проходил. После: macos20PASS; core_audio7PASS/1hardwareignored; workspace --no-run exit0. CoreAudio device останавливается перед чтением оставшегося ring, микрофон quiesce/drop до accepted block drain, sinks удерживаются до завершения; ошибка pause возвращается после cleanup. На системной ошибке cidre stop() вызывает Drop с повторной остановкой; debug assertion зависимости не заменяет нативное доказательство cleanup. Свежий native stop пока не запускался. Полный журнал/хеши `stop-tail-sha256.json`. Отдельно начат bounded repair persisted incomplete для частичной доставки, без изменения этих двух stop-файлов.

Независимое source-review stop-tail на двух указанных хешах: PASS в ограниченной области, новых blocker не найдено; хеши до/после совпали. System ring131072 samples переносится максимум32 порциями4096 в отдельный pool64 при штатно пустой промежуточной очереди. Ошибки передачи возвращаются как stop error. Остаточное ограничение cidre a9587fa: ошибка AudioDeviceStop приводит к повтору в Drop, который при повторной ошибке может вызвать debug_assert; гарантия обычного Err/дочитывания при любой системной ошибке не доказана. Аппаратное прекращение callback, native stop и весь транспорт этим review не приняты. Следующий общий снимок должен включать отдельное исправление partial manifest.

Partial-manifest first repair: recording_session.rs `60d8dcf24b7a498f7a1dd6291334abecf595db6575056f4d5befb3f853b8e6c8`, saver `6eb214fdcbe403af4e52b4b52a2a844663fd37d72ff4b0c72cab76d9315b8b3a`. RED сначала заблокирован отсутствующим ORT_LIB_LOCATION (не продуктовый RED), повтор с правильным окружением воспроизвёл ложную успешную финализацию; GREEN regression1 и saver20 PASS, workspace no-run0. Но независимое review BLOCKED/P1: добавленная обработка add_chunk error продолжает приём следующих blocks, буфер растёт при постоянном encode/fsync failure, receiver не закрывается и исходная ошибка теряется. Чистый partial-fanout путь с пустым хвостом корректен; весь patch не принят. Назначена вторая ограниченная правка первой ошибки writer: terminal intake, сохранить saver/первопричину, не продолжать накапливать samples, отдельный regression. Новая `.app` до закрытия blocker не упаковывается.

Совместный pre-terminal-writer снимок `11d5c01c11f1b70586f26fd0a4b8a11390c1fa30209674425a1b4722f768a461` (до/после совпали): frontend production build PASS; полный Rust workspace вне sandbox530PASS/0FAIL/10ignored exit0. Несмотря на это, снимок НЕ ПРИНЯТ из-за review P1 накопления samples после write error — этот путь не был покрыт тестами. Начат regression и ограниченный round2. Отдельно root убрал новые unused warnings: send_source только cfg(test), незадействованные positioned field/init удалены из тестового mixer; никакие проверки Clippy не подавлялись. Снимок этих двух файлов и журналы перечислены в `pre-terminal-writer-checks-sha256.json`.

Round2 writer: recording_session.rs `6522d79aac41b6e672a92b978123bf26a92b842e9b87157ae7084188f48f9b80`, saver `8b41acead1dbd1d83bf8d04bd1106bf386999d5bf414b0985e2df3dd7229ff1b`. Первая ошибка add_chunk останавливает intake и закрывает receiver, сохраняет принятый buffer и первоначальную Persistence-причину; следующие add_chunk не увеличивают counts. RED подтвердил открытую очередь после failure; GREEN forced failure1, прежний partial1, saver20, no-run0. Во время разработки root отклонил глобальный test-only next-error flag как гонку с параллельными tests; итоговая версия использует marker только в отдельном meeting_folder и cfg(test). Это моделируемая ошибка checkpoint, не проверка реального отказа диска. Независимое review и полный совместный runtime-прогон на этом снимке запущены; до их результата нет принятой новой `.app`. Хеши логов в `partial-manifest-round2-sha256.json`.

Повторное независимое review round2: PASS на session6522d79/saver8b41ace, хеши до/после совпали. P1 роста после write error устранён: receiver закрывается, новые samples отвергаются, первая причина сохраняется, уже принятый буфер получает заключительную попытку сохранения; complete MP4 не публикуется. Test marker ограничен отдельным tempdir, общего failpoint больше нет. При продолжающейся ошибке хранения нельзя гарантировать сохранение хвоста/manifest; возвращается ошибка. Реальный отказ диска и нативная запись не проверены. Совместный замороженный fingerprint для общего прогона: `52a49335bb2d883b18fd8e282917046d0525c3f331c26f1ca45c10f931fa0508`.

Итог автоматической проверки перед новой диагностической сборкой: source `52a49335bb2d883b18fd8e282917046d0525c3f331c26f1ca45c10f931fa0508` неизменен до/после; Rust workspace531PASS/0FAIL/10ignored, cargo fmt --all -- --check exit0, diffcheck0. Строгий Clippy exit101:13 сообщений lib,16 lib-test, наборы пересекаются (не складывать). Остались длинные сигнатуры/сложные типы, Default для backend и несколько замечаний к тестам. Новые unused предупреждения pool/timeline/helper устранены; общие проверки не ослаблялись. Это не полный release gate. Журналы и хеши `audio-round2-full-checks-sha256.json`; release-mode executable с tauri/custom-protocol собирается для отдельного локального `.app`, UNGATED.

### 2026-09-20: supplemental audit of existing system tracks

Read-only audit of three already completed recordings in `~/Movies/meetily-recordings` found real `system.mp4` files and successful FFmpeg decoding:

- `Meeting 20_09_26_16_40_45_2026-09-20_13-41`: system `47.338333s`, microphone `69.236667s`, mixed `69.021333s`, system track `2` checkpoints, `incomplete=false`;
- `Meeting 20_09_26_16_45_57_2026-09-20_13-45`: system `56.543667s`, microphone `59.733000s`, mixed `59.421333s`, system track `2` checkpoints, `incomplete=false`;
- `Meeting 20_09_26_10_31_22_2026-09-20_07-31`: system `44.831667s`, microphone `81.823333s`, mixed `81.621333s`, system track `2` checkpoints, `incomplete=false`.

For all nine MP4 files `ffprobe` and a full `ffmpeg -loglevel error -f null` decode exited `0`. Hashes, checkpoint counts and sample counts are in [native-system-track-audit-2026-09-20.json](evidence/native-system-track-audit-2026-09-20.json). The audit is supplemental: these recordings are not bound to the current dirty source fingerprint or final10 package, so they prove that the system source can produce a decodable track on this machine, but they do not close FEAT-071's current-artifact control-signal/synchronization acceptance or FORK-051's remaining permission/device matrix.

Уточнение к этому аудиту: первая запись (`16:40`) привязана к тому же source commit `fcfa6e0` и dirty fingerprint `52a49335…`; её native result также содержит разнесённые 880 Hz маркеры в mixed/system и наблюдение pause/resume. Эта привязка относится к более раннему diagnostic package `run-20260920-stop-incomplete-fixed`, а не к final10 fingerprint `cbf0160c…`; поэтому она усиливает диагностику смещения/укорочения system track, но не превращается в приёмку final10.

### 2026-09-20: repair общей временной шкалы

Причина разницы длительностей была подтверждена в коде: `SourceTrackSaver` соединял входные samples подряд и игнорировал `TrackSpan`, а системный CoreAudio источник запускался асинхронно и имел отдельный callback-счётчик. Исправление добавляет общую active-time шкалу для microphone/system, исключает время pause, создаёт новый epoch после resume, переводит native и implicit spans в общую шкалу, а raw saver материализует forward gaps нулевыми samples и отклоняет overlapping spans. Частота кадров масштабируется отдельно для raw и downstream дорожек.

Проверенный текущий source snapshot: HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`, dirty fingerprint `eceffe76df108b18460a317ea5156d2d8d4707bb239f28fec9b35f9a8f02002a`. Вне sandbox с полным Xcode и локальным Sherpa cache прошли: `cargo fmt --all -- --check`, `git diff --check`, `cargo clippy -p meetily --lib --tests --locked --offline`, Rust library `437 passed, 0 failed, 5 ignored`, все integration suites `99 passed, 0 failed, 5 ignored`, backend `25/25`, macOS capture `23/23`, incremental saver `25/25`; полный `./scripts/check.sh --full` также прошёл: Node `401`, Vitest `49`, Playwright `26`, production build, workspace Clippy/tests и packaged FFmpeg recovery `1/1`. Новые regressions для timeline/gap/overlap — `4/4`. Полный машиночитаемый отчёт: [audio-timeline-repair-2026-09-20.json](evidence/audio-timeline-repair-2026-09-20.json).

Нативная `.app` после этого изменения не запускалась: ранее выданные TCC-разрешения сохранены, новый prompt и сброс TCC намеренно не выполнялись. Поэтому этот этап закрывает источник рассинхронизации и автоматическую проверку, но не принимает текущий system.mp4, native mute/gain, pause/resume на физическом устройстве, полный deny/recovery TCC или переподключение микрофона. FORK-051 и FEAT-071 остаются открыты; FORK-036, FORK-037 и FORK-053 также не изменяют статус.

### 2026-09-21: Universal packaging after deployment-target repair

The Tauri base configuration now declares `bundle.macOS.minimumSystemVersion=14.2`; the release policy and Universal build test reject a missing or conflicting deployment target. After adding the migration core inter-process lock and free-space gate, a fresh offline Universal diagnostic package was built from commit `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c` and canonical dirty fingerprint `316591adbce1f3f78914caa509b1c69661d34756af5ee6ab91453ee249f400ac`. Its `Info.plist` reports `com.meetily.ai`, `meetily`, version `0.4.1`, and `LSMinimumSystemVersion=14.2`; `meetily`, `llama-helper`, and `ffmpeg` each contain `x86_64` and `arm64` slices. Deep strict codesign verification passed, but the package is ad-hoc signed, has no Developer ID team or notarization, and is therefore `UNGATED`.

The current source gate also passed: release policy, Universal build configuration `15/15`, frontend Node `402`, Vitest `49`, critical Playwright `26`, Rust format/Clippy, workspace Rust `455 passed / 0 failed / 5 ignored`, production build, and packaged FFmpeg recovery `1/1`. The app was not launched after this source/config change; existing TCC grants were preserved and no prompt or TCC reset was performed. Physical device reconnect, full TCC deny/recovery, current system.mp4, native mute/gain, and pause/resume synchronization remain unverified. [Universal evidence](evidence/native-universal-2026-09-21.json), [full gate](evidence/full-check-2026-09-21-universal.json).

Read-only local signing inventory on the same host returned `0 valid identities found` for `security find-identity -v -p codesigning`. No signing key or identity was used; this confirms the remaining Developer ID and notarization blocker for FORK-036/FORK-053.

### 2026-09-21: FORK-037 staging core

The product worktree now contains `frontend/src-tauri/src/database/migration.rs`. It accepts only a versioned manifest with named source namespaces and safe relative paths, rejects symlinks and overlapping source/target/staging roots, copies with `create_new`, verifies byte length and SHA-256, removes a partial destination after a failed copy, and permits missing unused source namespaces while rejecting missing namespaces used by the manifest. It now also persists the journal durably without replacing another operation, holds an OS-backed inter-process lock, calculates the checked temporary-space budget with a reserve, and rejects a supplied free-space snapshot below that budget. The isolated Rust migration suite passed `18/18`; strict Clippy passed. No user profile was read or changed. This is a reusable staging layer, not a completed bundle-ID upgrade: WebView export/import, SQLite WAL snapshot/reopen, filesystem free-space measurement at the caller boundary, startup wiring and a real installation with the final identity remain open.

### 2026-09-21: WebView migration protocol core and fresh Universal package

After the WebView migration protocol core was added, the full gate passed again: frontend Node `402 passed`, Vitest `56 passed`, critical Playwright `26 passed`, workspace Rust `455 passed / 0 failed / 5 ignored`, strict Clippy, production build, and packaged FFmpeg recovery `1/1`. The protocol-specific suite passed `7/7` for an explicit localStorage allowlist, deterministic checksummed packages, schema validation, and idempotent import markers.

A fresh Universal diagnostic package was built from commit `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c` and dirty source fingerprint `46aeebe4660806fd094cff3dc87d892c0aadea43748767e1ba89c9b5778b3885`. The app has `x86_64+arm64`, `LSMinimumSystemVersion=14.2`, and strict signature integrity; bundle fingerprint is `4e232bc94e0f22b0d840d18c8e71224c0865a89a3913514b5c943c1a6658a1bb`. The diagnostic verifier still reports the expected eight ungated failures: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization, and unsigned DMG. No app launch, TCC reset, permission prompt, user-data migration, commit, push, or publication occurred. Actual WebView IndexedDB wiring, SQLite WAL/reopen, startup integration, final signing/notarization, and current-artifact native smoke remain open. [Universal evidence](evidence/native-universal-2026-09-21.json), [full gate](evidence/full-check-2026-09-21-universal.json).

### 2026-09-21: WebView migration validation hardening

The protocol now rejects malformed optional transcript timing fields before import. The focused WebView suite is `8/8`, and the full gate remains green with Node `402`, Vitest `57`, critical Playwright `26`, workspace Rust `455 passed / 0 failed / 5 ignored`, strict Clippy, production build, and packaged FFmpeg recovery `1/1`.

The final diagnostic Universal package is bound to dirty source fingerprint `690f4478e61923cc7af09fdc58cb70bf7bd9662c80d3aa186da9e69c495ee863`; bundle fingerprint `16112b3779a1d7118a0aebb5af7a28050ff185202fb537d4817c8d246c9e0491`, DMG SHA-256 `12d839b06189793b52533486acc3c362ebe060fd47e7f9d926743eb5b7ae6d2e`. The diagnostic verifier again reports the expected eight ungated failures for dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization, and unsigned DMG. No app launch, TCC reset, permission prompt, user-data migration, commit, push, or publication occurred. Actual WebView IndexedDB wiring, SQLite WAL/reopen, startup integration, final signing/notarization, and current-artifact native smoke remain open. [Universal evidence](evidence/native-universal-2026-09-21.json), [full gate](evidence/full-check-2026-09-21-universal.json).

### 2026-09-21: real IndexedDB adapter and current diagnostic boundary

The product now includes a real IndexedDB migration source/sink adapter. It reads validated meetings and transcripts from the version-2 database, commits destination rows transactionally with `add` collision protection, records an idempotent operation marker, and rolls back allowlisted localStorage when the destination transaction fails. The focused migration suite is `12/12`; the full gate on dirty fingerprint `c9f8e252ce717feb29e187de0b350f26bcceb2ac68a3c85aa73ab4ab8a94f97d` passed Node `402`, Vitest `61`, critical Playwright `26`, Rust `455/455`, Universal config `15/15`, migration staging `18/18`, and packaged FFmpeg recovery `1/1`.

The current Universal `.app` has bundle fingerprint `0d8f54e682be0f0d59a8620db22d3299376adeac8c6bafb5a14d65724541f265`, executable hashes recorded in [native evidence](evidence/native-universal-2026-09-21.json), and strict ad-hoc signature verification. A current DMG was created outside the sandbox, passed `hdiutil verify`, and passed the mounted `dmg.bundle-match` check. This run did not launch the app, reset TCC, request permissions, read user storage, commit, push, or publish. SQLite WAL/reopen, startup upgrade, Developer ID, notarization, current-artifact native smoke, physical device reconnect, and full deny/recovery remain open.

### 2026-09-21: current package and migration snapshot boundary

После добавления SQLite WAL snapshot/reopen primitive полный офлайн-гейт для текущей dirty рабочей копии прошёл: Node `402/402`, Vitest `61/61`, critical Playwright `26/26`, Rust workspace `457 passed, 0 failed, 5 ignored`, Clippy, production frontend build и packaged FFmpeg recovery `1/1`. Отдельная migration-группа — `20/20`, включая строку только в WAL, проверку неизменности исходной main/-wal/-shm тройки, отказ от существующего destination и immutable close/reopen.

Universal `.app` и DMG пересобраны на source fingerprint `c9f191c68d24c44828a00a64385e900294e90ef3953cb648c88bab4152d6b9c0`; bundle fingerprint `de58c037829b3e3a7ba6db2f37268bbd3af1916a1417c4ba9ce41ee9d770d9a0`, DMG SHA-256 `c037c69ff3d1ba1a3e50092102ed051fd9325cdda5bb02e85d1f328b3d50705c`, `hdiutil verify` и `dmg.bundle-match` PASS. Diagnostic verifier ожидаемо оставил восемь блокеров: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization и unsigned DMG. `.app` не запускался, TCC не сбрасывался и новые пермишены не запрашивались; current system.mp4, native mute/gain, physical reconnect и full deny/recovery остаются непроверенными.

### 2026-09-21: caller-side free-space probe

После добавления системного `statvfs`-замера свободного места migration suite выросла до `21/21`; проверены существующий каталог и destination, которого ещё нет. Полный gate на source fingerprint `784c1510776342e5f166e47fca4794e7c956d625ae3d0cdafa78ccf84fd5ba4a` прошёл: Node `402/402`, Vitest `61/61`, critical Playwright `26/26`, Rust workspace `458 passed, 0 failed, 5 ignored`, Clippy, production build и packaged FFmpeg recovery `1/1`.

Universal `.app` и DMG пересобраны на том же fingerprint; bundle `7a91f315e5c2c220fef8eba857dc40695c97b5558d434c68d03953f72b8a1e41`, DMG SHA-256 `4ed5974284326054767dccbded0e6817bb67bc8fb1ab0afe0cfdf78af19b5434`, `hdiutil verify` и `dmg.bundle-match` PASS. Диагностические ошибки не изменились: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization и unsigned DMG. `.app` снова не запускался, TCC и пермишены не трогались; current system.mp4, native mute/gain, physical reconnect, startup upgrade и full deny/recovery остаются непроверенными.


### 2026-09-21: incomplete migration startup guard and fresh package

В native database startup добавлена read-only проверка `app_data_dir/migration/<operation-id>/journal.json`: `completed` пропускается, незавершённая фаза останавливает открытие базы до публикации частичного профиля. Проверены missing/completed/incomplete cases; migration suite `23/23`, строгий Clippy PASS. Полный gate на dirty source fingerprint `e9416fc4b71583fe965ffa1dfaaf38423bf41409314c8a12e4ffb11933f346b3` прошёл: Node `402/402`, Vitest `61/61`, critical Playwright `26/26`, Rust workspace `460 passed, 0 failed, 5 ignored`, production build и packaged FFmpeg recovery `1/1`.

Universal diagnostic package пересобран из того же snapshot: bundle fingerprint `3b49f7cc434815e07647323656ea79d5ca26e90915704c2e991611ee98b2e672`, DMG SHA-256 `85f626fa762c852e86c2ada5bea6ed7764d4a703ed9c4f0932783def793547db`, размер `157160537` bytes; все main/sidecar binaries `x86_64+arm64`, `hdiutil verify` и `dmg.bundle-match` PASS. Диагностический verifier оставил ожидаемые 8 отказов: dirty/diagnostic provenance, ad-hoc trust, Gatekeeper/notarization и unsigned DMG. Приложение не запускалось, TCC и пермишены не трогались; current system.mp4, native mute/gain, physical reconnect, public migration commands, real bundle-ID upgrade, Developer ID и notarization остаются непроверенными.


### 2026-09-21: publish/readback hardening and fresh diagnostic package

FORK-037 now includes `publish_manifest` and `verify_published_manifest`: publication accepts only canonical staged files, uses no-clobber creation, syncs directories, and rejects a second publish without changing an existing destination. Readback rechecks every manifest entry's size and SHA-256. The focused migration suite is `24 passed, 0 failed`; strict Clippy passes.

The full gate on dirty source fingerprint `36bcf7a6e16df7c4f3a275b9c68832cc86a10c7650f29616c0ec02021dd8d4ab` passed: Node `402`, Vitest `61`, critical Playwright `26`, Rust workspace `461 passed, 0 failed, 5 ignored`, production build, Clippy, and packaged FFmpeg recovery `1/1`. The current Universal package has bundle fingerprint `d329b22545b855ffc16ed67a120d79a36caf0ba3b3122bf9094f5c90ea1e8db5`, DMG SHA-256 `6f4a3709e426a03efb32f6d1b40ac54f8a4866bf4bfecbff1255f531dbdd4add`, and DMG size `157132087` bytes. Diagnostic verification report: `/private/tmp/meetily-universal-report-publish-readback-20260921.json`.

The verifier passes bundle metadata, resources, both architectures, hardened runtime, DMG structure/integrity, and mounted bundle match. It intentionally leaves eight expected failures for dirty/diagnostic eligibility, ad-hoc trust, Gatekeeper/notarization, and unsigned DMG. No app launch or TCC/permission request was performed. Public migration commands, real bundle-ID upgrade, Developer ID/notarization, current-artifact native system/mute/gain/synchronization, physical reconnect, and full deny/recovery remain open.

### 2026-09-21: атомарная публикация migration и граница native-проверки

В FORK-037 добавлена публикация через уникальный временный файл и `hard_link`: финальный destination не видит частичный файл, существующий destination не удаляется, а parent-каталоги проверяются как реальные каталоги без symlink. Journal phase обновляется монотонно и атомарно; `resume_published_manifest` завершает только совпадающие по размеру и SHA файлы и дописывает отсутствующие. Migration suite прошла `30/30`, строгий Clippy и полный gate — PASS: Node `402`, Vitest `61`, Playwright `26`, Rust `467` (5 ignored), packaged FFmpeg `1/1`.

Universal diagnostic package на fingerprint `f06a6f8d…` имеет bundle `a1c1830c…` и DMG `33a3e4c0…`; `hdiutil verify` и `dmg.bundle-match` PASS. `.app` не запускался, поэтому TCC и permission prompts не повторялись; current system.mp4, native mute/gain, physical reconnect и full deny/recovery остаются отдельными native gates.

### 2026-09-21: smoke harness self-check

Команда `MEETILY_AUDIO_SMOKE_HARNESS=1 scripts/smoke-macos-audio.sh` прошла: harness отвергнул нулевой результат для enumeration и CoreAudio capture и принял только точные one-test fixtures. Реальный hardware mode не запускался, поэтому этот результат не заменяет current-artifact capture и не вызывает TCC/permission prompts.

### 2026-09-21: permission lifecycle browser contract

Отдельный Playwright suite `tests/macos-permissions-lifecycle.spec.ts` прошёл `12/12`. Он проверяет одноразовый вызов microphone permission для `not_determined`, refresh после возврата из Settings, optional system audio, корректное microphone-only поведение и отсутствие повторного system-audio probe во время записи. Suite работает в browser harness и не является доказательством TCC или current-artifact native capture.
