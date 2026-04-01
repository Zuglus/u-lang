# U Language

Kotlin для Rust. Удобный язык поверх Rust-экосистемы, спроектированный для предсказуемой генерации ИИ.

- `.u` и `.rs` в одном проекте, один build, бесшовные вызовы
- `u run` — компиляция + кэш, 0.01 сек повторно
- `u build` — компиляция через Rust, нативная скорость
- Вся экосистема crates.io доступна
- **Архитектура: простое → сложное** — 6 уровней композиции

## Быстрый пример

```
name = "Мир"
print("Привет, $(name)!")           // унифицированная интерполяция: $(...)
```

```
fn factorial(n) -> Int              // явные типы в сигнатурах
    if n < 2
        return 1
    end
    return n * factorial(n - 1)
end

print("10! = $(factorial(10))")
```

```
// Pattern matching с понятными именами
fn sum(list) -> Int
    match list
        [] => 0
        [x] => x
        [x, ..rest] => x + sum(rest)  // rest = остальные элементы
    end
end
```

## Документация

- **[SPEC.md](spec/SPEC.md)** — полная спецификация языка
- **[FEATURES.md](FEATURES.md)** — дорожная карта фич
- **[PROGRESS.md](PROGRESS.md)** — текущий статус разработки

## Установка

```bash
git clone https://github.com/Zuglus/u-lang.git
cd u-lang/transpiler
cargo install --path .
```

## Запуск

```bash
u run examples/hello.u          # компиляция + кэш (0.01s)
u build examples/hello.u        # компиляция → нативный бинарник
u check examples/hello.u        # парсинг без выполнения
u test examples/test_demo.u     # запуск тестов
u fmt examples/hello.u          # форматирование
```

## Архитектура: простое → сложное

| Уровень | Элементы |
|---------|----------|
| 0 | Атомы: `Int`, `Float`, `String`, `Bool`, `none` |
| 1 | Составные: `List[T]`, `struct`, `enum` |
| 2 | Функции: `fn`, замыкания `\|x\| expr` |
| 3 | Управление: `if`, `match`, `for`, `range` |
| 4 | Модули: файл = модуль, `use` |
| 5 | Конкурентность: `spawn`, `Channel` |

## Мутация — `mut`

```
// мутация — mut в определении
fn increment(mut self)
    self.value = self.value + 1
end

// вызов — всегда через .
c.increment()
```

`mut` в определении функции/метода — единственный маркер мутации. При вызове — всегда `.`. Компилятор генерирует `&` или `&mut` автоматически.

## Отличия от Rust

| U | Rust | Причина |
|---|------|---------|
| `end` | `}` | Клавиатура |
| нет `;` | `;` | Упрощение |
| `fn(x) expr` | `\|x\| expr` | Клавиатура |
| `$(name)` | `format!("{name}")` | Упрощение, унификация |
| `mut` в сигнатуре | `&mut` при вызове | Проще — маркер только в определении |
| нет `let` | `let`/`let mut` | Упрощение |
| `spawn f()` | `tokio::spawn(...)` | Упрощение |
| скрипт по умолчанию | `fn main()` | Упрощение |

## Бенчмарки

| Метрика | Результат |
|---------|-----------|
| `u run hello.u` | 0.01 сек |
| `u run todo_cli.u` (Sqlite) | 0.01 сек |
| HTTP-сервер (Router + hyper) | 50,243 req/sec |
| vs Axum | 2.3x быстрее |
| 100K запросов | 0 ошибок |

## Примеры

| Пример | Что демонстрирует |
|--------|-------------------|
| `hello.u` | Строковая интерполяция, print |
| `calc.u` | Функции, рекурсия, циклы |
| `shapes.u` | Struct, enum, match, мутация через `.` |
| `todo_cli.u` | Sqlite, CLI-аргументы, ошибки через `match` |
| `workers.u` | Spawn, каналы, конкурентность |
| `server.u` | HTTP-сервер, keep-alive |
| `objects.u` | Impl, методы |

## Лицензия

MIT
