# CLAUDE.md — контекст для Claude Code

## Проект

U Language — Kotlin для Rust. Удобный язык поверх Rust-экосистемы. `.u` и `.rs` в одном проекте.

## Спецификация

`spec/SPEC.md` (v2.1). Примеры в `examples/`.

## Архитектура

```
.u файл → [Parser/pest] → AST → [Generator] → .rs → rustc → бинарник
```

- `u run` — компиляция + запуск + кэш (0.01 сек повторно)
- `u build` — компиляция через Rust, нативная скорость
- `u test` — запуск test fn
- `u check` — только парсинг (AST)
- `u fmt` — форматирование

## Ключевые решения

- **`mut` для мутации**: `mut` в определении функции/метода, при вызове — всегда `.`
- **`end`-блоки**: вместо `{}`
- **5 модификаторов**: `pub`, `test`, `mut`, `unsafe`, `weak`
- **Лямбда**: `x => expr`, несколько параметров `(x, y) => expr`
- **`!`** force-unwrap (panic). `?` убран.
- **`$`** строковая интерполяция, **`$(expr)`** для выражений
- **Return**: явный. Нет return = процедура
- **Типы**: `pub` fn — с типами и `->`, остальные — без
- **Spawn безопасен**: паники ловятся, `mut` на внешних данных запрещён
- **Именование**: заглавная = тип (`Sqlite`, `Channel`), строчная = функция (`print`, `read_file`)
- **30 ключевых слов**: fn, return, if, elif, else, end, for, in, while, loop, break, continue, struct, impl, trait, enum, match, pub, test, unsafe, weak, use, spawn, and, or, not, true, false, none, mut

## Четыре уровня доступности

1. **Язык** — `Int`, `String`, `Bool`, `List`, `Result`, `none`, `true`, `false`
2. **Ядро** (без use) — `print`, `read_file`, `write_file`, `list_dir`, `create_dir`, `sleep`, `range`, `len`, `str`, `int`
3. **Стандартная** (`use std.xxx`) — `Sqlite`, `Router`, `Channel`, `Args`, `Json`, `Regex`, строковые утилиты
4. **Внешнее** (`use`) — `.rs` модули, crates.io

## Команды

```
cd transpiler
cargo build                     # собрать
cargo test                      # тесты (34 штуки)
cargo install --path .          # установить как `u`
u run examples/hello.u          # компиляция + кэш (0.01s)
u build examples/hello.u        # компиляция → нативный бинарник
u check examples/hello.u        # только парсинг (AST)
u test examples/test_demo.u     # запуск test fn
u fmt examples/hello.u          # форматирование
```

## Работающие примеры

1. `hello.u` — строковая интерполяция, print
2. `calc.u` — fn, if, for, return, рекурсия
3. `shapes.u` — struct, enum, match =>, мутация через `.`
4. `todo_cli.u` — Sqlite, Args, match, `?` ошибки
5. `workers.u` — spawn, loop, Channel, ch.send/recv
6. `server.u` — HTTP-сервер (TcpListener), spawn, keep-alive
7. `fault_tolerance.u` — spawn ловит паники, `!` force-unwrap
8. `race_check.u` — безопасный счётчик через канал
9. `spawn_safety.u` — запрет `mut` в spawn
10. `sitegen.u` — маркдаун→HTML генератор, файловый I/O
11. `objects.u` — impl, trait, методы через `.`
12. `server_router.u` — Router API, 50K+ req/sec

## Приоритет работы

1. Генератор: расширение поддержки конструкций (u build/run)
2. Парсер: новые конструкции из спецификации
3. Стандартная библиотека: std.http, std.json, std.regex
4. Маппинг ошибок: rustc → .u файл (базовый работает)

## Структура транспилятора

```
transpiler/src/
  main.rs          — CLI (clap): u run / u build / u check / u test / u fmt
  lib.rs           — pub mod ast, parser, generator, formatter
  ast.rs           — AST типы (Program, Stmt, Expr, Value...)
  parser.rs        — pest парсер → AST
  u.pest           — PEG грамматика
  generator.rs     — AST → Rust-код (для u build/run)
  formatter.rs     — u fmt форматирование
```
