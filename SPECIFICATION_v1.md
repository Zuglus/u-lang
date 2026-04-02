# U-lang Language Specification v1.0

**Status Legend:**
- ✅ **Implemented** — полностью работает
- 🧪 **Partial** — работает частично или с ограничениями  
- 🚧 **In Progress** — в разработке
- 📐 **Designed** — спроектировано, не реализовано

---

## 1. Overview

U — системный язык программирования с:
- **Статической типизацией** — типы проверяются на этапе компиляции
- **Value semantics** — данные копируются, а не разделяются по ссылке
- **Channel-based concurrency** — нет shared memory, только message passing
- **Pattern matching** — мощный match для enum и структур
- **Модульной системой** — простой импорт/экспорт

### 1.1 Design Philosophy

1. **Predictability** — код делает то, что видно (нет скрытых аллокаций)
2. **Safety** — нет null pointer exceptions, нет data races
3. **Simplicity** — минимальный синтаксис, явное поведение
4. **Pragmatism** — работающий код важнее теоретической чистоты

---

## 2. Lexical Structure

### 2.1 Source Encoding
- UTF-8 только
- LF для новых строк (CRLF конвертируется)

### 2.2 Keywords (25 зарезервированных слов)

```
fn struct enum trait impl
if else elif end match
for in while loop
return break continue spawn
use pub test memory
mut not and or
```

**Status:** ✅ Все зарезервированы и работают как ключевые слова

### 2.3 Identifiers

```rust
identifier = [a-zA-Z_][a-zA-Z0-9_]*
```

**Ограничения:**
- Не может начинаться с цифры
- Не может быть ключевым словом
- Регистрозависимые (`Foo` ≠ `foo`)

**Status:** ✅ Работает

### 2.4 Literals

| Тип | Синтаксис | Примеры | Status |
|-----|-----------|---------|--------|
| Integer | decimal | `42`, `-10` | ✅ |
| Float | decimal | `3.14`, `-2.5` | ✅ |
| String | double quote | `"hello"` | ✅ |
| String interpolation | `"value: $x"` | `"count: $n"` | ✅ |
| Raw string | `#"raw"#` | `#"no \escape"#` | ✅ |
| Bool | | `true`, `false` | ✅ |
| None | | `none` | ✅ |
| List | | `[1, 2, 3]` | ✅ |

**String interpolation детали:**
- `$var` — вставка переменной ✅
- `$(expr)` — вставка выражения 🚧 (парсится, но не реализовано)

**Status:** ✅ Базовая интерполяция работает

### 2.5 Comments

```u
// Line comment

/*
Block comment 🚧
*/

/// Doc comment 🚧
```

**Status:** ✅ Line comments работают, block — не реализованы

### 2.6 Operators

| Приоритет | Операторы | Ассоциативность | Status |
|-----------|-----------|-----------------|--------|
| 1 (высший) | `()` `[]` `.` | left | ✅ |
| 2 | unary `-` `not` | right | ✅ |
| 3 | `*` `/` `%` | left | ✅ |
| 4 | `+` `-` | left | ✅ |
| 5 | `<` `>` `<=` `>=` | left | ✅ |
| 6 | `==` `!=` | left | ✅ |
| 7 | `and` | left | ✅ |
| 8 | `or` | left | ✅ |
| 9 (низший) | `=` | right | ✅ |

**Примечание:** `and`/`or` — keyword-операторы, не символы

---

## 3. Type System

### 3.1 Type Hierarchy

```
Type
├── Primitive
│   ├── Int (64-bit signed)
│   ├── Float (64-bit IEEE 754)
│   ├── Bool
│   ├── String (UTF-8, immutable)
│   └── None (unit type)
├── Compound
│   ├── List[T] 🧪
│   └── Tuple (T1, T2, ...) 📐
└── User-Defined
    ├── Struct
    ├── Enum (tagged union)
    └── Trait (interface)
```

### 3.2 Primitive Types

#### Int
- Размер: 64-bit signed
- Диапазон: -9,223,372,036,854,775,808 .. 9,223,372,036,854,775,807
- Переполнение: wrapping (как в Rust)

**Status:** ✅

#### Float  
- Размер: 64-bit IEEE 754
- Специальные значения: NaN, Inf (но не рекомендуются)

**Status:** ✅

#### Bool
- Значения: `true`, `false`
- Размер: 1 byte

**Status:** ✅

#### String
- Кодировка: UTF-8
- Immutable (неизменяемые)
- Внутреннее представление: `Vec<u8>` в runtime

**Status:** ✅

#### None
- Единственное значение: `none`
- Аналог `void`/`()` в других языках
- Используется для функций без возвращаемого значения

**Status:** ✅

### 3.3 List[T] 🧪

**Синтаксис:**
```u
// Создание
lst = [1, 2, 3]           // Тип выводится как List[Int]
empty = []                // 🚧 Требует аннотации типа

// Доступ
first = lst[0]            // ✅ Индексация
len = lst.len()           // ✅ Длина
first = lst.first()       // ✅ Первый элемент
last = lst.last()         // ✅ Последний элемент

// Итерация
for x in lst              // ✅
    print("$x")
end

// Методы (не реализованы) 🚧
// lst.push(val)
// lst.pop()
// lst.filter(fn)
// lst.map(fn)
```

**Ограничения:**
- `List[T]` в полях структур — только как `List` (без параметра) 🚧
- Нет метода `get(i)` — используйте индексацию `lst[i]`

### 3.4 Struct

**Определение:**
```u
struct Point
    x: Int
    y: Int
end

// Публичная структура
pub struct Config
    name: String
    port: Int
end
```

**Конструкция:**
```u
p = Point(x: 10, y: 20)
```

**Доступ к полям:**
```u
x_coord = p.x
p.x = 30           // ✅ Мутация полей разрешена
```

**Ограничения:**
- Типы полей — только простые идентификаторы (`List`, не `List[Int]`) 🧪
- Нет tuple-struct (`struct Point(Int, Int)`) 🚧
- Нет unit-struct (`struct Marker`) 🚧

**Status:** 🧪 Структуры работают, но с ограничением на типы полей

### 3.5 Enum (Tagged Union)

**Определение:**
```u
enum Shape
    Circle(radius: Int)
    Rectangle(width: Int, height: Int)
    Triangle(a: Int, b: Int, c: Int)
end
```

**Конструкция:**
```u
c = Circle(radius: 10)
r = Rectangle(width: 20, height: 30)
```

**Pattern matching:**
```u
match shape
    Circle(r) => print("circle $r")
    Rectangle(w, h) => print("rect $w x $h")
    _ => print("other")           // wildcard ✅
end
```

**Ограничения:**
- Варианты ОБЯЗАТЕЛЬНО должны иметь поля (нет unit variants) 🧪
- Нет generic enum (`enum Option[T]`) 🚧
- Нет рекурсивных enum 🚧 (нужно для AST!)

**Status:** 🧪 Enum работают, но с ограничениями (всегда с полями)

### 3.6 Trait (Interface)

**Определение:**
```u
trait Drawable
    fn draw()
end
```

**Implementation:**
```u
impl Drawable for Circle
    fn draw()
        print("drawing circle")
    end
end
```

**Использование:**
```u
fn render(d: Drawable)      // ❌ Не работает — нет trait objects
    d.draw()
end

// Работает через generic bounds (не реализовано) 🚧
// fn render[T: Drawable](d: T)
```

**Ограничения:**
- Нет trait bounds (`[T: Drawable]`) 🚧
- Нет associated types 🚧
- Нет default implementations 🚧
- Нет supertraits (`trait Drawable: Clone`) 🚧

**Status:** ✅ Базовые trait/impl работают, ограничения см. выше

---

## 4. Variables

### 4.1 Declaration

```u
// Immutable (по умолчанию)
x = 10
// x = 20  // ❌ Ошибка: нельзя переопределить

// Mutable через Wrapper pattern ✅
struct Counter
    value: Int
end

c = Counter(value: 0)
c.value = 10              // ✅ OK — мутируем поле структуры
```

**Ограничения:**
- Нет `mut` для локальных переменных (`mut x = 10`) 🚧
- Нет `const` или `static` 🚧

### 4.2 Scope

```u
x = 10

fn foo()
    x = 20        // Это НОВАЯ переменная, не мутация!
    print("$x")   // 20
end

foo()
print("$x")       // 10 — внешняя не изменилась
```

**Правило:** Вложенные scope создают новые переменные, не видят внешние.

**Status:** ✅ Работает как описано

---

## 5. Functions

### 5.1 Definition

```u
// Без возвращаемого значения (возвращает none)
fn greet(name: String)
    print("Hello, $name")
end

// С возвращаемым значением
fn add(a: Int, b: Int) -> Int
    return a + b
end

// Публичная функция
pub fn public_api()
    // ...
end

// Тест
fn test_addition()
    result = add(2, 3)
    assert(result == 5)
end
```

**Status:** ✅ Всё работает

### 5.2 Parameters

```u
// Immutable параметр (по умолчанию)
fn by_value(x: Int)
    // x нельзя изменить
end

// Mutable параметр
fn by_ref(mut x: Int)
    x = x + 1     // ✅ OK — создаётся локальная копия
end
```

**Ограничения:**
- Нет default parameters (`fn f(x: Int = 0)`) 🚧
- Нет variadic parameters (`fn f(args: Int...)`) 🚧
- Нет function types в параметрах (`fn(f: fn(Int) -> Int)`) 🚧

### 5.3 Lambda (Anonymous Functions)

```u
// Lambda синтаксис
add = fn(a, b) a + b
result = add(1, 2)        // 3

// Высший порядок
fn apply(f, x)
    return f(x)
end

double = fn(x) x * 2
result = apply(double, 5) // 10
```

**Ограничения:**
- Захват переменных (closures) — не тестировано полностью 🧪
- Нет типизированных lambda (`fn(x: Int) -> Int x + 1`) 🚧
- Нет shorthand синтаксиса (`|x| x + 1`) 🚧

**Status:** ✅ Базовые lambda работают, closures — под вопросом

### 5.4 Method Definitions

```u
struct Rectangle
    width: Int
    height: Int
end

impl Rectangle
    fn area(self: Rectangle) -> Int
        return self.width * self.height
    end
    
    fn scale(self: Rectangle, factor: Int) -> Rectangle
        return Rectangle(
            width: self.width * factor,
            height: self.height * factor
        )
    end
end
```

**Важно:** `self` — обычный параметр, не keyword. Должен быть явно типизирован.

**Status:** ✅ Работает

---

## 6. Control Flow

### 6.1 If-Else

```u
if x > 0
    print("positive")
elif x < 0
    print("negative")
else
    print("zero")
end
```

**Ограничения:**
- Нет `unless` 🚧
- Нет ternary operator (`cond ? a : b`) 🚧

**Status:** ✅ Работает

### 6.2 Loops

```u
// For loop
for i in items
    print("$i")
end

// While loop
while x > 0
    x = x - 1
end

// Infinite loop
loop
    if done
        break
    end
end
```

**Control statements:**
- `break` — выход из цикла ✅
- `continue` — следующая итерация ✅

**Ограничения:**
- Нет `break value` (возврат значения из loop) 🚧
- Нет range iteration (`for i in 0..10`) 🚧

**Status:** ✅ Базовые циклы работают

### 6.3 Pattern Matching

```u
match value
    // Literal pattern
    1 => print("one")
    
    // Variable pattern
    x => print("got $x")
    
    // Constructor pattern
    Circle(r) => print("radius $r")
    
    // Wildcard
    _ => print("other")
end
```

**Поддерживаемые patterns:**
- ✅ Literal: `1`, `"str"`, `true`
- ✅ Variable: `x`
- ✅ Constructor: `Variant(field)`
- ✅ Wildcard: `_`

**Не поддерживаются:**
- Guard patterns (`x if x > 0 =>`) 🚧
- Multiple patterns (`1 | 2 =>`) 🚧
- Range patterns (`1..5 =>`) 🚧
- List patterns (`[a, b] =>`) 🚧
- Binding (`x @ Pattern =>`) 🚧

**Status:** ✅ Базовые patterns работают

---

## 7. Modules

### 7.1 Structure

```
project/
├── main.u
└── modules/
    ├── math.u
    └── utils/
        └── strings.u
```

### 7.2 Import

```u
// Импорт конкретных элементов
use math: add, subtract, multiply

// Nested modules
use utils.strings: shout, whisper

// Не поддерживается:
// use math: *           🚧 — нет glob import
// use math: add as sum  🚧 — нет aliasing
```

### 7.3 Visibility

```u
// math.u

// Публичная — видна извне
pub fn add(a, b)
    return a + b
end

// Приватная — только внутри модуля
fn helper(x)
    return x * 2
end
```

**Ограничения:**
- Нет `pub(crate)`, `pub(super)` 🚧
- Нет re-export (`pub use other: item`) 🚧

**Status:** ✅ Базовая система модулей работает

---

## 8. Concurrency

### 8.1 Thread Spawning

```u
// Запуск функции в новом потоке
spawn worker()
spawn worker_with_arg(data)
```

### 8.2 Channels

```u
// Создание
ch = Channel.new()

// Отправка
ch.send("hello")

// Получение (блокирующее)
msg = ch.recv()
```

**Модель:** CSP (Communicating Sequential Processes)
- Нет shared memory
- Нет mutex/locks
- Только message passing через channels

**Status:** ✅ Работает

---

## 9. Memory Model

### 9.1 Semantics

**Copy-by-value:** Все присваивания создают копии.

```u
a = [1, 2, 3]
b = a           // b — КОПИЯ a
b.push(4)       // ❌ Ошибка — нет push
// Но если бы был: a всё ещё [1, 2, 3]
```

**Преимущества:**
- Нет data races (данные не разделяются)
- Предсказуемое поведение
- Простая семантика

**Недостатки:**
- Дорого для больших структур
- Нет эффективных циклических структур

### 9.2 Explicit Memory Tracking

```u
memory("before_allocation")
// ... allocate ...
memory("after_allocation")
```

**Status:** ✅ Работает (выводит в stderr)

---

## 10. Standard Library

### 10.1 std.io

| Function | Signature | Status |
|----------|-----------|--------|
| `print(s)` | `fn(String)` | ✅ |
| `read_file(path)` | `fn(String) -> String` | ✅ |
| `write_file(path, content)` | `fn(String, String)` | ✅ |
| `create_dir(path)` | `fn(String)` | ✅ |
| `list_dir(path)` | `fn(String) -> List[String]` | ✅ |
| `copy_file(from, to)` | `fn(String, String)` | ✅ |

**Примечание:** Это свободные функции, не методы.

### 10.2 std.string

| Method | Signature | Status | Notes |
|--------|-----------|--------|-------|
| `s.len()` | `fn() -> Int` | ✅ | |
| `s.find(sub)` | `fn(String) -> Int` | ✅ | |
| `s.find_from(sub, from)` | `fn(String, Int) -> Int` | ✅ | |
| `s.slice(start, end)` | `fn(Int, Int) -> String` | ✅ | |
| `s.slice_from(start)` | `fn(Int) -> String` | ✅ | |
| `s.split(delim)` | `fn(String) -> List[String]` | ✅ | |
| `s.split_lines()` | `fn() -> List[String]` | ✅ | |
| `s.first()` | `fn() -> String` | ✅ | Первый char или пустая строка |
| `s.last()` | `fn() -> String` | ✅ | Последний char или пустая строка |
| `s.trim()` | `fn() -> String` | 🧪 | В runtime ✅, generator ❌ |
| `s.contains(sub)` | `fn(String) -> Bool` | 🧪 | В runtime ✅, generator ❌ |
| `s.starts_with(prefix)` | `fn(String) -> Bool` | 🧪 | В runtime ✅, generator ❌ |
| `s.ends_with(suffix)` | `fn(String) -> Bool` | 🧪 | В runtime ✅, generator ❌ |
| `s.replace(old, new)` | `fn(String, String) -> String` | 🧪 | В runtime ✅, generator ❌ |
| `s.char_at(i)` | `fn(Int) -> String` | 🚧 | Используйте `slice(i, i+1)` |
| `s.substring(start, len)` | `fn(Int, Int) -> String` | 🚧 | Используйте `slice(start, start+len)` |
| `s.to_upper()` | `fn() -> String` | 🚧 | |
| `s.to_lower()` | `fn() -> String` | 🚧 | |
| `s.slice_from(start)` | `fn(Int) -> String` | ✅ |
| `s.split(delim)` | `fn(String) -> List[String]` | ✅ |
| `s.split_lines()` | `fn() -> List[String]` | ✅ |
| `s.first()` | `fn() -> String` 🧪 | ✅ |
| `s.last()` | `fn() -> String` 🧪 | ✅ |
| `s.trim()` | `fn() -> String` | 🧪 (в runtime, нет в generator) |
| `s.contains(sub)` | `fn(String) -> Bool` | 🧪 (в runtime, нет в generator) |
| `s.starts_with(prefix)` | `fn(String) -> Bool` | 🧪 (в runtime, нет в generator) |
| `s.ends_with(suffix)` | `fn(String) -> Bool` | 🧪 (в runtime, нет в generator) |
| `s.replace(old, new)` | `fn(String, String) -> String` | 🧪 (в runtime, нет в generator) |
| `s.char_at(i)` | `fn(Int) -> String` | 🚧 Используйте `slice(i, i+1)` |
| `s.to_upper()` | `fn() -> String` | 🚧 |
| `s.to_lower()` | `fn() -> String` | 🚧 |

### 10.3 std.list

| Method | Signature | Status | Notes |
|--------|-----------|--------|-------|
| `lst.len()` | `fn() -> Int` | ✅ | |
| `lst.first()` | `fn() -> T` | ✅ | |
| `lst.last()` | `fn() -> T` | ✅ | |
| `lst[i]` | Index access | ✅ | |
| `for x in lst` | Iteration | ✅ | |
| `lst.filter(pred)` | `fn(fn(T) -> Bool) -> List[T]` | 🚧 | |
| `lst.map(f)` | `fn(fn(T) -> U) -> List[U]` | 🚧 | |
| `lst.push(val)` | `fn(T)` | 🚧 | |
| `lst.pop()` | `fn() -> T` | 🚧 | |
| `lst.get(i)` | `fn(Int) -> T` | 🚧 | Используйте `lst[i]` |

### 10.4 std.channel

| Method | Signature | Status |
|--------|-----------|--------|
| `Channel.new()` | `fn() -> Channel[T]` | ✅ |
| `ch.send(val)` | `fn(T)` | ✅ |
| `ch.recv()` | `fn() -> T` | ✅ |
| `ch.try_send(val)` | `fn(T) -> Bool` | 🚧 |
| `ch.try_recv()` | `fn() -> Option[T]` | 🚧 |
| `ch.close()` | `fn()` | 🚧 |

---

## 11. FFI (Foreign Function Interface) 📐

**Не реализовано.** Планируется:

```u
// Объявление C-функции
extern fn printf(fmt: String, ...)

// Использование
unsafe
    printf("Hello from C\n")
end
```

---

## 12. Error Handling 🚧

**Текущий подход:** Enum-based

```u
enum Result
    Ok(value: Int)
    Err(message: String)
end

fn divide(a: Int, b: Int) -> Result
    if b == 0
        return Err(message: "division by zero")
    end
    return Ok(value: a / b)
end

// Использование
match divide(10, 2)
    Ok(v) => print("result: $v")
    Err(msg) => print("error: $msg")
end
```

**Планируется:** Generic Result/Option

```u
// 🚧 Не реализовано
enum Result[T, E]
    Ok(v: T)
    Err(e: E)
end

// С оператором ?
result = may_fail()?
```

---

## 13. Generics 🚧

**Не реализовано.** Планируется:

```u
// Generic struct
struct Box[T]
    value: T
end

// Generic enum
enum Option[T]
    Some(v: T)
    None
end

// Generic function
fn identity[T](x: T) -> T
    return x
end

// Generic with bounds
fn print[T: Display](x: T)
    // ...
end
```

---

## 14. Borrow Checker 🚧

**Не реализовано.** Планируется:

```u
// Immutable borrow
fn process(s: &String)
    print(s)     // ✅ Читаем
    // s.push('!') // ❌ Нельзя менять
end

// Mutable borrow
fn append(s: &mut String)
    s.push('!')  // ✅ Меняем
end
```

---

## 15. Known Limitations

Это известные ограничения текущей реализации. Не баги, а конструктивные ограничения языка на данном этапе.

### 15.1 Grammar Limitations

| Limitation | Example | Workaround |
|------------|---------|------------|
| Типы полей — только идентификаторы | `items: List[Int]` ❌ | `items: List` ✅ |
| Enum варианты всегда с полями | `Pending` ❌ | `Pending(code: Int)` ✅ |
| `end` — зарезервировано | `end = 10` ❌ | `end_ = 10` ✅ |
| Функциональные типы в параметрах | `fn(f: fn() -> Int)` ❌ | Используйте lambda |
| Многострочные конструкторы | `Point(\n  x: 10\n)` ❌ | `Point(x: 10)` ✅ |

### 15.2 Type System Limitations

| Limitation | Status | Workaround |
|------------|--------|------------|
| Generics | 🚧 | Concrete types only |
| Recursive enum | 🚧 | Flatten structures |
| Trait bounds | 🚧 | Manual dispatch |
| Borrow checker | 📐 | Copy-by-value semantics |

### 15.3 Standard Library Gaps

| Missing | Workaround |
|---------|------------|
| `char_at(i)` | `slice(i, i+1)` |
| `substring(start, len)` | `slice(start, start+len)` |
| `lst.push()`, `lst.pop()` | Reconstruct list |
| `lst.filter()`, `lst.map()` | Manual iteration |

### 15.4 String Interpolation

| Feature | Status | Example |
|---------|--------|---------|
| Variable | ✅ | `"$name"` |
| Expression | 🚧 | `"$(x + y)"` — не реализовано |

---

## Appendix A: Complete Grammar (EBNF)

```ebnf
program         = { statement }

statement       = fn_def | struct_def | enum_def | trait_def | impl_def
                | if_stmt | for_stmt | while_stmt | loop_stmt | match_stmt
                | return_stmt | break_stmt | continue_stmt
                | assignment | expr_stmt | use_stmt

fn_def          = [ "pub" ] [ "test" ] "fn" identifier 
                  "(" [ params ] ")" [ "->" type ] block "end"

struct_def      = [ "pub" ] "struct" identifier { field } "end"
enum_def        = [ "pub" ] "enum" identifier { variant } "end"
trait_def       = "trait" identifier { trait_method } "end"
impl_def        = "impl" [ identifier "for" ] identifier { fn_def } "end"

field           = identifier ":" type
variant         = identifier "(" field { "," field } ")"
trait_method    = "fn" identifier "(" [ params ] ")" [ "->" type ]

params          = param { "," param }
param           = [ "mut" ] identifier [ ":" type ]

block           = { statement }

if_stmt         = "if" expr block { "elif" expr block } [ "else" block ] "end"
for_stmt        = "for" identifier "in" expr block "end"
while_stmt      = "while" expr block "end"
loop_stmt       = "loop" block "end"
match_stmt      = "match" expr { arm } "end"
arm             = pattern "=>" (expr | block)
pattern         = literal | identifier | constructor | "_"
constructor     = identifier "(" [ identifier { "," identifier } ] ")"

return_stmt     = "return" [ expr ]
break_stmt      = "break"
continue_stmt   = "continue"

assignment      = identifier "=" expr
expr_stmt       = expr

use_stmt        = "use" path ":" identifier { "," identifier }
path            = identifier { "." identifier }

expr            = or_expr
or_expr         = and_expr { "or" and_expr }
and_expr        = comp_expr { "and" comp_expr }
comp_expr       = add_expr { ("==" | "!=" | "<" | ">" | "<=" | ">=") add_expr }
add_expr        = mul_expr { ("+" | "-") mul_expr }
mul_expr        = unary { ("*" | "/" | "%") unary }
unary           = [ "-" | "not" ] primary
primary         = literal | identifier | call | method_call | field_access
                | index_access | paren_expr | lambda | struct_init | list

call            = identifier "(" [ args ] ")"
method_call     = primary "." identifier "(" [ args ] ")"
field_access    = primary "." identifier
index_access    = primary "[" expr "]"
paren_expr      = "(" expr ")"
lambda          = "fn" "(" [ params ] ")" expr
struct_init     = identifier "(" named_args ")"
list            = "[" [ expr { "," expr } ] "]"

args            = expr { "," expr }
named_args      = named_arg { "," named_arg }
named_arg       = identifier ":" expr

literal         = integer | float | string | bool | none
integer         = [ "-" ] digit { digit }
float           = [ "-" ] digit { digit } "." digit { digit }
string          = '"' { char | "$" identifier } '"'
bool            = "true" | "false"
none            = "none"

type            = identifier

identifier      = letter { letter | digit | "_" }
letter          = "a"..."z" | "A"..."Z" | "_"
digit           = "0"..."9"
```

---

## Appendix B: Feature Matrix

| Feature | Parser | Type Checker | Runtime | Overall |
|---------|--------|--------------|---------|---------|
| Basic types | ✅ | ✅ | ✅ | ✅ |
| Functions | ✅ | ✅ | ✅ | ✅ |
| Structs | ✅ | ✅ | ✅ | ✅ |
| Enums | ✅ | ✅ | ✅ | ✅ |
| Pattern matching | ✅ | ✅ | ✅ | ✅ |
| Modules | ✅ | ✅ | ✅ | ✅ |
| Concurrency | ✅ | ✅ | ✅ | ✅ |
| String methods | ✅ | ✅ | 🧪 | 🧪 |
| File I/O | ✅ | ✅ | ✅ | ✅ |
| List methods | ✅ | 🧪 | 🚧 | 🚧 |
| Generics | 🚧 | 🚧 | 🚧 | 🚧 |
| Borrow checker | 📐 | 📐 | 📐 | 📐 |
| FFI | 📐 | 📐 | 📐 | 📐 |

---

## Appendix C: Migration Path

### Phase 1: Core (Сейчас)
- ✅ Всё базовое работает
- 🧪 Добавить 5 string методов в generator

### Phase 2: Extended (Ближайший месяц)
- 🚧 Рекурсивные enum (для AST!)
- 🚧 List methods (filter, map)
- 🚧 Generic Result/Option

### Phase 3: Advanced (3-6 месяцев)
- 📐 Generics
- 📐 Borrow checker
- 📐 FFI

### Phase 4: Ecosystem (6-12 месяцев)
- 📐 Package manager
- 📐 LSP
- 📐 Formatter
