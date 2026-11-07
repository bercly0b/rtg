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

На macOS и Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/bercly0b/rtg/releases/latest/download/rtg-tui-installer.sh | sh
```

Установщик кладёт `rtg` в `~/.cargo/bin` и добавляет этот каталог в `PATH`.
Чтобы установить в другое место, замените конец команды на
`| RTG_TUI_INSTALL_DIR=~/.local sh`: тогда `rtg` окажется в `~/.local/bin`.
Всё остальное — архивы для каждой платформы, контрольные суммы — в
[последнем релизе](https://github.com/bercly0b/rtg/releases/latest).

На Linux `rtg` нужна среда выполнения libc++: `sudo apt install libc++1 libc++abi1`.

Чтобы обновить `rtg`, установленный этим способом, выполните `rtg update`.
Когда выходит новый релиз, RTG показывает его в строке состояния.

Если вы ставили одну из прошлых версий через её `install.sh`, удалите старую
копию из `/usr/local`:

```bash
sudo rm -f /usr/local/bin/rtg /usr/local/lib/libtdjson.*
```

### Сборка из исходников

Что понадобится:

- [mise](https://mise.jdx.dev/installing-mise.html), активированный в вашей
  оболочке, — он установит версию Rust, закреплённую в [`mise.toml`](mise.toml)
- На Linux — libc++: TDLib вшивается в `rtg` статически, а её C++-рантайм
  берётся из системы:
  ```bash
  sudo apt install libc++-dev libc++abi-dev
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
