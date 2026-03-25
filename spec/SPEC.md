# U — язык программирования

## Скорость Rust. Простота для ИИ.

Версия 2.1 — март 2026

---

## 1. Что это

U — транспилятор в Rust. Простой код на U превращается в оптимизированный Rust автоматически. ИИ генерирует U-код, транспилятор генерирует Rust, компилятор Rust создаёт нативный бинарник.

Не интерпретатор. Не виртуальная машина. Компиляция в нативный код через Rust.

### 1.1. Доказано

| Факт | Числа |
|------|-------|
| Скорость vs Rust | 93% (0.44 vs 0.41 сек на 1000 файлов) |
| Скорость vs Python | 4.5x быстрее |
| HTTP-сервер | 50,243 req/sec |
| vs Axum (Rust-фреймворк) | 2.3x быстрее |
| Строк кода vs Rust | 22% меньше (137 vs 176) |
| Повторный запуск | 0.01 сек (кэш) |
| Первый запуск (hello) | ~0.5 сек (без тяжёлых зависимостей) |
| Один бинарник | Да |

### 1.2. Для кого

ИИ генерирует код. Человек описывает задачу. Язык проектируется для:

- Минимум токенов → меньше ошибок ИИ
- Однозначный синтаксис → предсказуемая генерация
- Строгие правила → компилятор ловит ошибки → быстрая обратная связь ИИ
- Нативная скорость → результат сразу быстрый

### 1.3. Принципы проектирования

- Сложность от задачи, не от языка
- Каждая фича убирает конкретную проблему
- Проверяется бенчмарками, не теориями

---

## 2. Инструменты

```
u run script.u        # компиляция + запуск + кэш
u run script.u        # повторно — из кэша (0.01 сек)
u build script.u      # бинарник для деплоя
u test script.u       # запуск test fn
u test                # все тесты в проекте
u check script.u      # парсинг, проверка ошибок
u fmt script.u        # форматирование
```

`u run` = `u build` + execute + cache. Один компилятор, одна среда.

### 2.1. Умные зависимости

Транспилятор анализирует AST и подключает только нужное:

| Скрипт использует | Зависимости | Время первой сборки |
|-------------------|------------|-------------------|
| print, if, for, файлы | Ничего | ~0.5 сек |
| parse_json | serde_json | ~0.8 сек |
| Sqlite | rusqlite | ~1.2 сек |
| spawn, сервер | tokio, hyper | ~1.5 сек |

Повторная сборка (код изменён): ~0.5 сек. Без изменений: 0.01 сек (кэш).

---

## 3. Ключевые слова (30)

```
fn, return, if, elif, else, end,
for, in, while, loop, break, continue,
struct, impl, trait, enum, match,
pub, test, unsafe, weak,
use, spawn, and, or, not,
true, false, none, mut
```

Go — 25. U — 30. Python — 35. Rust — 39.

---

## 4. Доступность

### 4.1. Язык — типы, часть синтаксиса

```
Int, Float, String, Bool
List[T], Map[K, V]
Result[T, E], T?
none, true, false
```

### 4.2. Ядро — без `use`

```
print, read_file, write_file, list_dir, create_dir
copy_file, copy_dir, is_dir
sleep, range, len, str, int, float, type_of
parse_json, to_json
```

### 4.3. Стандартная библиотека — `use std.xxx`

```
use std.db: Sqlite
use std.http: Router, Response, serve
use std.channel: Channel
use std.args: Args
```

### 4.4. Внешнее — `.rs` модули

```
use fast_math: fibonacci
```

### 4.5. Именование

Заглавная — тип: `Int`, `Sqlite`, `User`.
Строчная — функция или значение: `print`, `none`.

---

## 5. Мутация — `mut`

### 5.1. Правило

`mut` в определении функции или метода — единственный маркер мутации. При вызове — всегда `.`. Компилятор генерирует `&` или `&mut` автоматически.

### 5.2. Функции

```
// чтение — без mut
fn total(list) -> Int
    return list.sum()
end

// мутация — mut в параметре
fn sort(mut list)
    list.sort_in_place()
end

// вызов — всегда .
print(total(data))
sort(data)
```

### 5.3. Методы

```
impl Counter
    fn get(self) -> Int
        return self.value
    end

    fn increment(mut self)
        self.value = self.value + 1
    end
end

c.get()
c.increment()
```

### 5.4. Генерируемый Rust

| U-код | Rust |
|-------|------|
| `fn show(user)` | `fn show(user: &User)` |
| `fn sort(mut list)` | `fn sort(list: &mut Vec<i64>)` |
| `show(user)` | `show(&user)` |
| `sort(data)` | `sort(&mut data)` |
| `fn get(self)` | `fn get(&self)` |
| `fn increment(mut self)` | `fn increment(&mut self)` |

### 5.5. Субъект действия

Метод (точка) — объект действует на свои данные: `text.trim()`.
Функция — внешнее действие: `print(text)`, `sleep(100)`.

---

## 6. Конкурентность

### 6.1. Spawn

```
spawn handle(conn)

fn worker(ch)
    loop
        task = ch.recv()
        process(task)
    end
end
spawn worker(ch)
```

### 6.2. Каналы

```
use std.channel: Channel

ch = Channel.new()
ch.send("задача")
result = ch.recv()
```

### 6.3. Правила spawn

- Внешние переменные видны на чтение
- Мутация внешних данных в spawn запрещена (компилятор проверяет по `mut`)
- Spawn ловит паники автоматически

```
data = [1, 2, 3]
spawn fn()
    print(data.len())     // ok — чтение
end

spawn fn()
    sort(data)            // ошибка: sort(mut list) — мутация внешнего
end
```

### 6.4. Автопараллельность for (исследуется)

Компилятор анализирует тело `for`: нет `mut` на внешних данных → итерации независимы → может параллелить автоматически. Решение по практике.

### 6.5. Реализация

`spawn` → `tokio::spawn` (~400 байт на задачу). Каналы → `tokio::sync::mpsc`.

---

## 7. Синтаксис

### 7.1. Как в Rust

```
fn, struct, trait, impl, enum, pub, match + =>,
for, while, loop, if, else, elif, //, ?, ->
```

### 7.2. Отличия от Rust

| U | Rust | Причина |
|---|------|---------|
| `end` | `}` | Надёжнее для ИИ-генерации |
| нет `;` | `;` | Меньше токенов |
| `fn(x) expr` | `\|x\| expr` | Клавиатура |
| `$name` | `format!("{name}")` | Меньше токенов |
| `mut` в сигнатуре | `&mut` при вызове | Проще — маркер только в определении |
| нет `let` | `let`/`let mut` | Меньше токенов |
| `and`, `or`, `not` | `&&`, `\|\|`, `!` | Читаемость |
| `[T]` дженерики | `<T>` | Клавиатура |
| `spawn f()` | `tokio::spawn(...)` | Одно слово |
| `return` явный | последнее выражение | Один способ |
| скрипт без main | `fn main()` | Меньше boilerplate |

### 7.3. Строки

```
// обычная
name = "Андрей"
greeting = "Привет, $name!"

// raw — кавычки внутри свободно
html = #"<div class="card">$name</div>"#

// многострочная raw
page = #"
<html>
<body class="main">
    <h1>$name</h1>
</body>
</html>
"#
```

Интерполяция: `$name` — переменная, `$(expr)` — выражение, `$100` — буквально (цифра после $).

### 7.4. Блоки

```
if x > 0
    do_something()
elif x == 0
    do_other()
else
    handle()
end

for item in list
    if item < 0
        continue
    end
    if item == target
        break
    end
    process(item)
end

while x > 0
    x = x - 1
end

loop
    conn = server.accept()
    spawn handle(conn)
end

match shape
    Circle(r) => 3.14 * r * r
    Rect(w, h) => w * h
end
```

### 7.5. Функции

```
fn add(a, b)
    return a + b
end

pub fn process(data: List[Int]) -> Result[Int, Error]
    return data.filter(fn(x) x > 0).sum()
end
```

Приватные — типы выводятся. Публичные — типы обязательны.

### 7.6. Структуры, трейты

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

### 7.7. Тесты

```
fn add(a, b)
    return a + b
end

test fn test_add()
    assert(add(2, 3) == 5)
    assert_eq(add(0, 0), 0)
end
```

### 7.8. Dot-методы

```
// String
s.len()  s.trim()  s.find("x")  s.replace("a", "b")
s.starts_with("#")  s.ends_with(".md")  s.contains("hello")
s.slice(0, 5)  s.split_lines()

// List
list.len()  list.first()  list.last()  list.sum()
list.filter(fn(x) x > 0)  list.map(fn(x) x * 2)
list.join(", ")  list.is_empty()
list[0]  list[i]

// Map
map["key"]  map.len()  map.keys()

// Int
n.abs()  n.to_string()
```

### 7.9. Циклы

| Конструкция | Для чего |
|-------------|---------|
| `for item in list` | Итерация по коллекции |
| `while condition` | Цикл с условием |
| `loop` | Бесконечный цикл (серверы, воркеры) |
| `break` | Выход из цикла |
| `continue` | Пропуск итерации |

---

## 8. Интеграция с Rust

`.rs` файлы — полноценные модули в проекте:

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

U для 90% кода. Rust для 10% — парсеры, драйверы, SIMD, криптография.

---

## 9. Стратегия памяти

### 9.1. Принцип

Программист не думает о памяти. Компилятор выбирает оптимальный механизм. `mut` даёт информацию.

### 9.2. Что работает сейчас

- Stack-first — структуры на стеке
- Borrowing — нет `mut` → `&`, есть `mut` → `&mut`
- Move — последнее использование → move

### 9.3. Исследовано, не реализовано

| Механизм | Источник |
|----------|---------|
| Escape-анализ | Go, Java JIT |
| Reuse / FBIP | Perceus (Koka) |
| Compile-time RC elision | Lobster |
| Arena | Verona |
| COW | Swift |
| Auto-weak | Новое |

Реализация — когда появится конкретная задача.

---

## 10. Решаемые проблемы

### 10.1. Проблемы Rust

| Проблема | Решение U |
|----------|-----------|
| Сложный синтаксис (lifetimes, `&mut`) | `mut` в сигнатуре, компилятор генерирует |
| Медленная компиляция | Кэш, умные зависимости |
| Крутая кривая обучения | Скрипт без main/use/типов |
| Boilerplate | Нет `let`, `;`, `fn main()` |

### 10.2. Проблемы Python

| Проблема | Решение U |
|----------|-----------|
| Медленный (4.5x) | Нативная компиляция через Rust |
| GIL — нет параллельности | spawn, реальные потоки |
| None → крэш в рантайме | Option[T], ? при компиляции |
| pip hell, деплой | Один бинарник |

### 10.3. Проблемы Go

| Проблема | Решение U |
|----------|-----------|
| GC паузы | Нет глобального GC |
| if err != nil | ? оператор |
| Нет enum/ADT | enum + match => |
| Нет доступа к Rust-экосистеме | .rs файлы, crates.io |

---

## 11. Чего не будет

- Интерпретатор. Только компиляция.
- Наследование. Трейты и композиция.
- Null/nil. `Option[T]` с `?`.
- Исключения. `Result[T, E]` с `?`.
- Множественный синтаксис. Один способ.
- Глобальный GC.
- Виртуальная машина / байткод.
- Extension functions. Методы через `impl`.
- Перегрузка функций. Уникальные имена.
- Implicit conversions. Явные преобразования.
- Макросы. Явный код.
- Глобальное mutable состояние.

---

## 12. Модификаторы

| Модификатор | Значение |
|-------------|----------|
| `pub` | Видимость за пределами модуля |
| `test` | Тестовая функция |
| `mut` | Мутация параметра |
| `unsafe` | Обход проверок |
| `weak` | Слабая ссылка (при необходимости) |

---

## 13. Архитектура

```
.u файл → парсер (pest) → AST → генератор → .rs файл → rustc → бинарник
                                     ↓
                              анализ AST → минимальный Cargo.toml
                                     ↓
                              кэш (~/.u-cache/) → мгновенный повторный запуск
```

Репозиторий: github.com/Zuglus/u-lang

---

## 14. Открытые вопросы

1. Автопараллельность `for` — проверить практикой.
2. Генератор — none, .first(), .filter(), JSON field access.
3. Стандартная библиотека — HTTP-клиент, расширение.
4. Ошибки — маппинг rustc → .u строки.
5. LSP — автодополнение.
6. Стратегия памяти — реализовать когда появится задача.
7. `weak` — нужен ли как ключевое слово или auto-weak достаточно.

---

*Проверяется бенчмарками, не теориями.*
