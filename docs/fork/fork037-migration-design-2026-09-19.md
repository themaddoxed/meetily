# FORK-037: проект переноса при смене bundle ID

Статус: протокол staging, WebView IndexedDB adapter и Tauri-команды миграции реализованы, полная миграция ещё не закрыта. Документ основан на read-only исследовании агента `migration_design` 2026-09-19 и требованиях FORK-037. Финальная identity отложена в FORK-028. Пользовательские данные не читались и не изменялись.

Исследован `.worktrees/feat071-audio-controls`, HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c` с незакоммиченными изменениями FORK-051/FEAT-071 и FORK-037. Пользовательские storage-файлы не изменялись. В рабочую копию добавлены параметризованный Rust core `frontend/src-tauri/src/database/migration.rs`, протокол WebView `frontend/src/services/webMigration.ts` и реальный IndexedDB adapter `frontend/src/services/indexedDBService.ts`: manifest с именованными source namespace, проверка относительных путей и непересекающихся roots, отказ от symlink, staged copy с `create_new`, проверка размера/SHA-256, durable journal без замены чужой операции, межпроцессный lock, checked disk-budget/free-space gate, caller-side `statvfs` measurement, allowlisted localStorage, transactional IndexedDB destination commit, collision-safe `add`, rollback localStorage, per-entry/package checksums, schema validation включая optional transcript timing fields и idempotent operation marker. Добавлен SQLite WAL trio snapshot: копирование main/-wal/-shm в staging, повторная проверка неизменности источника, `VACUUM INTO`, `integrity_check` и immutable close/reopen. Rust migration suite прошёл `21/21`, WebView migration tests — `12/12`, Clippy `-D warnings` прошёл. Это не запуск приложения, не startup upgrade и не чтение пользовательского профиля.

Core migration layer does not publish data or alter the old profile. It is deliberately independent of the final bundle ID so it can be reviewed before FORK-028. The staging core has a durable journal, an OS-backed inter-process lock, and a checked disk-budget/free-space gate. The product now also has a real IndexedDB source/sink adapter with an explicit localStorage allowlist, checksummed deterministic packages, schema validation, transactional collision-safe destination writes, and idempotent import markers. SQLite WAL-consistent copy and close/reopen verification are implemented as a bounded primitive; filesystem free-space measurement, a read-only startup guard for incomplete operation journals, and public Tauri prepare/publish/status commands are implemented. A real old-to-new bundle-ID upgrade and independent review of the final identity remain required for task acceptance.

## Данные и существующая защита

Пути в этой таблице относятся к исследованной рабочей копии.

| Данные | Владелец в коде | Решение для переноса |
| --- | --- | --- |
| `app_data_dir/meeting_minutes.sqlite`, прежний `meeting_minutes.db` | `frontend/src-tauri/src/database/manager.rs:281` | Согласованный снимок БД с учётом WAL; исходник не открывать для изменения |
| Notes, vocabulary, speakers, lifecycle, настройки | `frontend/src-tauri/migrations/` | Проверять все пользовательские таблицы текущей схемы, включая новые |
| `recording_preferences.json` | `frontend/src-tauri/src/audio/recording_preferences.rs:129` | Сохранить настройки и внешний `save_folder` |
| `onboarding-status.json` | `frontend/src-tauri/src/onboarding.rs:43` | Сохранить пользовательские выборы; технический статус разрешений проверить заново |
| Notification settings | `frontend/src-tauri/src/notifications/settings.rs:113` | `${dirs::config_dir()}/meetily/notifications.json`; сохранить пользовательские настройки, заново получить системное разрешение |
| `app_data_dir/models`, `models/summary` | `whisper_engine/commands.rs:14`, `summary_engine/models.rs:265` | Скопировать и проверить размеры/хеши без повторного скачивания |
| Записи: выбранная папка или `~/Movies/meetily-recordings` | `audio/recording_preferences.rs:50` | Оставить файлы на месте и сохранить ссылки; не менять Recording Root в этой задаче |
| localStorage и IndexedDB | `ConfigContext.tsx`, `summary-language-preferences.ts`, `indexedDBService.ts` | Нужен отдельный список долговременных ключей и способ экспорта из прежней identity |

Текущий `database/legacy_fastapi.rs` умеет делать staging-копию SQLite/WAL/SHM, согласованный snapshot, проверку известной схемы, публикацию без перезаписи и восстановление после сбоя. Это пригодная основа, но его `LEGACY_TABLES` проверяет лишь старую FastAPI-проекцию: meetings, transcripts, summary_processes, transcript_chunks, settings и transcript_settings. Распространять этот digest на перенос всего современного приложения нельзя: он не доказывает сохранность notes, vocabulary, diarization и остальных новых таблиц.

## Предлагаемый порядок

1. После FORK-028 зафиксировать точные старый и новый каталоги из identity. Не искать похожие папки по всему Home. До выбора нового ID значения назначения остаются неопределёнными.
2. Перед созданием пустой БД нового приложения предложить перенос найденных данных. Пока решение не принято, не создавать live destination DB, которая могла бы помешать переносу.
3. Убедиться, что оба приложения не записывают данные. Существующий gate в пределах одного app-data каталога недостаточен для доказательства остановки старого приложения: нужен согласованный механизм блокировки двух каталогов либо проверяемая процедура закрытия старой версии. Повторное чтение хешей помогает обнаружить гонку, но не заменяет блокировку.
4. Если новое назначение уже содержит пользовательскую БД, остановиться без замены и объединения данных. Состояние предыдущей прерванной миграции обрабатывать только по проверенному журналу этой операции.
5. Составить явный список разрешённых к переносу файлов: тип, относительный путь, длина, SHA-256. Отклонять обход каталога, ссылки и неоднозначные SQLite sidecars. Не включать неизвестные файлы молча.
6. Проверить место с учётом одновременно существующих snapshot, candidate, копий моделей, журнала и резерва. Точный резерв и формула должны быть определены до реализации. Недостаток места останавливает процесс до публикации.
7. Создать staging только в новом каталоге. Скопировать главный SQLite-файл вместе с WAL/SHM по существующему безопасному алгоритму; открывать только копию. Получить согласованный snapshot, мигрировать candidate и проверить данные до/после.
8. Проверка БД: `integrity_check`, известная история SQLx, полный список таблиц и содержимого. Для неизменённых таблиц сравнить упорядоченные значения и число строк; для преобразованных схемой — явно заданные инварианты сохранения данных. Не требовать побайтового равенства SQLite после SQL migrations.
9. Скопировать перечисленные stores и модели, проверить их содержимое. Состояние незавершённых загрузок должно иметь отдельное правило: безопасное продолжение по проверенному manifest или сохранение без активации; неизвестный файл не удалять.
10. Внешние записи оставить на прежнем месте. Проверить существование ожидаемых дорожек и ссылки в БД, не переписывая source recordings. Перевод путей относительно Recording Root остаётся FORK-043.
11. Журнал операции в destination содержит версию формата, ID операции, source/destination identity, список принадлежащих операции файлов, хеши и фазу. До первой публикации журнал должен быть сохранён надёжно. Публиковать без перезаписи и с проверкой после повторного открытия.
12. Поскольку несколько файлов нельзя публиковать одним обычным rename, запуск нового приложения должен читать журнал и не открывать частично перенесённое состояние. Признак завершения записывается только после проверки всего набора.

Исходный каталог старого продукта и внешние записи остаются неизменными. Резервные копии и staging принадлежат новому каталогу операции; создавать новые файлы внутри старого каталога без необходимости не предлагается.

## WebView и разрешения

Изменение bundle ID может отделить прежние localStorage/IndexedDB от нового приложения. Простого копирования SQLite недостаточно. Перед реализацией нужен список долговременных ключей: язык интерфейса, тема, язык/настройки summary, пользовательские шаблоны и другие реально сохраняемые предпочтения. Нельзя считать browser recovery cache единственным источником состояния встречи: рабочее состояние должно восстанавливаться через Rust/SQLite.

Нужно выбрать и проверить способ получить данные старого WebView: явный экспорт из старой версии либо поддерживаемый доступ к отдельной копии профиля. Слепое копирование внутреннего WebKit-каталога в новый профиль не принимается как проверенный способ. sessionStorage не переносится. Точный список ключей и механизм экспорта остаются открытой частью проекта.

Системные разрешения macOS не копируются. Новый bundle ID проходит собственный запрос доступа к микрофону и системному звуку; сохранённое значение onboarding не должно изображать уже выданное разрешение.

## Сбои и возврат к старой версии

До публикации ошибка оставляет исходник нетронутым. После частичной публикации следующий запуск сверяет журнал и хеши, затем завершает проверенную операцию либо отводит её собственные файлы в отдельный карантин. Не удалять файлы, принадлежность которых операции нельзя доказать; несовпадение хеша останавливает автоматическое исправление.

Старое приложение продолжает работать с прежними данными. После начала новых записей в новом приложении возврат к старому не является обратной миграцией: новые встречи нельзя автоматически потерять или перезаписать. Для такого возврата требуется отдельный экспорт/план переноса. UI должен сообщать эту границу до переключения.

## Приёмка реализации

| Проверка в отдельной временной папке | Обязательный результат |
| --- | --- |
| Все современные таблицы, настройки и шаблоны | Сохранились значения, связи и ожидаемые преобразования схемы |
| Строка только в WAL | Строка доступна после переноса; хеши исходной тройки не изменились |
| Microphone/system raw tracks и внешний `save_folder` | Файлы и пути сохранились, встреча открывается в новом приложении |
| Модели и прерванная загрузка | Проверенные модели доступны без сети; повреждённые не активированы и не удалены |
| Прерванные обработка/запись | Восстановление не создаёт дубли и не удаляет исходный звук |
| Сбой на каждой операции и повторный запуск | Нет частично принятого состояния; журнал позволяет безопасно продолжить или остановиться |
| Заполненный диск на staging/copy/publish | Старые данные неизменны; завершение не заявляется |
| Уже существующая destination DB | Перенос отказан без перезаписи |
| Неизвестная схема, ссылки, повреждение, изменение source | Понятный отказ без автоматической потери данных |
| Повторный первый запуск | Завершённый перенос не повторяется |
| Реальная установка с новым bundle ID | Встречи, шаблоны, speakers, settings, модели и воспроизведение доступны после перезапуска; разрешения запрошены заново |

Переиспользовать подходы `frontend/src-tauri/tests/data_integrity.rs` и fault/restart tests `database/legacy_fastapi.rs`. Проверки должны включать фактическую временную файловую систему и upgrade/reopen, а не только тестовые ответы функций.

## Источники и границы

SHA-256 проверенных агентом файлов:

- `database/manager.rs`: `1d3835913bf8130e191a68f9d4934d6cab639773660c8fa139ac286d612c26f7`.
- `database/legacy_fastapi.rs`: `49be106738a1c981435b14d4436b301a58e899cbcafe00020596378f0147b363`.
- `audio/recording_preferences.rs`: `45b227ba8ca3edae0cb04ee8ab32f9c73911a9392a6cc05c53f2e4660efb1692`.
- `tests/data_integrity.rs`: `440d9e6301f6eef3ab7733a56dab7973c98c1dfd807584d71897c666d2786bc0`.

До реализации остаются: final identity и окончательные пути назначения; вызов WebView export/import из пользовательского сценария; real upgrade и независимое review окончательного проекта. Staging lock, journal, disk-budget/free-space gate с caller-side `statvfs`, SQLite WAL snapshot/reopen, adapter tests и Tauri prepare/publish/status commands уже проверены. Пользовательский профиль и upgrade не проверялись. `lib.rs` уже изменён FORK-051, поэтому подключение нового startup flow требует согласования, без замены файла целиком. FORK-037 остаётся открытым.


## Уточнение 20 сентября: явный список данных

Read-only inventory агента `migration_design` и дополнительная проверка main относятся к product HEAD `fcfa6e0ba4b0953b2c03c87cde24153f92e74e0c`, dirty source fingerprint `4ec41cb8751e0ed2e6e90496fa00d1e1ea6c6f32335fa8aa45a4c420d24f1247`. Пользовательские файлы не открывались. Ниже — список по коду, а не утверждение о содержимом установленного профиля.

| Источник | Что переносить | Правило |
| --- | --- | --- |
| Tauri Store `recording_preferences.json`, ключ `preferences` | Настройки записи, внешний `save_folder`, выбор устройств | Сохранить значения. Устройства разрешать заново по фактическому списку, не обещать наличие старого устройства |
| Tauri Store `onboarding-status.json`, ключ `status` | Завершённые шаги и выбор моделей | Сохранить предпочтения; доступность моделей проверить по перенесённым файлам, системные разрешения запросить заново |
| Tauri Store `preferences.json`, ключ `show_recording_notification` | Выбор, показывать ли уведомление «сообщите участникам о записи» | Перенести true/false без изменения; это настройка интерфейса, а не системное разрешение |
| Tauri Store `analytics.json` | `analyticsOptedIn`, `analyticsDefaultOffMigrationV1`; идентификатор и история использования | Исходный файл сохранить в приватном архиве операции. В новом профиле записать `analyticsOptedIn=false`, `analyticsDefaultOffMigrationV1=true`; прежний отказ сохраняется, прежнее согласие не включает аналитику нового продукта автоматически. Новый opt-in только через явный выбор пользователя в новом приложении. Старые `user_id`, счётчики и историю не активировать в новом профиле |
| `${dirs::config_dir()}/meetily/notifications.json` | Preferences, DND, consent | Отдельный явно разрешённый источник вне Tauri app-data; `system_permission_granted` в новом профиле сбросить в false |
| `${dirs::data_dir()}/Meetily/templates/*.json` | Все пользовательские шаблоны, включая переопределения встроенных | Это реальные пользовательские данные; сохранить ID и содержимое. Встроенные bundle templates не копировать вместо пользовательских |
| `app_data_dir/selected_summary_template.json` | Поле `templateId` | Основной путь, используемый native app-командами. Проверить ссылку после загрузки шаблонов |
| `${dirs::data_dir()}/Meetily/selected_summary_template.json` | Прежний отдельный selection-файл, если присутствует | Legacy fallback. При наличии обоих не терять альтернативу: сохранить её в архиве операции; актуальный app-data файл имеет приоритет согласно текущим native-командам |
| localStorage: `themeMode`, `interfaceLanguage`, `primaryLanguage`, `showConfidenceIndicator`, `isAutoSummary` | Пользовательские предпочтения | Явный экспорт/импорт с проверкой типа и допустимых значений |
| localStorage: `summarySystemPrompt`, `summaryLanguageDefault`, `summaryLanguageRecents` | Prompt и языковые предпочтения | Сохранить; не записывать содержимое prompt в диагностические журналы |
| localStorage: `summaryLanguageFallback:<meetingId>`, `detectedSummaryLanguageFallback:<meetingId>` | Динамические языковые предпочтения встречи | Сохранить известный префикс и ID; применять только при отсутствии более нового значения в SQLite, конфликт сохранить для проверки |
| localStorage: `providerModelMap`, `betaFeatures` | Последний выбор модели для провайдера и экспериментальные настройки | Сохранить исходные значения в экспорте; активировать только известные целевой версии пары/флаги. Неподдерживаемые значения не удалять из архива |
| IndexedDB `MeetilyRecoveryDB`, version 2 | Stores `meetings`, `transcripts` и `migrationOperations` | Обязательный перенос несохранённых встреч и связанных transcripts с исходными ID, SQLite ID, порядком и folderPath; saved rows — производный cache, но их исключение фиксировать в отчёте; operation marker не перезаписывает чужую операцию |

Не переносить как активное состояние: localStorage `transcription_progress`, `downloading-models`, тестовый `e2e-summary-template-selection`, browser cache `meetily.selected-summary-template` (выбор восстановить из native-файла). Не переносить sessionStorage: `recovery_dialog_shown`, `meetily_user_id`, `is_first_launch`, `meetily:explicit-microphone-device`, `autoStartRecording`, `indexeddb_current_meeting_id`. Это состояние сеанса, а не доказательство выполняющейся операции.

Четыре production Tauri Store найдены совместным поиском TypeScript и Rust; только поиск `tauri_plugin_store` в Rust пропускает два файла интерфейса. Владельцы дополнительных stores: `frontend/src/lib/recordingNotification.tsx:13`, `frontend/src/components/RecordingSettings.tsx:74`, `frontend/src/components/AnalyticsProvider.tsx:33`, `frontend/src/components/AnalyticsConsentSwitch.tsx:74`, `frontend/src/lib/analytics.ts:177`. Для analytics правило выше — намеренный сброс согласия при смене продукта с сохранением исходного состояния в архиве; это не согласие macOS и не потеря исходного файла.

Список путей задаётся именованными источниками (`legacy_app_data`, `legacy_named_config`, `legacy_named_data`, `web_export`), а не произвольными абсолютными путями из импортируемого JSON. При разрешении реальных каталогов проверить совпадение/вложенность и регистр имён; нельзя считать config/data разными физическими каталогами без проверки. Каждый файл включается один раз по проверенному пути. Ссылки и выход из разрешённого корня отклоняются. Настройки и SQLite могут содержать приватные значения: файлы экспорта доступны только владельцу; в отчётах — типы, количество, размер и хеши целых артефактов, без значений настроек и ключей.

### Обязательные отдельные пути назначения

Смена bundle ID сама по себе не меняет нынешние жёстко заданные `Meetily/templates` и `meetily/notifications.json`. Новая версия обязана прекратить читать и записывать эти legacy locations как действующие хранилища: они используются только как явно разрешённые источники migration. После переноса владельцы templates и notifications обращаются к копиям под новым `app_data_dir` (`templates/`, `notifications.json`); выбор шаблона — к `selected_summary_template.json` там же. Значение нового `app_data_dir` определяется после FORK-028 через финальную identity. Все четыре Tauri Stores тоже используют только новый app-data root.

До копирования и после повторного открытия проверить canonical paths и file identities источника и назначения, включая случай регистра имён на файловой системе: совпадение, алиас или размещение назначения внутри исходного каталога отклоняется. Старые именованные каталоги не становятся fallback для обычных записей новой версии. Только после проверки новых владельцев/путей допустимо обещать независимые настройки и шаблоны двух приложений. Внешний Recording Root остаётся явным исключением: он сохраняется по ссылке, его переделка принадлежит FORK-043, и перенос не изменяет существующие записи.

### Экспорт WebView: выбранное направление реализации

Использовать явный экспорт из совместимой промежуточной версии под прежней identity. Она читает только перечисленные localStorage keys и `MeetilyRecoveryDB` через свой существующий WebView, затем передаёт типизированный пакет в Rust. Новое приложение импортирует пакет через собственный WebView и проверяет обратным чтением число записей, ID и digest. Копирование закрытого внутреннего профиля WebKit не используется. Для старой версии без экспорта нужен подтверждённый upgrade до промежуточной версии; наличие JSON с произвольным именем не доказывает совместимость.

Экспорт производится только после остановки записи, обработки и загрузок, завершения pending browser writes и закрытия SQLite pool. До завершения экспорта или явной отмены все новые изменения в старой версии запрещены. Пакет содержит version 1, operation ID, исходные identity/schema versions, явный список экспортированных ключей/записей, stable IDs и контрольные суммы; значения не попадают в общий migration journal. Один и тот же ID операции не импортируется повторно. Если старый профиль изменился после экспорта, новый импорт требует свежего согласованного пакета.

В product worktree добавлены протокол-пакет `frontend/src/services/webMigration.ts` и adapter `frontend/src/services/indexedDBService.ts` с тестами `12/12`: явный allowlist localStorage, сортировка и stable JSON, SHA-256 каждой записи и пакета, проверка схемы включая optional transcript timing fields, чтение реальных IndexedDB stores, transactional destination commit с `add`, rollback localStorage и idempotent operation marker. Публичные команды/round-trip WebView, одновременное сохранение browser/SQLite данных и startup integration должны быть проверены до реализации публичного перехода.

### Блокировка и журнал операции

Подтверждено по `database/legacy_fastapi.rs:541`: существующий `acquire_app_data_gate` — реестр `AsyncMutex` внутри одного процесса. Он **не является межпроцессной блокировкой** и не защищает от второго приложения с другим bundle ID.

Для перехода обе совместимые версии должны использовать один системный файловый lock, вычисленный из canonical legacy root и UID; расположение coordinator-файла задаёт приложение, не импортируемый пакет. В рабочем режиме удерживается shared lease на время жизни процесса. Экспорт после остановки внутренних работ и импорт до инициализации нового профиля требуют exclusive lease; если другой процесс ещё работает, перенос не начинается. При переходе shared → exclusive не считать замену атомарной: потеря возможности получить lock означает отказ и повторную проверку, а не продолжение без защиты. Старые версии без такого протокола не объявляются совместимыми. После завершения обе версии могут снова использовать собственные данные; это не автоматическая обратная синхронизация.

Журнал версии 1 находится в namespace операции внутри destination. Обязательные поля: `formatVersion`, `operationId`, `sourceIdentity`, `targetIdentity`, версии схем/экспорта, ID разрешённых корней, исходные и ожидаемые digest, список файлов (source root ID + relative path, target relative path, byteLength, SHA-256, ожидаемое преобразование), опубликованные артефакты и `phase`. Пути операции вычисляются заново и сравниваются с журналом. Credentials и содержимое пользовательских документов в журнал не включаются.

Фазы: `preparing` → `prepared` → `publishing` → `verified` → `completed`. Неизвестная версия/фаза останавливает запуск. Переход записывается через собственный временный файл, fsync файла и каталога. Публикация каждого файла не заменяет существующий destination; после сбоя принадлежность уже опубликованного файла устанавливается по журналу и хешу. `completed` разрешён только после reopen/readback проверки SQLite, stores, templates и WebView import; до этого обычный runtime нового приложения не открывает частично перенесённые данные. Source и внешние записи не удаляются. Возврат после новых данных в destination требует отдельного плана.

### Проверяемый дисковый бюджет

Для планирования определить `D` как сумму размеров исходной SQLite-тройки; `F` — сумму моделей, stores и templates, требующих копирования; `W` — размер сериализованного browser-пакета. Все суммы считаются с checked arithmetic, переполнение — отказ.

Консервативный лимит временного потребления: `P = 7*D + F + 2*W + 128 MiB`. Он покрывает две SQLite-тройки (неизменный backup и рабочую копию), snapshot с бюджетом `D + 64 MiB`, candidate с бюджетом `4*D + 64 MiB`, по одной копии прочих файлов и две копии browser-пакета. Публикация внутри того же destination volume не должна создавать ещё одну полную копию моделей. Резерв `R = max(256 MiB, ceil(P/10))`; начальное свободное место должно быть не меньше `P + R`.

Это **лимит и правило отказа**, а не обещание верхней границы для любой будущей SQLite migration: если snapshot/candidate перерастает свой бюджет, операция останавливается без публикации и без изменения source. Перед каждой фазой заново проверяется место для оставшихся выделений плюс R; чужой процесс всё равно может заполнить диск, поэтому ENOSPC обрабатывается и проверяется в каждой операции. Файловая система, где необходимые fsync/no-clobber операции не поддерживаются, отклоняется с понятной ошибкой. Конкретные свободные байты пользовательского диска этим проектом не проверялись.

### Дополнительные условия проверки

В матрицу реализации добавить: `show_recording_notification=false` сохраняется; analytics source true/false/missing даёт target opt-out с migration marker, архив сохраняет исходный выбор, до нового opt-in нет аналитических запросов; новая версия после снятия exclusive lock меняет только свои templates/notifications, хеши старых файлов неизменны; пользовательский template с тем же ID, что у встроенного; оба selection-файла с разными значениями; notifications вне app-data; несохранённая встреча только в IndexedDB с zero transcripts и доступным аудио; повтор browser-import с тем же operation ID; сбой после WebView import до completed; попытка записи из старого/нового процесса при exclusive lock; версия старого приложения без lock/export; переполнение расчёта диска и превышение candidate budget. Успех по SQLite отдельно не закрывает эти сценарии.

Дополнительные SHA-256 источников: `summary/templates/loader.rs` — `9c255e3c8d2bb3ef02ce8576bf3d7cbd27cdd8258171cc2b1dceed3aa224a94d`; `summary/template_selection.rs` — `3585af5378edf51bedde37a904a5ecda9b7f370a1ffaf150deab60d071ee5f6e`; `notifications/settings.rs` — `c6fdd54cd6d7cc0033d112261394bcae154d5a1215062fd5fb9b3828c15846a2`; `services/indexedDBService.ts` — `6c0799669b1033e26aa557ede9c70d8eff93b8cc77d7b1d60c018d6aa8d3c6b9`. Эти уточнения не являются реализацией или проверкой миграции. FORK-028 остаётся блокером финальной identity и назначения.

### Схема БД: дополнительная проверка на пустом временном профиле

Main применил все 15 SQL migration-файлов по порядку к новой SQLite в `/private/tmp`, без чтения пользовательской БД. Получены 12 пользовательских таблиц: `diarization_settings`, `licensing`, `meeting_diarization_status`, `meeting_notes`, `meeting_transcript_vocabulary`, `meetings`, `settings`, `speaker_segments`, `summary_processes`, `transcript_chunks`, `transcript_settings`, `transcripts`. `PRAGMA integrity_check` вернул `ok`. Детальная схема, версия SQLite и хеш каждого SQL-файла сохранены в [schema inventory](evidence/native-audio-2026-09-19/migration-schema-20260920.json), SHA-256 `6c4f72791a9a5e5e1904e988761b2c4874dd8ff0169438e81ccaa5474e026ee0`.

Это проверка состава схемы на пустой БД, не перенос встреч и не SQLx runtime test: `_sqlx_migrations` этим скриптом не создавался. Реальная миграция отдельно проверяет ledger SQLx, все 12 таблиц с непустыми связанными fixtures и native startup recovery. Полный digest нельзя ограничивать прежними шестью `LEGACY_TABLES`.

Дополнительные источники исправленного списка: `components/RecordingSettings.tsx` SHA-256 `0334f5672e92b3be676ff10477de826cb2d2d1a3bb33feed7faacb05ad7751bc`; `components/AnalyticsProvider.tsx` SHA-256 `6ff67c7870303afd61cffb90375542c3da9f33509ad6e3a6658fe9d532f68fbb`; `lib/analytics.ts` SHA-256 `6b00ccb0e8285cf4ded49363fb0d032310829642eb0aa015e3e3b1300b5e42cc`.

## Проверка SQLite WAL snapshot/reopen 2026-09-21

В product worktree добавлен `snapshot_sqlite_database(source, snapshot, staging)`. Он принимает только обычный main-файл и его `-wal`/`-shm` sidecars, отбрасывает ссылки и существующий destination, копирует trio в частный staging, сверяет SHA-256 источника до и после, делает `VACUUM INTO`, проверяет `PRAGMA integrity_check`, удаляет staging после завершения и открывает опубликованный snapshot в SQLite immutable read-only режиме. Тесты проверяют строку, записанную только в WAL, неизменность source trio, очистку staging и отказ без перезаписи существующего destination. Изолированная группа миграции прошла `20 passed, 0 failed`; строгий Clippy прошёл.

Это доказательство согласованного snapshot primitive, а не завершённая миграция профиля: публичные Tauri-команды и full upgrade startup wiring не подключены; read-only incomplete-journal guard уже подключён, final bundle ID не выбран, реальный upgrade на отдельной установленной копии не выполнялся, пользовательские данные не читались. На следующем этапе добавлен отдельный caller-side `statvfs` probe; его свежая проверка приведена ниже. Полный gate этого более раннего snapshot на dirty source fingerprint `c9f191c68d24c44828a00a64385e900294e90ef3953cb648c88bab4152d6b9c0` прошёл: Node `402`, Vitest `61`, critical Playwright `26`, Rust `457 passed / 0 failed / 5 ignored`, Clippy, production build и packaged FFmpeg recovery `1/1`.

## Caller-side free-space measurement 2026-09-21

Добавлен `available_migration_free_space(path)` на базе системного `statvfs`. Для существующего файла, каталога и ещё не созданного destination выбирается ближайший существующий ancestor, затем берётся `f_bavail * f_frsize`; `check_migration_budget_at_path` сразу сравнивает это значение с консервативным бюджетом и резервом. Тест проверяет существующий каталог и новый вложенный путь. Migration suite теперь `21 passed, 0 failed`, строгий Clippy прошёл.

Это закрывает измерение места на caller boundary, но не превращает core в готовую миграцию: full upgrade startup wiring и публичные команды ещё не подключены; read-only incomplete-journal guard уже подключён, final bundle ID не выбран, реальный upgrade не выполнялся, пользовательские данные не читались. Текущий полный gate на fingerprint `784c1510776342e5f166e47fca4794e7c956d625ae3d0cdafa78ccf84fd5ba4a` прошёл: Node `402`, Vitest `61`, critical Playwright `26`, Rust `458 passed / 0 failed / 5 ignored`, Clippy, production build и packaged FFmpeg recovery `1/1`.


## Startup guard 2026-09-21

`ensure_no_incomplete_migrations` перечисляет operation directories под `app_data_dir/migration`, отвергает symlink/non-file journals и несоответствие UUID каталога журналу, разрешает только фазу `completed`, а при `preparing`, `prepared`, `publishing` или `verified` останавливает database startup до открытия профиля. Проверки read-only: missing/completed operation разрешены, incomplete operation отклоняется; migration suite `23 passed, 0 failed`, strict Clippy PASS. Полный gate на source fingerprint `e9416fc4b71583fe965ffa1dfaaf38423bf41409314c8a12e4ffb11933f346b3` прошёл: Node `402`, Vitest `61`, Playwright `26`, Rust `460 passed, 0 failed, 5 ignored`, production build и packaged FFmpeg recovery `1/1`.

Это закрывает startup fail-closed guard, но не является публичным upgrade: final identity не выбрана, Tauri-команды export/import не подключены, старые и новые профили на отдельной установке не проверялись.


## Publish and readback primitive 2026-09-21

The product migration core now has a bounded publication step after staging. `publish_manifest` validates the journal, requires a real staging root and real destination directory, copies only manifest-listed target-relative files with create-new semantics, rechecks that each source remains inside staging, verifies each copied SHA-256/length, and syncs the files/directories. An existing destination returns `DestinationExists` and is left unchanged. `verify_published_manifest` performs a separate canonical-path, size, and SHA-256 readback. The caller still owns the migration lock. Focused migration tests are `24 passed, 0 failed`; strict Clippy passes.

The full gate on dirty fingerprint `36bcf7a6e16df7c4f3a275b9c68832cc86a10c7650f29616c0ec02021dd8d4ab` passed with Node `402`, Vitest `61`, critical Playwright `26`, Rust `461 passed, 0 failed, 5 ignored`, production build, Clippy, and packaged FFmpeg recovery `1/1`. This closes the reusable staging/publication primitive, not the user-facing upgrade: Tauri export/import commands, final bundle identity, and a real old-to-new bundle-ID run remain unverified.

## Atomic publication and resume hardening 2026-09-21

`copy_and_verify` теперь пишет в UUID-временный sibling, делает `sync_all`, сверяет длину и SHA-256, а затем публикует через `hard_link`; при сбое финальный destination не бывает частичным и cleanup удаляет только собственный временный файл. `publish_manifest` и `resume_published_manifest` проверяют каждый parent как реальный каталог, отбрасывают symlink, не перезаписывают существующие файлы и позволяют безопасно завершить публикацию после уже совпавших entries. `update_journal_phase` сохраняет только монотонные фазы через уникальный temp и atomic rename; lock остаётся обязанностью caller. Фокусная migration suite прошла `30 passed, 0 failed`, включая эти race/symlink/uppercase-SHA/resume проверки; strict Clippy PASS.

Это усиливает reusable core, но не закрывает FORK-037: public Tauri export/import commands, final bundle identity, user-facing WebView/SQLite round-trip и реальный старый-to-новый bundle-ID upgrade ещё не проверены. Текущий Universal diagnostic package привязан к dirty fingerprint `f06a6f8d…`, bundle `a1c1830c…`, DMG `33a3e4c0…`; full gate PASS (`402` Node, `61` Vitest, `26` Playwright, `467` Rust, `5` ignored, FFmpeg `1/1`).

## Tauri boundary 2026-09-21

Добавлены три зарегистрированные команды: `prepare_profile_migration`, `publish_profile_migration` и `get_profile_migration_status`. Prepare принимает явный source/target identity, список файлов с длиной и SHA-256, source roots и бюджет места; target root всегда берётся из `app.path().app_data_dir()`. Journal и lock остаются под `app_data_dir/migration`, а staging лежит в отдельном sibling-каталоге на той же файловой системе, чтобы не пересекаться с destination. Publish переводит журнал через `publishing → verified → completed`, принимает только совпадающие уже опубликованные файлы и после сбоя продолжает через `resume_published_manifest`. Командный тест проверяет полный prepare → publish → повторный вызов; migration group теперь `33 passed, 0 failed`, strict Clippy PASS.

Команды закрывают Tauri boundary reusable migration protocol, но не объявляют реальный upgrade: WebView export/import ещё нужно вызвать из UI-потока с выбранной final identity, старый и новый профиль не запускались рядом, пользовательские данные не читались, а FORK-028 оставляет окончательные каталоги и bundle ID неопределёнными.
