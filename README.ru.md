<p align="center">
  <a href="README.md">English</a> | <b>Русский</b>
</p>

<h1 align="center">RTG</h1>

<h3 align="center">
  Быстрый TUI-клиент Telegram в стиле vim, написанный на Rust.
</h3>

<div align="center">

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Built with ratatui](https://img.shields.io/badge/built%20with-ratatui-success)](https://github.com/ratatui/ratatui)
[![Powered by TDLib](https://img.shields.io/badge/powered%20by-TDLib-blue)](https://github.com/tdlib/td)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/bercly0b/rtg)](https://github.com/bercly0b/rtg/releases/latest)

</div>

RTG — быстрый Telegram-клиент для терминала с управлением с клавиатуры,
построенный на [TDLib](https://core.telegram.org/tdlib). Он сводит Telegram к
сути — чатам и сообщениям. И всё. Никаких историй. Никакой рекламы. Никаких
уведомлений. Наслаждайтесь тишиной.

Проект на ранней стадии разработки: основная работа с чатами уже готова, новые
возможности появляются регулярно.

<p align="center">
  <img src="docs/assets/readme-demo.gif" alt="Демо RTG: навигация по списку чатов Telegram клавишами в стиле vim, поиск чатов, чтение сообщений и запись голосового сообщения в терминале" width="800">
</p>

## Возможности

- TUI-интерфейс на [ratatui](https://github.com/ratatui/ratatui) и `crossterm`
- Бэкенд Telegram на TDLib с сохранением сессии
- Список чатов в реальном времени со статусом подключения и счётчиками непрочитанных
- Чтение, отправка, ответы, редактирование, удаление и копирование сообщений
- Реакции на сообщения
- Запись голосовых сообщений (через настраиваемую команду `ffmpeg`)
- Загрузка файлов с ограничением размера для автозагрузки
- Открытие вложений через собственные обработчики MIME-типов (в стиле mailcap)
- Настраиваемые контекстно-зависимые сочетания клавиш
- Оптимистичный UI: мгновенный отклик на действия пользователя

## Установка

### Готовые сборки

Скачайте архив для своей платформы из
[последнего релиза](https://github.com/bercly0b/rtg/releases/latest):

| Платформа | Архив |
|---|---|
| macOS Apple Silicon | `rtg-<version>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `rtg-<version>-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `rtg-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `rtg-<version>-aarch64-unknown-linux-gnu.tar.gz` |

Затем распакуйте его и запустите установщик:

```bash
tar xzf rtg-<version>-<target>.tar.gz
cd rtg-<version>-<target>
./install.sh          # устанавливает в /usr/local — на Linux может понадобиться sudo
```

Чтобы установить в другой каталог (например, `~/.local`):

```bash
INSTALL_PREFIX=~/.local ./install.sh
```

**Linux:** входящей в комплект TDLib нужна среда выполнения libc++:

```bash
sudo apt install libc++1 libc++abi1
```

### Сборка из исходников

Что понадобится:

- [mise](https://mise.jdx.dev/installing-mise.html), активированный в вашей
  оболочке, — он установит версию Rust, закреплённую в [`mise.toml`](mise.toml)
- На Linux — среда выполнения libc++, которую использует готовая сборка TDLib:
  ```bash
  sudo apt install libc++1 libc++abi1 libunwind8
  ```

TDLib (~50 МБ) скачивается автоматически при первой сборке через фичу
`download-tdlib` крейта `tdlib-rs`. Поддерживаемые платформы: Linux
x86_64/arm64, macOS Intel/Apple Silicon, Windows x86_64/arm64. Для других
платформ см. [инструкцию по сборке TDLib](https://tdlib.github.io/td/build.html).

```bash
git clone https://github.com/bercly0b/rtg.git
cd rtg
mise install
cargo build --release
./target/release/rtg
```

## Настройка

RTG читает настройки из `~/.config/rtg/config.toml`
(или из `$XDG_CONFIG_HOME/rtg/config.toml`, если переменная задана).

При первом запуске RTG попросит ключи Telegram API (`api_id` и `api_hash`) —
получить их можно на [my.telegram.org/apps](https://my.telegram.org/apps).
После ввода RTG создаст файл конфигурации по указанному выше пути и сохранит в
нём ключи.

Дальше в этом файле можно настроить остальные параметры — логирование, запись
голосовых, лимиты загрузки, обработчики MIME-типов и сочетания клавиш. Полный
список параметров — в [`config.example.toml`](config.example.toml).

## Участие в разработке

Мы рады любой помощи. Перед тем как открыть pull request, прочитайте
[CONTRIBUTING.md](CONTRIBUTING.md) (на английском), а задачи, где нужна помощь,
ищите среди [открытых issues](https://github.com/bercly0b/rtg/issues).

## Лицензия

[MIT](LICENSE)
