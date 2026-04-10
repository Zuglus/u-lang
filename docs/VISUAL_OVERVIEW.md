# U-lang: Визуальный Overview

## Что это такое?

U — язык со **строгой типизацией**, но **без скобочек** и **с end-terminator'ами**.

Синтаксис похож на:
- **Ruby/Python** — нет скобочек, человекочитаемый
- **Rust** — статическая типизация, pattern matching
- **Lua** — `end` вместо `{}`

---

## Hello World

```u
print("Hello, World!")
```

**Заметь:** нет `main()`, нет `import`, нет точки с запятой.

---

## Переменные

```u
// Простые типы
name = "Alice"      // String
age = 30            // Int
pi = 3.14           // Float
active = true       // Bool

// String interpolation
message = "Hello, $name!"   // "Hello, Alice!"
```

**Важно:** переменные immutable (нельзя переопределить).
```u
x = 10
x = 20      // ❌ Ошибка!
```

**Мутация через Wrapper:**
```u
struct Counter
    value: Int
end

c = Counter(value: 0)
c.value = 10     // ✅ OK — мутируем поле структуры
```

---

## Функции

```u
// Без параметров
fn greet()
    print("Hello!")
end

// С параметрами (типы обязательны!)
fn add(a: Int, b: Int) -> Int
    return a + b
end

// Использование
result = add(5, 3)
print("$result")        // 8
```

**Важно:** `-> Type` для возвращаемого значения, `end` в конце.

---

## Структуры

```u
struct Person
    name: String
    age: Int
end

// Создание (именованные параметры!)
alice = Person(name: "Alice", age: 30)

// Доступ
print(alice.name)       // "Alice"
alice.age = 31          // ✅ Мутация полей разрешена
```

**Ограничение:** типы полей — только простые идентификаторы.
```u
struct Data
    items: List       // ✅ OK
    // items: List[Int]   // ❌ НЕ работает!
end
```

---

## Enum (Tagged Unions)

```u
enum Shape
    Circle(radius: Int)
    Rectangle(width: Int, height: Int)
end

// Создание
c = Circle(radius: 10)
r = Rectangle(width: 20, height: 30)
```

**Важно:** варианты ОБЯЗАТЕЛЬНО должны иметь поля!
```u
// enum Status
//     Pending           // ❌ НЕ работает (нет полей)
// end

enum Status
    Pending(code: Int)  // ✅ OK
end
```

---

## Pattern Matching

```u
match shape
    Circle(r) => 
        print("Circle with radius $r")
    Rectangle(w, h) =>
        print("Rectangle $w x $h")
    _ =>
        print("Other shape")      // wildcard
end
```

---

## Control Flow

### If-Else

```u
if x > 0
    print("positive")
elif x < 0
    print("negative")
else
    print("zero")
end
```

**Важно:** `elif` а не `else if`, `end` в конце.

### Loops

```u
// For (итерация)
for item in items
    print("$item")
end

// While
while x > 0
    x = x - 1
end

// Infinite
loop
    if done
        break
    end
end
```

---

## Lists

```u
// Создание
numbers = [1, 2, 3, 4, 5]

// Доступ
first = numbers[0]          // 1
last = numbers.last()       // 5 (работает!)
len = numbers.len()         // 5 (работает!)

// Итерация
for n in numbers
    print("$n")
end

// НЕ работает (пока):
// numbers.push(6)          // ❌
// numbers.filter(fn(x) x > 2)  // ❌
```

---

## Модули

```u
// Импорт
use math: add, multiply
use utils.strings: shout

// Использование
result = add(10, 20)
msg = shout("hello")
```

**Структура файлов:**
```
project/
├── main.u
└── modules/
    ├── math.u
    └── utils/
        └── strings.u
```

---

## Конкурентность ( spawn / channels )

```u
// Создаём канал
ch = Channel.new()

// Запускаем поток
spawn worker(ch)

fn worker(ch)
    // Отправляем сообщение
    ch.send("Hello from thread!")
end

// Получаем сообщение (блокируется)
msg = ch.recv()
print("$msg")
```

**Модель:** нет shared memory, только message passing.

---

## String Methods

```u
s = "hello world"

// Работают:
s.len()                     // 11
s.find("world")             // 6 (позиция или -1)
s.slice(0, 5)               // "hello" (substring)
s.split(" ")                // ["hello", "world"]
s.split_lines()             // по \n
s.first()                   // "h"
s.last()                    // "d"

// Не работают (но есть в runtime):
// s.trim()                 // 🚧
// s.contains("ell")        // 🚧
// s.starts_with("he")      // 🚧

// Workaround:
// s.char_at(i) -> s.slice(i, i+1)
char = s.slice(0, 1)        // "h"
```

---

## File I/O

```u
// Чтение
content = read_file("input.txt")

// Запись
write_file("output.txt", "Hello!")

// Директории
create_dir("my_folder")
files = list_dir(".")       // List[String]
```

**Важно:** это свободные функции, не методы.

---

## Lambda (Anonymous Functions)

```u
// Простая lambda
double = fn(x) x * 2
result = double(5)          // 10

// Высший порядок
fn apply(f, x)
    return f(x)
end

result = apply(fn(x) x * 2, 5)   // 10
```

---

## Trait + Impl

```u
// Определяем интерфейс
trait Drawable
    fn draw()
end

// Реализуем для структуры
impl Drawable for Circle
    fn draw()
        print("Drawing circle")
    end
end

// Используем
fn render(d: Drawable)      // ❌ НЕ работает — нет trait objects
    d.draw()
end

// Работает:
c = Circle(radius: 10)
c.draw()                    // "Drawing circle"
```

---

## Чего НЕТ (и что вместо этого)

| Что ожидаешь | Что в U-lang |
|--------------|--------------|
| `x = 10; x = 20` | ❌ Нет — используй Wrapper struct |
| `if (x > 0) { ... }` | ❌ Нет — `if x > 0 ... end` |
| `for i in 0..10` | ❌ Нет — пока только `for i in list` |
| `enum Option<T>` | ❌ Нет — только concrete types |
| `Result<T, E>` | ❌ Нет — свой enum: `enum Result Ok(v: Int) Err(m: String) end` |
| `&T`, `&mut T` | ❌ Нет — copy-by-value |
| `a[i] = x` | ❌ Нет — lists immutable |
| `s[i]` (string index) | ❌ Нет — `s.slice(i, i+1)` |

---

## Quick Reference

```u
// ПЕРЕМЕННЫЕ
x = 10                      // immutable
s = "hello $x"             // interpolation

// ФУНКЦИИ
fn add(a: Int, b: Int) -> Int
    return a + b
end

// СТРУКТУРЫ
struct Point x: Int y: Int end
p = Point(x: 10, y: 20)

// ENUM
enum Shape Circle(r: Int) Rectangle(w: Int, h: Int) end

// MATCH
match shape
    Circle(r) => print("$r")
    _ => print("other")
end

// LOOPS
for x in list ... end
while cond ... end
loop ... end

// MODULES
use math: add

// CONCURRENCY
ch = Channel.new()
spawn worker(ch)
msg = ch.recv()
```

---

## Сравнение с другими языками

### U-lang vs Python

```python
# Python
def add(a, b):
    return a + b

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
```

```u
// U-lang
fn add(a: Int, b: Int) -> Int
    return a + b
end

struct Point
    x: Int
    y: Int
end
```

### U-lang vs Rust

```rust
// Rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}

struct Point { x: i64, y: i64 }
```

```u
// U-lang
fn add(a: Int, b: Int) -> Int
    return a + b
end

struct Point
    x: Int
    y: Int
end
```

---

## Что работает прямо сейчас?

✅ **Работает:**
- Все базовые типы (Int, Float, String, Bool)
- Функции с типами
- Структуры и enum
- Pattern matching
- Модули (use)
- spawn/channels
- String methods (len, find, slice, split)
- File I/O (read_file, write_file)

🚧 **Не работает (из ключевого):**
- Generics (`List[Int]` в полях)
- Recursive enum (для AST)
- List methods (push, pop, filter, map)
- Borrow checker (`&T`)
- Trait bounds

---

## Полезные трюки

### 1. "Мутабельная" переменная через struct

```u
struct Var
    value: Int
end

x = Var(value: 10)
x.value = 20        // "Мутация"
```

### 2. Option/Result через enum

```u
enum Result
    Ok(v: Int)
    Err(msg: String)
end

fn divide(a: Int, b: Int) -> Result
    if b == 0
        return Err(msg: "division by zero")
    end
    return Ok(v: a / b)
end

// Использование
match divide(10, 2)
    Ok(v) => print("$v")
    Err(m) => print("Error: $m")
end
```

### 3. Char через slice

```u
s = "hello"
first = s.slice(0, 1)    // "h"
third = s.slice(2, 3)    // "l"
```
