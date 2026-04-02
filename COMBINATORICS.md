# U-lang: Комбинаторика элементов

## 1. String Interpolation (уточнение)

| Синтаксис | Работает? | Пример |
|-----------|-----------|--------|
| `$var` | ✅ | `"hello $name"` |
| `$(var)` | ✅ | `"hello $(name)"` — то же самое |
| `$(expr)` | ❌ | `"$(x + y)"` — НЕ работает |

**Правило:** `$` работает только для **переменных**, не для выражений.

```u
// ✅ Работает:
name = "Alice"
print("Hello $name")      // Hello Alice
print("Hello $(name)")    // Hello Alice

// ❌ Не работает:
// print("$(name + '!')")  // Ошибка!

// Workaround:
greeting = name + "!"
print("$greeting")        // Alice!
```

---

## 2. Lifecycle: Создание, Действие, Умирание

### 2.1 Variables

```
СОЗДАНИЕ:   name = value
ДЕЙСТВИЕ:   чтение (read-only, immutable)
УМИРАНИЕ:   конец scope (автоматически)
```

```u
fn example()
    x = 10              // создание
    print("$x")         // действие (чтение)
    // x = 20          // ❌ нельзя!
end                     // x умирает здесь
```

**Взаимодействие:** Переменные передаются в функции **by value** (копия).

```u
fn modify(n: Int)
    n = n + 1           // меняем локальную копию
    print("внутри: $n") // 11
end

x = 10
modify(x)
print("снаружи: $x")    // 10 (не изменилось!)
```

---

### 2.2 Struct

```
СОЗДАНИЕ:   ИмяПоля(имя: значение, ...)
ДЕЙСТВИЕ:  
  - чтение поля: s.field
  - мутация поля: s.field = value
УМИРАНИЕ:   конец scope (автоматически)
```

```u
// Создание
p = Point(x: 10, y: 20)

// Действия
print("$(p.x)")         // чтение
p.x = 30                // мутация

// Передача в функцию — by value!
fn move_point(p: Point)
    p.x = p.x + 1       // меняем копию
end

original = Point(x: 0, y: 0)
move_point(original)
print("$(original.x)")  // 0 (не изменилось!)
```

**Взаимодействие Struct ↔ Struct:**
```u
struct Line
    start: Point
    end: Point
end

// Вложенные структуры
line = Line(
    start: Point(x: 0, y: 0),
    end: Point(x: 10, y: 10)
)
print("$(line.start.x)")  // 0
```

**Взаимодействие Struct ↔ List:**
```u
struct Point x: Int y: Int end

// Список структур
points = [
    Point(x: 0, y: 0),
    Point(x: 1, y: 1),
    Point(x: 2, y: 2)
]

// Итерация
for p in points
    print("$(p.x), $(p.y)")
end

// Доступ по индексу
first = points[0]
print("$(first.x)")      // 0
```

---

### 2.3 Enum

```
СОЗДАНИЕ:   Вариант(поле: значение, ...)
ДЕЙСТВИЕ:   pattern matching (match)
УМИРАНИЕ:   конец scope (автоматически)
```

```u
enum Shape
    Circle(radius: Int)
    Rectangle(width: Int, height: Int)
end

// Создание
c = Circle(radius: 5)
r = Rectangle(width: 10, height: 20)

// Действие — только через match!
match c
    Circle(rad) => print("радиус $rad")
    Rectangle(w, h) => print("$w x $h")
end
```

**Взаимодействие Enum ↔ Enum:**
```u
enum Option
    Some(value: Int)
    None(dummy: Int)    // нужно поле!
end

enum Result
    Ok(value: Int)
    Err(message: String)
end

// Вложение
fn divide(a: Int, b: Int) -> Result
    if b == 0
        return Err(message: "zero")
    end
    return Ok(value: a / b)
end

// Обработка Result -> Option (ручное преобразование)
match divide(10, 2)
    Ok(v) => print("success: $v")
    Err(m) => print("error: $m")
end
```

**Взаимодействие Enum ↔ Struct:**
```u
struct Drawing
    shape: Shape
    color: String
end

d = Drawing(
    shape: Circle(radius: 10),
    color: "red"
)

match d.shape
    Circle(r) => print("круг радиуса $r, цвет $(d.color)")
    Rectangle(w, h) => print("прямоугольник")
end
```

---

### 2.4 List

```
СОЗДАНИЕ:   [elem1, elem2, ...]
ДЕЙСТВИЕ:
  - чтение: lst[i]
  - длина: lst.len()
  - первый: lst.first()
  - последний: lst.last()
  - итерация: for x in lst
УМИРАНИЕ:   конец scope (автоматически)
```

```u
// Создание
nums = [1, 2, 3]

// Действия
first = nums[0]         // индексация
len = nums.len()        // длина
for n in nums           // итерация
    print("$n")
end

// ❌ Нет мутации!
// nums.push(4)         // нет!
// nums[0] = 100        // нет!
```

**Взаимодействие List ↔ List:**
```u
// Список списков (матрица)
matrix = [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9]
]
print("$(matrix[1][2])")  // 6

// Конкатенация через итерацию (ручная)
list1 = [1, 2]
list2 = [3, 4]
// ❌ Нет встроенного concat
```

**Взаимодействие List ↔ String:**
```u
// String -> List[String] (split)
s = "a,b,c"
parts = s.split(",")    // ["a", "b", "c"]

// List[String] -> String (ручно)
// ❌ Нет встроенного join!
// Workaround: итерация с накоплением
```

---

### 2.5 String

```
СОЗДАНИЕ:   "текст" или "текст $var"
ДЕЙСТВИЕ:
  - len(): длина
  - find(sub): позиция подстроки
  - slice(start, end): подстрока
  - split(delim): разбить на список
УМИРАНИЕ:   конец scope (автоматически)
```

```u
// Создание
s1 = "hello"
s2 = "world"

// Действия
len = s1.len()                  // 5
pos = (s1 + " " + s2).find("world")  // 6
sub = s1.slice(0, 2)            // "he"
words = "a b c".split(" ")       // ["a", "b", "c"]

// Конкатенация
combined = s1 + " " + s2        // "hello world"
```

**Взаимодействие String ↔ другие типы:**
```u
// Int -> String (ручно через print... но нет to_string!)
// ❌ Нет встроенного преобразования

// String -> Int (runtime функция)
// val = str_to_int("42")  // если доступно
```

---

### 2.6 Function

```
СОЗДАНИЕ:   fn имя(параметры) -> Тип ... end
  или:      fn(параметры) выражение    (lambda)
ДЕЙСТВИЕ:   вызов — имя(аргументы)
УМИРАНИЕ:   конец программы
```

```u
// Создание именованной
fn add(a: Int, b: Int) -> Int
    return a + b
end

// Создание lambda
double = fn(x) x * 2

// Вызов
result1 = add(10, 20)       // 30
result2 = double(5)         // 10

// Передача функции как аргумент
fn apply(f, x)
    return f(x)
end

triple = apply(fn(x) x * 3, 10)  // 30
```

**Взаимодействие Function ↔ Function:**
```u
// Композиция (ручная)
fn compose(f, g, x)
    return f(g(x))
end

add1 = fn(x) x + 1
mul2 = fn(x) x * 2

result = compose(add1, mul2, 5)  // (5 * 2) + 1 = 11
```

---

### 2.7 Channel

```
СОЗДАНИЕ:   Channel.new()
ДЕЙСТВИЕ:
  - send(val): отправить (блокирует если нет получателя)
  - recv(): получить (блокирует если пусто)
УМИРАНИЕ:   конец scope (автоматически, но сообщения могут потеряться!)
```

```u
// Создание
ch = Channel.new()

// Отправка в другом потоке
spawn sender(ch)

fn sender(ch)
    ch.send("hello")
end

// Получение
msg = ch.recv()
print("$msg")           // "hello"
```

**Взаимодействие Channel ↔ другие типы:**
```u
// Через channel можно передавать любые типы!
ch_point = Channel.new()
spawn send_point(ch_point)

fn send_point(ch)
    p = Point(x: 10, y: 20)
    ch.send("$(p.x),$(p.y)")    // сериализация вручную!
end

raw = ch_point.recv()           // "10,20"
// Десериализация вручную...
```

---

## 3. Таблица комбинаторики

### 3.1 Создание (Конструкторы)

| Тип | Синтаксис создания | Пример |
|-----|-------------------|--------|
| **Int** | литерал | `42`, `-10` |
| **Float** | литерал | `3.14`, `-2.5` |
| **String** | литерал / интерполяция | `"hello"`, `"hi $name"` |
| **Bool** | литерал | `true`, `false` |
| **List** | литерал | `[1, 2, 3]` |
| **Struct** | Имя(поле: значение, ...) | `Point(x: 10, y: 20)` |
| **Enum** | Вариант(поле: значение, ...) | `Circle(radius: 5)` |
| **Function** | `fn` определение | `fn add(a, b) a+b end` |
| **Lambda** | `fn(params) expr` | `fn(x) x*2` |
| **Channel** | `Channel.new()` | `ch = Channel.new()` |

### 3.2 Действия (Операции)

| ↓ На этом \ Делаем это → | Чтение | Мутация | Передача | Уничтожение |
|-------------------------|--------|---------|----------|-------------|
| **Variable** | ✅ `$var` | ❌ Нет | ✅ by value | scope end |
| **Struct поле** | ✅ `s.f` | ✅ `s.f = x` | ✅ by value | scope end |
| **List элемент** | ✅ `lst[i]` | ❌ Нет | ✅ by value | scope end |
| **Enum** | ✅ `match` | ❌ Нет | ✅ by value | scope end |
| **Channel** | ✅ `recv()` | ✅ `send()` | ✅ by ref? | scope end |
| **Function** | ✅ `call()` | ❌ Нет | ✅ as arg | program end |

### 3.3 Взаимодействие типов друг с другом

| ↓ Тип A \ Тип B → | Int | String | List | Struct | Enum | Function | Channel |
|-------------------|-----|--------|------|--------|------|----------|---------|
| **Int** | `+ - * / %` | ❌ | ❌ | ❌ | ❌ | arg | ❌ |
| **String** | ❌ | `+` concat | `split()` | ❌ | ❌ | arg | `send()` |
| **List** | `len()` | ❌ | nested | элементы | элементы | iter | ❌ |
| **Struct** | поле | поле | элемент | nested | поле | arg | `send()` |
| **Enum** | `match` | `match` | ❌ | `match` | nested | arg | ❌ |
| **Function** | return | return | return | return | return | compose | ❌ |
| **Channel** | ❌ | `send/recv` | ❌ | ❌ | ❌ | ❌ | ❌ |

Легенда:
- ✅ Работает / Возможно
- ❌ Не работает / Невозможно
- `метод()` — доступно как метод

---

## 4. Anti-patterns (что НЕ работает)

```u
// ❌ Переопределение переменной
x = 10
x = 20          // Ошибка!

// ❌ Мутация списка
lst = [1, 2, 3]
lst[0] = 100    // Ошибка!
lst.push(4)     // Ошибка!

// ❌ Выражения в интерполяции
print("$(x + y)")   // Ошибка!

// ❌ Нету unit variants
enum Status
    Pending     // Ошибка! Нужно: Pending(code: Int)
end

// ❌ Нету generics
struct Box[T]   // Ошибка!

// ❌ Нету borrow
fn f(x: &Int)   // Ошибка! Используй: fn f(x: Int)

// ❌ Нету trait objects
fn draw(d: Drawable)    // Ошибка! Drawable — не тип

// ❌ Нету исключений
throw Error()   // Ошибка! Используй: return Err(...)

// ❌ Нету null
x = null        // Ошибка! Используй: enum Option
```

---

## 5. Idioms (правильные паттерны)

### Мутация через Wrapper
```u
struct Counter value: Int end
c = Counter(value: 0)
c.value = c.value + 1
```

### Error handling через enum
```u
enum Result
    Ok(val: Int)
    Err(msg: String)
end
```

### Optional values через enum
```u
enum Option
    Some(val: Int)
    None(dummy: Int)
end
```

### Итерация с накоплением
```u
sum = 0
for n in numbers
    sum = sum + n
end
```

### Конкатенация строк
```u
full = part1 + " " + part2
```

---

## 6. Quick Reference Card

```u
// ПЕРЕМЕННЫЕ
x = 10                              // immutable

// ФУНКЦИИ
fn f(a: Int) -> Int ... end         // с возвратом
fn g() ... end                      // без возврата

// СТРУКТУРЫ
struct S f: Int end
s = S(f: 10)
s.f = 20                            // мутация поля

// ENUM
enum E A(x: Int) B(y: Int) end
match e
    A(x) => ...
    B(y) => ...
end

// СПИСКИ
lst = [1, 2, 3]
lst[0]                              // доступ
lst.len()                           // длина
for x in lst ... end                // итерация

// СТРОКИ
s = "hello"
s.len()                             // длина
s.slice(0, 2)                       // подстрока
s.split(" ")                         // в список
s1 + s2                             // конкатенация
"$var"                              // интерполяция

// КОНКУРЕНТНОСТЬ
ch = Channel.new()
spawn f(ch)
ch.send(val)
ch.recv()
```
