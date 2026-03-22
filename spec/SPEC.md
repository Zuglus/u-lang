# Спецификация единого языка программирования

## Рабочее название: U (Universal)

Черновик v1.1 — март 2026

---

## 1. Позиционирование

U — Kotlin для Rust. Удобный язык поверх Rust-экосистемы. Не замена Rust, а партнёр.

- U для 90% кода: серверы, CLI, скрипты, обработка данных
- Rust для 10%: парсеры, движки, драйверы, SIMD, криптография
- `.u` и `.rs` в одном проекте, один build, бесшовные вызовы
- Вся экосистема crates.io доступна

### 1.1. Принцип

Синтаксис Rust — за основу. Отличия — только где обосновано клавиатурой или упрощением. Один способ написать каждую конструкцию. Простота = скорость (меньше абстракций → быстрее код).

---

## 2. Два инструмента

| Команда | Что делает | Скорость старта | Скорость работы |
|---------|-----------|-----------------|-----------------|
| `u run` | Интерпретатор AST | 0.01 сек | Скриптовая |
| `u build` | Компиляция через Rust | 2-7 сек | Нативная (35K req/sec) |

`u run` — для разработки и скриптов. `u build` — для продакшна. Один и тот же код.

---

## 3. Четыре уровня доступности

### 3.1. Язык — типы и значения, часть синтаксиса

Всегда доступны, не импортируются:

```
Int, Float, String, Bool          // базовые типы
List[T], Map[K, V]                // коллекции
Result[T, E], T?                  // ошибки и nullable
none, true, false                 // значения
```

### 3.2. Ядро — функции без которых скрипт не напишешь

Всегда доступны, без `use`:

```
print(value)                      // вывод
read_file(path) -> Result         // чтение файла
write_file(path, content)         // запись файла
list_dir(path) -> List[String]    // список файлов
create_dir(path)                  // создание папки
sleep(ms)                         // пауза
range(from, to) -> List[Int]      // диапазон
len(collection) -> Int            // длина
str(value) -> String              // конвертация в строку
int(value) -> Int                 // конвертация в число
type_of(value) -> String          // тип значения
```

### 3.3. Стандартная библиотека — специализированное, поставляется с `u`

Через `use std.xxx`:

```
use std.db: Sqlite                // база данных
use std.http: Router, Response, serve  // HTTP-сервер
use std.json: parse_json, to_json     // JSON
use std.channel: Channel          // каналы для конкурентности
use std.args: Args                // аргументы командной строки
use std.regex: Regex              // регулярные выражения
use std.str: starts_with, ends_with, contains, replace,
             find, find_from, slice_from, slice_range,
             split_lines, str_len, trim, path_stem
```

### 3.4. Внешнее — `.rs` модули и crates.io

Через `use`:

```
use fast_math: fibonacci          // .rs файл рядом
use my_parser: parse              // .rs модуль
```

### 3.5. Правило именования

Заглавная — тип или конструктор: `Int`, `String`, `Sqlite`, `Channel`, `User`.
Строчная — функция или значение: `print`, `read_file`, `none`, `true`.

---

## 4. Мутация — оператор `::`

### 4.1. Принцип

`.` — чтение или команда внешней системе. `::` — изменение своих байтов в RAM.

### 4.2. Чтение и команды через `.`

```
users.len()                       // чтение
db.exec("insert...")              // команда БД
conn.write(response)              // команда ОС
ch.send("task")                   // команда каналу
print("hello")                    // команда ОС
```

### 4.3. Мутация через `::`

```
list::push(4)                     // меняет list в RAM
list::sort()                      // меняет list в RAM
user::name = "Иван"               // меняет поле в RAM
```

### 4.4. Функции и методы

```
fn sort(::list)                   // :: в определении
    list::sort_in_place()
end

sort(::users)                     // :: при вызове

impl List
    fn len(self) -> Int           // чтение
        return self.items.count()
    end

    fn sort(::self)               // мутация
        self.items::sort_raw()
    end
end
```

### 4.5. Spawn — запрет :: на внешних данных

```
data = [1, 2, 3]
spawn fn() print(data.len())     // ok: чтение
spawn fn() data::push(4)         // ошибка компиляции
```

---

## 5. Конкурентность

### 5.1. Spawn и каналы

```
use std.channel: Channel

spawn handle(conn)                        // запуск горутины
spawn fn() send_email(user.email, msg)    // лямбда

ch = Channel.new()
spawn worker(ch)
ch.send("задача")
result = ch.recv()
```

### 5.2. Гарантии

- `::` запрещён в spawn на внешних данных — компилятор проверяет
- Spawn ловит паники автоматически — горутина падает, остальные продолжают
- Разделяемое состояние не повреждено — горутина не могла его мутировать

### 5.3. Реализация

`u run` — `std::thread` + `std::sync::mpsc` (для скриптов достаточно).
`u build` — `tokio::spawn` + `tokio::sync::mpsc` (~400 байт на задачу).

---

## 6. Синтаксис

### 6.1. Как в Rust

```
fn, struct, trait, impl, enum, pub, match + =>,
for, while, loop, if, else, elif, //, ?, ->
```

### 6.2. Отличия от Rust

| U | Rust | Причина |
|---|------|---------|
| `end` | `}` | Клавиатура |
| нет `;` | `;` | Упрощение |
| `fn(x) expr` | `\|x\| expr` | Клавиатура |
| `$name` | `format!("{name}")` | Упрощение |
| `::` мутация | `&mut` | Уникальная фича |
| нет `let` | `let`/`let mut` | Упрощение |
| `and`, `or`, `not` | `&&`, `\|\|`, `!` | Клавиатура |
| `!` unwrap | `.unwrap()` | Упрощение |
| `[T]` дженерики | `<T>` | Клавиатура |
| `.` для путей | `::` для путей | `::` занят мутацией |
| `spawn f()` | `tokio::spawn(...)` | Упрощение |
| `return` явный | последнее выражение | Один способ |
| скрипт по умолчанию | `fn main()` | Упрощение |

### 6.3. Блоки

```
if x > 0
    do_something()
elif x == 0
    do_other()
else
    handle()
end
```

### 6.4. Функции

```
fn add(a, b)
    return a + b
end

pub fn process(data: List[Int]) -> Result[Int, Error]
    return data.filter(fn(x) x > 0).sum()
end

fn log(message)
    file.write(message)
end
```

### 6.5. Лямбда — одно выражение

```
data.filter(fn(x) x > 0).map(fn(x) x * 2)
```

### 6.6. Enum и match

```
enum Shape
    Circle(radius: Float)
    Rect(width: Float, height: Float)
end

match shape
    Circle(r) => pi * r * r
    Rect(w, h) => w * h
end
```

### 6.7. Структуры, трейты, impl

```
struct User
    name: String
    age: Int
end

trait Display
    fn to_string(self) -> String
end

impl Display for User
    fn to_string(self) -> String
        return "$(self.name), $(self.age)"
    end
end
```

### 6.8. Строковая интерполяция

```
greeting = "Привет, $name! Возраст: $(user.age + 1)"
```

### 6.9. Логические операторы

```
if x > 0 and y < 10
    do_something()
end

if not found or expired
    refresh()
end
```

---

## 7. Интеграция с Rust

`.rs` файлы в проекте — полноценные модули:

```rust
// fast_math.rs
pub fn fibonacci(n: u64) -> u64 {
    if n <= 1 { return n; }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 2..=n { let tmp = a + b; a = b; b = tmp; }
    b
}
```

```
// main.u
use fast_math: fibonacci
print("fib(50) = $(fibonacci(50))")
```

`u build` — транспилирует `.u`, компилирует `.rs`, линкует вместе. Один бинарник.

---

## 8. Модификаторы

| Модификатор | Значение |
|-------------|----------|
| `unsafe` | Обход проверок, только на строку |
| `weak` | Слабая ссылка |
| `test` | Тестовая функция |
| `pub` | Видимость за пределами модуля |

---

## 9. Система сборки

```
u run script.u              // интерпретатор, 0.01 сек
u build script.u            // компиляция, нативная скорость
u test                      // запуск test fn
u check script.u            // парсинг, проверка ошибок
u fix script.u              // автодобавление use
```

---

## 10. Происхождение

| Концепция | Источник |
|-----------|----------|
| Синтаксис, типы, `?`, `=>`, `->`, `enum`, `pub` | Rust |
| `struct`, `trait`, `impl`, `match`, `for/while/loop` | Rust |
| `::` мутация, spawn safety | Новое |
| RC+GC автоматический | Python, Swift |
| `end` блоки | Ruby, Lua |
| `$` интерполяция | Kotlin, Shell |
| `and/or/not` | Python, SQL |
| Горутины + каналы | Go (модель), tokio (реализация) |
| Скрипт по умолчанию | Python, Kotlin |
| U + Rust в одном проекте | Kotlin + Java |
| Четыре уровня доступности | Rust (prelude), Python (builtins) |

---

## 11. Чего не будет

- Наследование. Трейты и композиция.
- Null/nil. `Option[T]` с `T?`.
- Исключения. `Result[T, E]` с `?`.
- Множественный синтаксис. Один способ.
- Расширяемые аннотации. 4 модификатора.
- Мутабельные переменные. Мутация через `::`.
- Многострочные лямбды.
- `memory(auto/own/no)`. Просто `.u` и `.rs`.

---

## 12. Бенчмарки

| Метрика | Результат |
|---------|-----------|
| `u run hello.u` | 0.01 сек |
| `u run calc.u` (рекурсия, циклы) | 0.01 сек |
| `u run todo_cli.u` (Sqlite) | 0.01 сек |
| HTTP-сервер (Router + hyper) | 35,342 req/sec |
| HTTP-сервер vs Axum | +64% быстрее |
| 100K запросов | 0 ошибок |
| Латентность p99 | 5 мс |

---

## 13. Открытые вопросы

1. Границы U↔Rust — автоматические преобразования типов на стыке.
2. Стандартная библиотека — точный состав ядра и std.
3. `!` force-unwrap — предупреждение компилятора?
4. Persistent data structures для конкурентных данных в RAM.
5. Mesh-горутины — прямое общение без центрального канала.
6. Маппинг ошибок rustc → .u файл (базовый работает).
7. `u test` — запуск test fn.
8. Форматировщик — единый стиль.
9. `u fix` — автодобавление use.
10. LSP — автодополнение, навигация.

---

## 14. Путь реализации

### Фаза 1 — текущий этап

Транспилятор + интерпретатор. 12 работающих примеров. Репозиторий: github.com/Zuglus/u-lang

### Фаза 2 — реальные проекты

CLI, серверы, скрипты на U. Измерить выигрыш vs чистый Rust.

### Фаза 3 — собственный компилятор

Если фаза 2 подтвердила. Байткод + Cranelift для быстрой сборки.

---

*Проверяется через прототипирование.*
