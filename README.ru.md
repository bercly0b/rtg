<p align="center">
  <a href="README.md">English</a> | <b>Русский</b>
</p>

<h1 align="center">RTG — Telegram TUI client in Rust</h1>

<h3 align="center">
  Быстрый vim-like клиент Telegram (CLI/TUI) на TDLib и ratatui.
</h3>

<div align="center">

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Built with ratatui](https://img.shields.io/badge/built%20with-ratatui-success)](https://github.com/ratatui/ratatui)
[![Powered by TDLib](https://img.shields.io/badge/powered%20by-TDLib-blue)](https://github.com/tdlib/td)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/bercly0b/rtg)](https://github.com/bercly0b/rtg/releases/latest)

</div>

RTG — быстрый Telegram-клиент для терминала на Linux и macOS, написанный на Rust
с [TDLib](https://core.telegram.org/tdlib). Он сводит Telegram к сути — чатам и
сообщениям. И всё. Никаких историй. Никакой рекламы. Никаких уведомлений.
Наслаждайся тишиной.

<p align="center">
  <img src="docs/assets/readme-demo.gif" alt="Демо RTG: навигация по списку чатов Telegram клавишами в стиле vim, поиск чатов, чтение и отправка сообщений, реакция на сообщение и просмотр информации о чате в терминале" width="800">
</p>

## Возможности

- Список чатов со счётчиками непрочитанных, поиском, папками и архивом
- Темы форумов
- Чтение, отправка, ответы, редактирование, удаление и копирование сообщений
- Реакции
- Голосовые сообщения, запись через `ffmpeg`
- Отправка файлов через встроенный файл-пикер
- Скачивание и сохранение файлов, лимит размера для автозагрузки
- Вложения и ссылки открываются в ваших приложениях (обработчики в стиле mailcap)
- Информация о чате и о сообщении
- Клавиши в стиле vim, любую можно переназначить
- Встроенное обновление: `rtg update`

## Установка

RTG работает на macOS и Linux.

### Скрипт установки

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/bercly0b/rtg/releases/latest/download/rtg-tui-installer.sh | sh
```

Скрипт кладёт `rtg` в `~/.cargo/bin` и добавляет этот каталог в `PATH`. Архивы
для каждой платформы и контрольные суммы — в
[последнем релизе](https://github.com/bercly0b/rtg/releases/latest).

### Homebrew

```bash
brew install bercly0b/rtg/rtg
```

### Cargo

```bash
cargo install --locked rtg-tui
```

`rtg` собирается из исходников, нужен набор инструментов Rust.

### Требования на Linux

`rtg` нужны Ubuntu 24.04 или новее (glibc 2.39) и среда выполнения libc++:

- Ubuntu 24.04: `sudo apt install libc++1 libc++abi1`
- Ubuntu 26.04 и новее: `sudo apt install libc++1 libc++abi1 llvm-libunwind1`

Для сборки через Cargo нужны ещё заголовки libc++:
`sudo apt install libc++-dev libc++abi-dev`.

## Первый запуск

1. Запустите `rtg`.
2. При первом запуске введите номер телефона и код, который пришлёт Telegram,
   затем пароль двухэтапной аутентификации, если он включён.
3. Нажмите `?`, чтобы увидеть горячие клавиши.

`rtg logout` выходит из аккаунта и удаляет локальную сессию.

## Горячие клавиши

Везде:

| Клавиша | Действие |
| --- | --- |
| `?` | Показать горячие клавиши |
| `U` | Установить обновление |
| `q` | Выйти |

В списке чатов:

| Клавиша | Действие |
| --- | --- |
| `j` / `k` | Следующий / предыдущий чат |
| `gg` | Первый чат |
| `Enter` / `l` | Открыть чат |
| `/` | Поиск по чатам |
| `f` | Выбрать папку или архив |
| `A` | Отправить чат в архив или вернуть из него |
| `r` | Отметить чат прочитанным |
| `I` | Информация о чате |

В открытом чате:

| Клавиша | Действие |
| --- | --- |
| `j` / `k` | Следующее / предыдущее сообщение |
| `G` | Последнее сообщение |
| `h` / `Esc` | Назад к списку чатов |
| `i` | Написать сообщение |
| `r` | Ответить на сообщение |
| `e` | Редактировать сообщение |
| `dd` | Удалить сообщение |
| `y` | Скопировать сообщение |
| `R` | Поставить реакцию |
| `l` | Открыть вложение |
| `o` | Открыть ссылку из сообщения |
| `A` | Прикрепить файл |
| `v` | Записать голосовое сообщение |
| `D` | Скачать файл |
| `S` | Сохранить файл в «Загрузки» |
| `m` | Информация о сообщении |

При наборе сообщения `Enter` отправляет его, `Shift+Enter` переносит строку.

Полный список — в самом RTG: нажмите `?`.

## Настройка

RTG читает настройки из `~/.config/rtg/config.toml`
(или из `$XDG_CONFIG_HOME/rtg/config.toml`, если переменная задана). Файл
необязателен.

```toml
[download]
max_auto_download_size = "10MB"

[open]
"audio/*" = "mpv {file_path}"
"image/*" = "imv {file_path}"

[attach]
dirs = ["~/Downloads", "~/Documents"]

[keys]
quit = "Q"
delete_message = "xx"
```

| Секция | Что задаёт |
| --- | --- |
| `[logging]` | Уровень логирования и число хранимых файлов логов |
| `[telegram]` | Собственные API-ключи Telegram |
| `[voice]` | Команду записи голосовых сообщений |
| `[attach]` | Где файл-пикер ищет файлы |
| `[download]` | Лимит размера для автозагрузки |
| `[open]` | Приложения для открытия вложений по MIME-типу |
| `[updates]` | Ежедневную проверку новых релизов |
| `[keys]` | Горячие клавиши |

Полный список параметров — в [`config.example.toml`](config.example.toml).

## Участие в разработке

Мы рады любой помощи. Перед тем как открыть pull request, прочитайте
[CONTRIBUTING.md](CONTRIBUTING.md) (на английском) — там же описана сборка из
исходников, — а задачи, где нужна помощь, ищите среди
[открытых issues](https://github.com/bercly0b/rtg/issues).

Если вам понравился RTG, поставьте звезду — помогите другим заметить этот
проект. Списки вроде [awesome-rust](https://github.com/rust-unofficial/awesome-rust)
принимают проекты только с достаточным числом звёзд.

## Лицензия

[MIT](LICENSE)
