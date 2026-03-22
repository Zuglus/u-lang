# CLAUDE.md — контекст для Claude Code

## Проект

U Language — Kotlin для Rust. Удобный язык поверх Rust-экосистемы. `.u` и `.rs` в одном проекте.

## Спецификация

`spec/SPEC.md` (v1.1). Примеры в `examples/`.

## Архитектура

```
.u файл → [Parser/pest] → AST → [Interpreter]   → результат      (u run, 0.01s)
                              → [Generator]    → .rs → rustc → бинарник (u build, нативная скорость)
```

Два инструмента, один код:
- `u run` — интерпретатор AST, std::thread для spawn, встроенный Sqlite
- `u build` — транспиляция в Rust, tokio для async, компиляция через rustc

## Ключевые решения

- **Нет memory(auto/own/no)**: просто `.u` и `.rs` файлы
- **`::` для мутации**: `.` — чтение/команда, `::` — изменение байтов в RAM
- **`end`-блоки**: вместо `{}`
- **4 модификатора**: `unsafe`, `weak`, `test`, `pub`
- **Лямбда**: одно выражение `fn(x) x > 0`
- **`?`** пробрасывание ошибок, **`!`** force-unwrap (panic)
- **`$`** строковая интерполяция, **`$(expr)`** для выражений
- **Return**: явный. Нет return = процедура
- **Типы**: `pub` fn — с типами и `->`, остальные — без
- **Spawn безопасен**: catch_unwind, `::` запрещён в spawn
- **Именование**: заглавная = тип (`Sqlite`, `Channel`), строчная = функция (`print`, `read_file`)

## Четыре уровня доступности

1. **Язык** — `Int`, `String`, `Bool`, `List`, `Result`, `none`, `true`, `false`
2. **Ядро** (без use) — `print`, `read_file`, `write_file`, `list_dir`, `create_dir`, `sleep`, `range`, `len`, `str`, `int`
3. **Стандартная** (`use std.xxx`) — `Sqlite`, `Router`, `Channel`, `Args`, `Json`, `Regex`, строковые утилиты
4. **Внешнее** (`use`) — `.rs` модули, crates.io

## Команды

```
cd transpiler
cargo build                     # собрать
cargo test                      # тесты (12 штук)
cargo install --path .          # установить как `u`
u run examples/hello.u          # интерпретатор (0.01s)
u build examples/hello.u        # компиляция (нативная скорость)
u check examples/hello.u        # только парсинг (AST)
```

## Работающие примеры (12)

1. `hello.u` — строковая интерполяция, print
2. `calc.u` — fn, if, for, return, рекурсия
3. `shapes.u` — struct, enum, match =>, `::` мутация
4. `todo_cli.u` — Sqlite, Args, match, `?` ошибки
5. `workers.u` — spawn, loop, Channel, ch.send/recv
6. `server.u` — HTTP-сервер (TcpListener), spawn, keep-alive
7. `fault_tolerance.u` — spawn ловит паники, `!` force-unwrap
8. `race_check.u` — безопасный счётчик через канал
9. `spawn_safety.u` — запрет `::` в spawn
10. `sitegen.u` — маркдаун→HTML генератор, файловый I/O
11. `objects.u` — impl, trait, методы через `.` и `::`
12. `server_router.u` — Router API, 35K req/sec

## Приоритет работы

1. Интерпретатор: расширение поддержки конструкций (u run)
2. Парсер: новые конструкции из спецификации
3. Генератор: AST → Rust-код + валидация (u build)
4. Стандартная библиотека: std.http, std.json, std.regex
5. Маппинг ошибок: rustc → .u файл (базовый работает)

## Структура транспилятора

```
transpiler/src/
  main.rs          — CLI (clap): u run / u build / u check
  lib.rs           — pub mod ast, parser, generator, interpreter
  ast.rs           — AST типы (Program, Stmt, Expr, Value...)
  parser.rs        — pest парсер → AST
  u.pest           — PEG грамматика
  generator.rs     — AST → Rust-код (для u build)
  interpreter.rs   — AST → прямое выполнение (для u run)
```

## Интерпретатор (interpreter.rs)

Встроенные типы: `Int`, `Float`, `Str`, `Bool`, `List`, `Struct`, `Variant`, `Channel`, `Db`, `Type`, `None`.
Конкурентность: `std::thread::spawn` + `std::sync::mpsc` + `Arc<Mutex<>>`.
Sqlite: `rusqlite` (bundled). Паники: `catch_unwind` в spawn.
