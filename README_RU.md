[English](README.md) | **Русский**

# Тестовый плагин House Builder

Плагин для проверки системы плагинов BIMCraft. Одна кнопка строит домик обычными командами BIMCraft:
четыре стены, пол, дверь, окно, помещение, крышу и текстовую заметку. Всё это — **один шаг undo**;
если какая-то команда не прошла, плагин возвращает ошибку и откатывает весь вызов.

> **Важно:** для кнопки **Open File** нужна системная BIMCraft ≥ 0.10.0 (в ABI 1 есть разрешение
> `files` и проверка его песочницей). Установленный в этой системе `/usr/bin/bimcraft` от 3 ноября 2026 —
> это 0.10.0, но он **старше** этой сборки: в нём разрешения `files` ещё нет, и кнопка ответит
> `unknown command`. Для теста песочницы запускайте проверочный сборочный вариант (см. «Установка»).

> **Важно:** BIMCraft **не исполняет `src/lib.rs`**. Плагин — это `plugin.json` плюс собранный
> **WebAssembly-модуль** (`house_builder.wasm`), который указывается в поле `module`.
> Просто положить `.rs`-файл в папку плагинов недостаточно — он не загрузится.
>
> В этой папке готовый `house_builder.wasm` **уже лежит**, поэтому для установки и запуска
> **собирать ничего не нужно**: достаточно скопировать папку. Сборка нужна только если вы
> меняете `src/lib.rs` — см. раздел «Пересборка модуля».

## Нужно ли собирать?

| Что вы делаете                                            | Нужна сборка?                                              |
| --------------------------------------------------------- | ---------------------------------------------------------- |
| Просто устанавливаете этот плагин и нажимаете Build House | **Нет** — в папке уже есть `house_builder.wasm`            |
| Правите `src/lib.rs` или `Cargo.toml`                     | Да — пересоберите `.wasm` и положите рядом с `plugin.json` |
| Берёте папку на другую машину                             | Нет — копируйте папку целиком, `.wasm` уже внутри          |
| Кладёте в папку плагинов только `src/lib.rs`              | Не заработает: нужен `plugin.json` и `.wasm`               |

## Что внутри

```text
test-plugin-house/
├── plugin.json          манифест: id house-builder, ABI 1, три функции, разрешение files, вкладка Test Plugins
├── house_builder.wasm   собранный модуль (wasm32-unknown-unknown, release)
├── Cargo.toml           исходник плагина (отдельный cdylib, SDK из checkout BIMCraft)
├── scripts/
│   ├── build-wasm.sh    запускает сборку (оставляет окно терминала открытым)
│   ├── build-wasm.py    интерактивная сборка .wasm из src/lib.rs
│   └── test-plugin.sh   прогоняет собранный плагин через bimcraft-cli
└── src/lib.rs           функции build, check и open_file
```

Функции:

| Команда                      | Что делает                                                                                        |
| ---------------------------- | ------------------------------------------------------------------------------------------------- |
| `plugin:house-builder:build` | строит дом; параметры `at`, `width`, `depth`, `height`, `room` (все необязательные)               |
| `plugin:house-builder:check` | читает модель и возвращает **текстовый отчёт** о её составе (поля `report` и `counts`); в окне отчёт открывается в диалоге; undo-шага не делает |
| `plugin:house-builder:open_file` | **Open File** — открывает файл проекта (`.bimcraft` или `.ifc`) по абсолютному пути из любой папки. Параметр `path` обязателен. Требует разрешения `files` (его проверяет песочница). Возвращает `report`, `path`, `name` (имя проекта) и `counts`. Открытие заменяет проект, поэтому это не undo-шаг |

### Тест песочницы: кнопка Open File

Кнопка проверяет, что песочница пускает плагин к диску **только с разрешением**. В `plugin.json`
добавлено `"permissions": ["files"]` — это **запрос**, а не разрешение: разрешение даёт пользователь
в **Manage > Plugins** (оно хранится в `state.json` папки плагинов, в CLI его нет).

1. **Без разрешения** нажмите **Open File** — вызов упадёт с сообщением
   `` `open` needs to read files, and the plugin isn't given the files permission ``.
2. Выдайте плагину «Files» в **Manage > Plugins** и нажмите **Open File** с путём к файлу
   (в поле параметра; CLI: `--cmd 'plugin:house-builder:open_file {"path":"/абсолютный/путь.bimcraft"}'`).
3. Откроется диалог с отчётом `OPENED <путь> / project: <имя> / elements: <N>`; проект заменится файлом.

Что стоит проверить: файл в чужой папке (например, `/tmp`, домашней папке), несуществующий путь,
файл не-(bimcraft/ifc) расширения, и что `build`/`check` без разрешения по-прежнему работают.

### Отчёт кнопки Check House

Функция `check` возвращает JSON со строковым полем `report` — готовым для человека текстом:

```json
{"report": "MODEL REPORT - …", "counts": {"wall": 4, "floor": 1, "roof": 1, "door": 1, "window": 1, "room": 1}}
```

Если у ответа плагиновой команды есть строковое поле `report`, десктоп открывает его в диалоге
с заголовком в виде подписи кнопки (текст плагина не переводится) и кнопками **Copy** / **Close**;
в CLI и MCP это просто JSON в ответе команды. Отчёт `check` содержит: имя проекта и его стандарт,
уровни с отметками, элементы по категориям, что стоит на каждом уровне, помещения с площадями и
признаком enclosed, стены (общая длина, высота, типы), двери и окна (метки, типы, размеры), полы
с площадью, крыши, текстовые заметки, размер библиотеки (сколько типов, материалов, семейств и
параметров), предупреждения и число шагов истории.

## Установка

### Вариант 1: указать папку переменной окружения (быстро, без копирования)

`BIMCRAFT_PLUGINS` может указывать прямо на папку плагина (в ней лежит `plugin.json`):

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/BIMCraft-plugins-clean
BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house \
  cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:build {}'
```

Для десктопа то же самое перед запуском:

```sh
BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house \
  cargo run -q -p bimcraft
```

Пустое значение `BIMCRAFT_PLUGINS=` отключает плагины полностью.

### Вариант 2: положить в обычную папку плагинов (как ставит пользователь)

Скопируйте всю папку `test-plugin-house` в папку плагинов пользователя:

```sh
# Linux
mkdir -p ~/.config/bimcraft/plugins
cp -r /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house ~/.config/bimcraft/plugins/

# macOS
mkdir -p ~/Library/Application\ Support/BIMCraft/plugins
cp -r /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house ~/Library/Application\ Support/BIMCraft/plugins/
```

```powershell
# Windows (PowerShell)
New-Item -ItemType Directory -Force "$env:APPDATA\BIMCraft\plugins"
Copy-Item -Recurse "C:\путь\test-plugin-house" "$env:APPDATA\BIMCraft\plugins\"
```

Если `XDG_CONFIG_HOME` задан, на Linux папка плагинов — `$XDG_CONFIG_HOME/bimcraft/plugins`.
После копирования перезапустите приложение или нажмите **Refresh** в **Manage > Plugins**.

Проверка установки в CLI:

```sh
bimcraft-cli commands | grep house-builder
```

В списке должны быть `plugin:house-builder:build` и `plugin:house-builder:check` с полем `"plugin": "house-builder"`.

## Как проверить в интерфейсе

1. Запустите десктоп (с `BIMCRAFT_PLUGINS=...` или после копирования в папку плагинов).
2. Откройте **Manage > Plugins**. В списке будет `House Builder (test) 0.1.0`, статус **Loaded**,
   автор `BIMCraft test`, три функции (текст `asks to read and write files` — это запрос `files`); ниже — журнал загрузки.
3. На ленте появится вкладка **Test Plugins** с панелью **House** и кнопками **Build House** и **Check House**.
4. Нажмите **Build House** — на плане появятся стены, пол, дверь, окно, помещение и крыша;
   в строке состояния ошибок быть не должно. В журнале плагина: `building a 8000 x 6000 mm house at 0, 0`
   и `the house is built: one undo step`.
5. Нажмите **Ctrl+Z** (Undo) — весь дом исчезнет за один шаг с подписью
   `House Builder (test): Build House`.
6. **Check House** откроет диалог **Check House** с текстовым отчётом о модели: уровни с отметками,
   элементы по категориям, что стоит на каждом уровне, помещения с площадями, стены с общей длиной,
   двери и окна с размерами, полы, крыши, заметки, библиотека, предупреждения и шаги истории
   (для только что построенного дома — `Walls 4 · Floors 1 · Doors 1 · Windows 1 · Rooms 1` и т. д.).
   Undo-шага не будет; в **Manage > Plugins** в журнале появится `the model report has N lines`.
7. В **Manage > Plugins** нажмите **Disable** — вкладка **Test Plugins** и команды исчезнут сразу;
   **Enable** вернёт их. **Refresh** перечитывает папку без перезапуска.

## Те же проверки в CLI

```sh
export BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/BIMCraft-plugins-clean

# 1. Пустой проект: проверка до постройки
cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:check'

# 2. Постройка (значения по умолчанию: 8000 x 6000 мм в [0, 0])
cargo run -q -p bimcraft-cli -- run --cmd 'plugin:house-builder:build {}'

# 3. Свои размеры и имя помещения
cargo run -q -p bimcraft-cli -- run \
  --cmd 'plugin:house-builder:build {"at":[1000,2000],"width":5000,"depth":4000,"room":"Studio"}'

# 4. Один шаг undo: после undo счётчики снова нулевые
cargo run -q -p bimcraft-cli -- run \
  --cmd 'plugin:house-builder:build {}' --cmd 'undo' --cmd 'plugin:house-builder:check'

# 5. Сохранить проект и посмотреть план
cargo run -q -p bimcraft-cli -- run \
  --cmd 'plugin:house-builder:build {}' --save /tmp/house.bimcraft
cargo run -q -p bimcraft-cli -- run /tmp/house.bimcraft \
  --cmd 'render {"path":"/tmp/house.png","width":900,"height":600}'
```

Ожидаемые ответы для дома 8000 × 6000:

- `build` возвращает `size`, ids стен (`walls.ids` — 4), пол, дверь, окно, помещение, крышу и заметку;
  площадь пола и помещения — 48 м².
- `check` возвращает `{"report": "<текст отчёта>", "counts":{"wall":4,"floor":1,"roof":1,"door":1,"window":1,"room":1}}`
  (текст отчёта многострочный; в CLI его видно в JSON ответа, в десктопе он открывается в диалоге).
- `undo` — `{"undone":"House Builder (test): Build House","count":1}`.

## Через MCP

Инструмент `execute` существующего MCP-сервера принимает команду плагина; отдельных инструментов
для плагинов нет:

```sh
BIMCRAFT_PLUGINS=/media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house \
  cargo run -q -p bimcraft-cli -- mcp
```

Затем, например, `tools/call` с `{"name":"execute","arguments":{"command":"plugin:house-builder:build","params":{}}}`.
`list_commands` покажет плагиновые команды с `"plugin":"house-builder"`.

## Тест собранного плагина

`scripts/test-plugin.sh` проверяет **уже собранный** модуль: он берёт `bimcraft-cli` из checkout-а
BIMCraft, подставляет `BIMCRAFT_PLUGINS` на папку плагина и гоняет `check` в четырёх сценариях —
пустой проект, только что построенный дом, `--sample` и `--highrise` (20 этажей, около 10 000 элементов), —
а потом печатает таблицу и журнал плагина для упавших сценариев:

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
./scripts/test-plugin.sh
```

Скрипт интерактивный: спросит папку плагина и checkout BIMCraft (Enter — значение по умолчанию),
ничего не собирает и не меняет. Двойной клик тоже работает. Если `.wasm` старше `src/lib.rs`,
он об этом предупредит.

Ожидаемая картина — **4 из 4**. Если сценарий падает с `all fuel consumed by WebAssembly`, значит
плагин упёрся в топливо песочницы: разбор ответа стоит примерно 0,7 тыс. топлива на байт, то есть
20 млн — это около 25 КБ ответов за вызов. Отчёт поэтому зовёт `inspect {"brief": true}` (уровни,
счётчики, предупреждения, история и **размер** библиотеки — четверть килобайта вместо девятнадцати)
и сокращает списки элементов на большой модели. Подробности — в
`BIMCraft-plugins-clean/docs/areas/plugins.md`, раздел «Где именно упал вызов»: там же описана строка
журнала `the call ran 24 command(s) and used … M fuel; the last steps, and the fuel left when each ran: …`,
по которой видно, на что ушло топливо.

## Пересборка модуля

Готовый `.wasm` уже лежит в папке. Пересобрать после правок `src/lib.rs` можно скриптом
(он сам спросит папку плагина, target, release и куда положить готовый модуль):

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
./scripts/build-wasm.sh
```

`build-wasm.sh` — тонкая обёртка: она запускает `scripts/build-wasm.py` и не закрывает
терминал после успеха или ошибки. Можно запустить и двойным кликом. Python-скрипт
интерактивный: Enter принимает значения по умолчанию. Если target `wasm32-unknown-unknown`
не установлен, он предложит выполнить `rustup target add`.

Эквивалент вручную:

```sh
cd /media/kat/Seagate5-2/00_Opensource/27_BIMCraft-0100/test-plugin-house
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/house_builder.wasm ./house_builder.wasm
```

`scripts/build-wasm.py` читает имя пакета из `Cargo.toml`, находит
`target/<target>/release/<имя>.wasm`, проверяет сигнатуру WebAssembly и копирует файл
в путь из `"module"` в `plugin.json` (`house_builder.wasm`). Скрипт требует Python 3.10+
и стандартную библиотеку — сторонних пакетов нет.

`Cargo.toml` берёт SDK по относительному пути `../BIMCraft-plugins-clean/crates/plugin-sdk`.
Если папка плагина перенесена, поправьте `path` на свой checkout или соберите плагин рядом с ним.

## Если что-то не так

- **Плагина нет в списке.** Проверьте `plugin.json` в папке и путь `BIMCRAFT_PLUGINS`; причина
  отказа всегда пишется в **Manage > Plugins** и в stderr CLI.
- **Ошибка `there's no type ...` / дверь или окно не ставятся.** Плагин пользуется типами проекта по
  умолчанию; в проекте без стандартных типов двери/окна создайте их или укажите тип вручную.
- **Крыша не строится.** Крыша ставится над стенами уровня ниже; если стены уже есть на другом уровне
  или проект пуст, постройте дом в новом проекте (`--cmd 'new {}'`).
- **Дом строится поверх старого.** Плагин не очищает модель: используйте новый проект или другой `at`.
- Без разрешения `files` кнопка **Open File** отвечает `... isn't given the files permission` — это и есть работа песочницы. `build` и `check` разрешения не требуют.

Папка плагина лежит вне git-репозитория и не входит в PR #256; это отдельный тестовый артефакт.



Да, всё верно: сам по себе .rs не работает. BIMCraft не компилирует Rust на лету — он загружает только готовый WebAssembly-модуль, указанный в манифесте:
"module": "house_builder.wasm"
То есть плагин — это два обязательных файла:
plugin.json          манифест
house_builder.wasm   собранный модуль
src/lib.rs и Cargo.toml — это исходники для сборки, приложение их не читает.
При этом в test-plugin-house уже лежит собранный house_builder.wasm (107 510 байт), поэтому для установки и запуска собирать ничего не нужно — достаточно скопировать папку в папку плагинов.
Если оставить в папке только .rs, будет ошибка загрузки: модуль house_builder.wasm не найден, плагин попадёт в Manage > Plugins со статусом ошибки, а его команды не зарегистрируются.