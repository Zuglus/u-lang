# U Language

Kotlin для Rust. Удобный язык поверх Rust-экосистемы.

- `.u` и `.rs` в одном проекте, один build, бесшовные вызовы
- `u run` — компиляция + кэш, 0.01 сек повторно
- `u build` — компиляция через Rust, нативная скорость
- Вся экосистема crates.io доступна

## Быстрый пример

```
name = "Мир"
print("Привет, $name!")
```

```
fn factorial(n)
    if n < 2
        return 1
    end
    return n * factorial(n - 1)
end

print("10! = $(factorial(10))")
```

```
db = Sqlite.open("app.db")?
db.exec("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)")?
db.exec("INSERT INTO users (name) VALUES ($1)", "Иван")?
users = db.query("SELECT id, name FROM users")?
for row in users
    print("$(row.int("id")). $(row.string("name"))")
end
```

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
| `$name` | `format!("{name}")` | Упрощение |
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
| `todo_cli.u` | Sqlite, CLI-аргументы, `?` ошибки |
| `workers.u` | Spawn, каналы, конкурентность |
| `server.u` | HTTP-сервер, keep-alive |
| `fault_tolerance.u` | Автоматический catch паник в spawn |
| `race_check.u` | Безопасный счётчик через канал |
| `spawn_safety.u` | Запрет `mut` в spawn |
| `sitegen.u` | Статический сайт-генератор |
| `objects.u` | Impl, trait, методы |
| `server_router.u` | Router API, 50K+ req/sec |

## Спецификация

Полная спецификация: [spec/SPEC.md](spec/SPEC.md) (v2.1)

## Лицензия

MIT
