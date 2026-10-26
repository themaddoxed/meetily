> Public privacy copy: local user paths and contact/provenance fields were anonymized. Historical test results and original hashes do not verify this edited copy. See [publication notes](/docs/development/public-source-snapshot.md).

# Реестр рабочих деревьев

[Полный индекс](../../INDEX.md) · [правила организации](repository-map.md)

Снимок до реорганизации, 18 октября 2026. Все 18 checkout проверены через
`git worktree list --porcelain`, `git status --porcelain=v1 --untracked-files=all`
и последний commit. 13 деревьев имели локальные изменения. Число ниже — количество
путей в porcelain, включая отдельно перечисленные untracked-файлы, а не задач.
После организации корневое дерево дополнительно содержит изменения документации.

Автор последнего commit во всех перечисленных HEAD — **Local Developer**.
Это Git-авторство, а не атрибуция незакоммиченных правок или назначение исполнителя.

| Checkout | Ветка / HEAD | Локальных путей до организации | Назначение / граница |
| --- | --- | ---: | --- |
| [корень проекта](../..) | `main` / `53f4415058fb` | 209 | Текущий checkout main: продуктовый код и локальные Q&A/scanner/doc изменения; не чистый кандидат. |
| [/Users/developer/.codex/worktrees/amanu-analysis-publish/meeting-minutes](../../../../.codex/worktrees/amanu-analysis-publish/meeting-minutes) | `detached` / `0e52b3dfa8b2` | 0 | Исторический amanu analysis checkpoint; detached HEAD не означает действующую задачу. |
| [/Users/developer/.codex/worktrees/amanu-analysis/meeting-minutes](../../../../.codex/worktrees/amanu-analysis/meeting-minutes) | `detached` / `e6abb9b21e3a` | 1 | Отдельный amanu analysis checkout с локальным файлом. |
| [/Users/developer/.codex/worktrees/feat071-fork051-integration/meeting-minutes](../../../../.codex/worktrees/feat071-fork051-integration/meeting-minutes) | `detached` / `61e890f444f8` | 0 | Чистый исторический checkpoint совмещённых audio/lifecycle работ. |
| [.worktrees/alpha-041-mdtask](../../.worktrees/alpha-041-mdtask) | `docs/alpha-041-mdtask` / `8ea895975c25` | 1 | Документальная декомпозиция задач. |
| [.worktrees/dev-integration-2026-10-03](../../.worktrees/dev-integration-2026-10-03) | `dev` / `53f4415058fb` | 0 | Чистая интеграционная база dev на момент снимка. |
| [.worktrees/feat071-audio-controls](../../.worktrees/feat071-audio-controls) | `themaddoxed/feat071-audio-controls` / `61e890f444f8` | 60 | Исходник работ по управлению аудио; локальные изменения сохранены. |
| [.worktrees/fork-051-permissions-lifecycle](../../.worktrees/fork-051-permissions-lifecycle) | `fork/fork-051-permissions-lifecycle` / `f35d864370cb` | 14 | Permissions/lifecycle; есть staged и unstaged изменения. |
| [.worktrees/fork-first-batch](../../.worktrees/fork-first-batch) | `feature/fork-first-batch` / `fcfa6e0ba4b0` | 0 | Историческая первая группа переноса. |
| [.worktrees/hig-agent-skills](../../.worktrees/hig-agent-skills) | `feature/hig-agent-skills` / `8ffc5e6de85d` | 159 | Исторический HIG/Universal checkpoint; локальные файлы сохранены. |
| [.worktrees/macos-fork-foundation](../../.worktrees/macos-fork-foundation) | `themaddoxed/macos-fork-foundation-safe` / `f6911b3993f1` | 169 | Foundation macOS fork с локальным overlay. |
| [.worktrees/macos-fork-integration](../../.worktrees/macos-fork-integration) | `fork/macos-integration` / `b7126ea82be8` | 0 | Историческая macOS integration line. |
| [.worktrees/main-features-2026-10-02](../../.worktrees/main-features-2026-10-02) | `integration/main-features-2026-10-02` / `53f4415058fb` | 17 | Исходник интеграции в текущий main; локальный overlay остаётся. |
| [.worktrees/main-overlay-preserved-2026-10-02](../../.worktrees/main-overlay-preserved-2026-10-02) | `salvage/main-overlay-2026-10-02` / `952e3614fe27` | 175 | Salvage старого main overlay; не считать лишней копией. |
| [.worktrees/quality-toolchain-protocol](../../.worktrees/quality-toolchain-protocol) | `themaddoxed/quality-protocol-clippy` / `e9dd56e4e350` | 2 | Работы по протоколу проверок/toolchain. |
| [.worktrees/semantic-theme-2026-10-03](../../.worktrees/semantic-theme-2026-10-03) | `feature/semantic-theme-2026-10-03` / `53f4415058fb` | 13 | Источник ещё не закоммиченной semantic theme. |
| [.worktrees/task-decomposition-publish](../../.worktrees/task-decomposition-publish) | `themaddoxed/task-decomposition-plan` / `605021c229de` | 12 | Декомпозиция/подготовка документации; имя ветки не доказывает публикацию. |
| [.worktrees/visible-meeting-search](../../.worktrees/visible-meeting-search) | `feature/visible-meeting-search` / `6f2c4488edc6` | 22 | Исходник работ по видимому поиску; свежий статус интеграции проверять по diff. |

## Сохранённые результаты и переносы

- Q&A: [точный manifest](../fork/evidence/qa-transfer-fixed-2026-10-04.json), локальные файлы в корне; не отдельный destination commit.
- Theme: [исторический отчёт](../fork/evidence/theme-project-full-2026-10-04.json), исходник в `semantic-theme-2026-10-03`; не считать результатом текущего main.
- Интеграционные backup, patch и report: [integration-backups-2026-10-02](../../.worktrees/integration-backups-2026-10-02). Это папка резервных материалов, не зарегистрированный Git worktree.
- Полные HEAD, исходные статусы и авторство: [snapshot JSON](evidence/2026-10-18-workspace-before.json).

Ни одно дерево или Git ref не переносилось, не удалялось и не переключалось.
Для продолжения сначала повторить read-only status нужного checkout. Исторические
имена веток показывают назначение, но не дают разрешения интегрировать весь diff.
Наличие чистого checkout не подтверждает автоматические тесты или native-приёмку.
