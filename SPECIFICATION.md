# U-lang Language Specification (Draft)

## 1. Overview

U-lang — язык для ИИ-генерации кода. Фокус на предсказуемость, явность, минимум магии.

**Core principles:**
1. Локально понятно — не нужно "моделировать состояние"
2. Явное лучше неявного
3. Простой синтаксис для генерации ИИ
4. Memory-safe без GC (ownership + ref counting)

---

## 2. Lexical Structure

### 2.1 Keywords

```
fn, struct, enum, trait, impl, for, in, while, loop
if, else, elif, end, match, return, break, continue
spawn, use, memory, pub, test, mut, not, and, or
```

### 2.2 Literals

```u
42          // Int
3.14        // Float
"hello"     // String
true        // Bool
[1, 2, 3]   // List
```

### 2.3 Operators

```
+ - * / %           // arithmetic
== != < > <= >=     // comparison
=                   // assignment
->                  // return type
=>                  // match arm
not and or          // logical (words!)
```

### 2.4 Comments

```u
// single line
/* multi-line */    // если нужно
```

---

## 3. Types

### 3.1 Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `Int` | 64-bit signed integer | `42`, `-10` |
| `Float` | 64-bit float | `3.14`, `-2.5` |
| `String` | UTF-8 string | `"hello"`, `"$var"` |
| `Bool` | Boolean | `true`, `false` |

### 3.2 Compound Types

**List:**
```u
numbers = [1, 2, 3]           // List of Int
names = ["a", "b"]            // List of String
empty = []                    // Empty list (type inferred)
```

**Struct:**
```u
struct Point
    x: Int
    y: Int
end

p = Point(x: 10, y: 20)
p.x = 30                      // mutate field
```

**Enum (tagged union):**
```u
enum Shape
    Circle(radius: Int)
    Rect(width: Int, height: Int)
end

s = Circle(radius: 5)
```

### 3.3 Type Inference

**Local variables:** inferred from value
```u
x = 42        // Int
y = "hello"   // String
z = [1, 2]    // List
```

**Function params:** MUST be explicit
```u
fn add(a: Int, b: Int) -> Int   // OK
fn add(a, b)                     // ERROR
```

---

## 4. Variables and Mutability

### 4.1 Immutable by Default

```u
x = 10
x = 20        // ERROR: cannot reassign
```

### 4.2 Mutable State Pattern (Wrapper)

```u
struct Counter
    value: Int
end

c = Counter(value: 0)
c.value = 10   // OK: mutate field, not variable
```

### 4.3 Mutable Parameters

```u
fn increment(mut n: Int)    // mut = can modify
    n = n + 1
end
```

---

## 5. Functions

### 5.1 Definition

```u
fn add(a: Int, b: Int) -> Int
    return a + b
end

fn greet(name: String)    // no return type = Unit
    print("Hello $name")
end
```

### 5.2 Test Functions

```u
test fn test_add()
    assert(add(2, 3) == 5)
end
```

### 5.3 Lambda (Anonymous Functions)

```u
doubled = numbers.map(fn(x) x * 2)
```

---

## 6. Control Flow

### 6.1 If

```u
if x > 0
    print("positive")
elif x < 0
    print("negative")
else
    print("zero")
end
```

### 6.2 For Loop

```u
for i in [1, 2, 3]
    print(i)
end

for name in names
    print(name)
end
```

### 6.3 While Loop

```u
while x < 10
    x = x + 1
end
```

### 6.4 Match

```u
match shape
    Circle(r) => print("radius $r")
    Rect(w, h) => print("$w x $h")
end
```

---

## 7. Concurrency

### 7.1 Spawn

```u
fn worker(id: Int)
    print("Worker $id")
end

spawn worker(1)    // new thread
```

### 7.2 Channels

```u
ch = Channel.new()
spawn receiver(ch)
ch.send("message")
```

### 7.3 Safety

- No shared mutable state between threads
- `spawn` captures by value (clone)
- Compiler checks no external mutation

---

## 8. Memory Management

### 8.1 Ownership

```u
fn consume(s: String)    // takes ownership
    print(s)
end                    // s dropped here

s = "hello"
consume(s)
// s is NOT accessible here
```

### 8.2 Borrowing

```u
fn print_len(s: String)    // borrow (read-only)
    print(s.len())
end                      // ownership returned

s = "hello"
print_len(s)
print(s)                 // OK: s still owned
```

### 8.3 Memory Statement (explicit allocation tracking)

```u
memory("buffer")
buf = allocate(1024)
```

### 8.4 Reference Counting

- Cyclic references broken by `memory` annotations
- Or manual `drop()`

---

## 9. Modules and Packages (TBD)

### 9.1 File Structure

```
project/
├── main.u
├── utils.u
└── types.u
```

### 9.2 Imports (Proposed)

```u
use utils: string_utils
use types: Point, Rectangle
```

---

## 10. FFI (Foreign Function Interface) (TBD)

### 10.1 Calling C

```u
extern fn printf(format: String, ...)

printf("Hello %s", "World")
```

---

## 11. Standard Library (TBD)

### 11.1 Core Modules

- `std.io` — file I/O
- `std.string` — string operations
- `std.list` — list operations
- `std.channel` — concurrency

### 11.2 Planned

- `std.map` — when needed
- `std.json` — JSON parsing
- `std.net` — networking

---

## 12. Grammar (EBNF)

```ebnf
program      = { statement }

statement    = fn_def | struct_def | enum_def | trait_def | impl_block
             | assignment | mutation_stmt | if_stmt | for_loop | while_loop
             | match_stmt | return_stmt | break_stmt | continue_stmt
             | spawn_stmt | expr_stmt

fn_def       = ["pub"] ["test"] "fn" identifier "(" params ")" [return_type] block "end"
struct_def   = ["pub"] "struct" identifier { typed_field } "end"
enum_def     = ["pub"] "enum" identifier { variant } "end"

assignment   = identifier "=" expression
mutation_stmt= identifier "." identifier "=" expression
if_stmt      = "if" expression block { "elif" expression block } [ "else" block ] "end"
for_loop     = "for" identifier "in" expression block "end"
while_loop   = "while" expression block "end"
match_stmt   = "match" expression { match_arm } "end"
return_stmt  = "return" [ expression ]
spawn_stmt   = "spawn" function_call

block        = { statement }
expression   = logical_expr
logical_expr = comparison { ("and" | "or") comparison }
comparison   = addition [ comp_op addition ]
addition     = multiplication { ("+" | "-") multiplication }
multiplication= unary { ("*" | "/" | "%") unary }
unary        = ["-" | "not"] primary
primary      = literal | identifier | function_call | method_call | field_access | "(" expression ")"

(* ... more details ... *)
```

---

## 13. Open Questions

1. **Generics**: needed? `List[T]` vs separate ListInt, ListString?
2. **Error handling**: Result type vs exceptions vs return codes?
3. **Traits**: full trait system or just interfaces?
4. **Packages**: module system design?
5. **Macros**: hygienic macros or no macros?

---

## Appendix A: Code Style for AI

### DO
```u
// Explicit types in functions
fn process(data: List, threshold: Int) -> Bool

// Wrapper for mutable state
struct Config
    value: Int
end

// Clear pattern matching
match option
    Some(v) => use(v)
    None => default()
end
```

### DON'T
```u
// Implicit parameter types
fn process(data, threshold)   // NO

// Reassigning variables
x = 10
x = 20                        // NO

// Symbolic logic operators
a && b                        // NO: use 'a and b'
```
