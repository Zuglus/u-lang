# CLAUDE.md — контекст для Claude Code

## Проект

U Language — единый язык программирования. Транспилятор: U syntax → Rust code → rustc.

## Спецификация

Полная спецификация в `spec/SPEC.md` (v0.9). Примеры в `examples/`.

## Ключевые решения

- **Четыре режима**: скрипт (по умолчанию), `memory(auto)` (RC+GC, spawn→tokio), `memory(own)` (ownership, Thread.spawn), `memory(no)` (без кучи, bare metal)
- **`::` для мутации своих байтов**: `.` — чтение или команда внешней системе (db.exec, ch.send, conn.write), `::` — изменение своих байтов в RAM (list::push, user::name = "...")
- **Скрипт по умолчанию**: нет `memory(...)` → всё автоматически, стандартная библиотека доступна
- **`end`-блоки**: вместо `{}`
- **4 модификатора**: `unsafe`, `weak`, `test`, `public`
- **Лямбда**: одно выражение: `fn(x) x > 0`
- **`?`**: пробрасывание ошибок и nullable
- **`$`**: строковая интерполяция
- **`use`**: обязателен только с `memory(...)`
- **Return**: явный. Нет return = процедура
- **Типы**: `public` fn — с типами, остальные — без
- **Скобки `()`**: обязательны при вызове функции
- **Spawn безопасен**: автоматический `catch_unwind`, паника → лог в stderr, горутина умирает, остальные живут
- **`::` запрещён в spawn**: `fn f(::data)` + `spawn f(x)` → ошибка компиляции

## Архитектура транспилятора

```
.u файл → [Parser] → AST → [Validator] → [Generator] → .rs файл → rustc → бинарник
```

Транспилятор на Rust. Парсер: pest. CLI: clap. Генератор возвращает `Result<String, String>` — валидация перед генерацией (spawn safety).

## Команды

```
cd transpiler
cargo build          # собрать транспилятор
cargo test           # запустить тесты (9 тестов)
cargo install --path .  # установить как команду `u`
u run examples/hello.u  # запустить .u файл
u build examples/hello.u  # собрать бинарник
u check examples/hello.u  # только парсинг (AST)
```

## Работающие примеры (9)

1. `hello.u` — строковая интерполяция, print
2. `calc.u` — fn, if, for, return, рекурсия, списки
3. `shapes.u` — struct, type (enum), match, `::` мутация полей
4. `todo_cli.u` — SQLite, Args, match на строках, `?` ошибки
5. `workers.u` — spawn, loop, Channel, ch.send/recv
6. `server.u` — HTTP-сервер (TcpListener), memory(auto), use, spawn
7. `fault_tolerance.u` — spawn автоматически ловит паники
8. `race_check.u` — безопасный счётчик через канал
9. `spawn_safety.u` — демонстрация запрета `::` в spawn

## Приоритет работы

1. Парсер: разбор .u файла в AST
2. Генератор: AST → Rust-код + валидация
3. CLI обёртка: `u build`, `u run`, `u check`
4. Маппинг ошибок: rustc ошибки → .u файл
5. Рантайм: crate u-runtime (Sqlite, Args, Channel, HttpServer, catch, error, read_file, mime_type, sleep)
