# U-lang Complete Specification

**Legend:**
- ✅ Implemented — работает сейчас
- 🔄 In Progress — в разработке
- 📋 Planned — запланировано
- ❓ TBD — нужно решить

---

## 1. Lexical Structure

### 1.1 Keywords

| Keyword | Status | Notes |
|---------|--------|-------|
| `fn` | ✅ | function definition |
| `struct` | ✅ | struct definition |
| `enum` | ✅ | enum (tagged union) |
| `trait` | ✅ | interface definition |
| `impl` | ✅ | implementation |
| `for` | ✅ | for loop |
| `in` | ✅ | for-in iterator |
| `while` | ✅ | while loop |
| `loop` | ✅ | infinite loop |
| `if` | ✅ | conditional |
| `else` | ✅ | else branch |
| `elif` | ✅ | else-if branch |
| `end` | ✅ | block terminator |
| `match` | ✅ | pattern matching |
| `return` | ✅ | return from function |
| `break` | ✅ | break loop |
| `continue` | ✅ | continue loop |
| `spawn` | ✅ | spawn thread |
| `use` | ✅ | module import |
| `memory` | ✅ | explicit memory tracking |
| `pub` | ✅ | public visibility |
| `test` | ✅ | test function |
| `mut` | ✅ | mutable parameter |
| `not` | ✅ | logical NOT |
| `and` | ✅ | logical AND |
| `or` | ✅ | logical OR |
| `as` | 📋 | type cast |
| `async` | ❓ TBD | async/await |
| `await` | ❓ TBD | async/await |
| `const` | ❓ TBD | compile-time constant |
| `static` | ❓ TBD | static lifetime |
| `import` | ❓ TBD | alternative to `use` |
| `export` | ❓ TBD | module exports |
| `try` | ❓ TBD | error handling |
| `catch` | ❓ TBD | error handling |
| `throw` | ❓ TBD | error handling |
| `defer` | ❓ TBD | deferred execution |
| `panic` | ❓ TBD | unrecoverable error |
| `unsafe` | ❓ TBD | unsafe block |
| `where` | ❓ TBD | trait bounds |

### 1.2 Literals

| Type | Syntax | Status | Example |
|------|--------|--------|---------|
| Integer | decimal | ✅ | `42`, `-10` |
| Integer | hex | 📋 | `0xFF`, `0x1A` |
| Integer | binary | 📋 | `0b1010` |
| Integer | octal | 📋 | `0o777` |
| Float | decimal | ✅ | `3.14`, `-2.5` |
| Float | scientific | 📋 | `1.5e10` |
| String | double quote | ✅ | `"hello"` |
| String | interpolation | ✅ | `"value: $x"` |
| String | raw | ✅ | `#"raw"#` |
| String | multiline | 📋 | `\"""multi"""` |
| Bool | | ✅ | `true`, `false` |
| None | | ✅ | `none` |
| List | | ✅ | `[1, 2, 3]` |
| Char | | 📋 | `'a'` |

### 1.3 Operators

| Operator | Meaning | Status |
|----------|---------|--------|
| `+` | addition | ✅ |
| `-` | subtraction/negation | ✅ |
| `*` | multiplication | ✅ |
| `/` | division | ✅ |
| `%` | modulo | ✅ |
| `==` | equality | ✅ |
| `!=` | inequality | ✅ |
| `<` | less than | ✅ |
| `>` | greater than | ✅ |
| `<=` | less or equal | ✅ |
| `>=` | greater or equal | ✅ |
| `=` | assignment | ✅ |
| `->` | return type | ✅ |
| `=>` | match arm | ✅ |
| `&` | borrow | 📋 |
| `&mut` | mutable borrow | 📋 |
| `|` | union type | ❓ TBD |
| `..` | range | 📋 |
| `..=` | inclusive range | 📋 |
| `?` | error propagation | ❓ TBD |
| `!` | never type | ❓ TBD |

### 1.4 Delimiters

| Symbol | Usage | Status |
|--------|-------|--------|
| `()` | grouping, function call | ✅ |
| `[]` | list, index access | ✅ |
| `{}` | block (future) | ❓ TBD |
| `:` | type annotation, named arg | ✅ |
| `,` | separator | ✅ |
| `.` | field access, method call | ✅ |
| `//` | line comment | ✅ |
| `/* */` | block comment | 📋 |
| `///` | doc comment | 📋 |

---

## 2. Types

### 2.1 Primitive Types

| Type | Description | Size | Status |
|------|-------------|------|--------|
| `Int` | signed integer | 64-bit | ✅ |
| `Int8` | signed integer | 8-bit | 📋 |
| `Int16` | signed integer | 16-bit | 📋 |
| `Int32` | signed integer | 32-bit | 📋 |
| `Int64` | signed integer | 64-bit | 📋 |
| `UInt` | unsigned integer | 64-bit | 📋 |
| `UInt8` | unsigned integer | 8-bit | 📋 |
| `UInt16` | unsigned integer | 16-bit | 📋 |
| `UInt32` | unsigned integer | 32-bit | 📋 |
| `UInt64` | unsigned integer | 64-bit | 📋 |
| `Float` | floating point | 64-bit | ✅ |
| `Float32` | floating point | 32-bit | 📋 |
| `Float64` | floating point | 64-bit | ✅ (alias) |
| `Bool` | boolean | 1-bit | ✅ |
| `String` | UTF-8 string | dynamic | ✅ |
| `Char` | UTF-8 character | 4 bytes | 📋 |

### 2.2 Compound Types

| Type | Syntax | Status |
|------|--------|--------|
| `List[T]` | `[1, 2, 3]` | ✅ (T inferred) |
| `Tuple` | `(1, "a", true)` | 📋 |
| `Array[T; N]` | fixed-size array | 📋 |
| `Map[K, V]` | key-value storage | ❓ TBD |
| `Set[T]` | unique elements | ❓ TBD |
| `Option[T]` | `Some(T)` or `None` | ✅ (via enum) |
| `Result[T, E]` | `Ok(T)` or `Err(E)` | ✅ (via enum) |
| `Pointer[T]` | raw pointer | ❓ TBD |
| `Reference[T]` | borrowed reference | 📋 |

### 2.3 User-Defined Types

| Type | Syntax | Status |
|------|--------|--------|
| `struct` | `struct Point x: Int, y: Int end` | ✅ |
| `enum` | `enum Shape Circle(r: Int) end` | ✅ |
| `trait` | `trait Drawable fn draw() end` | ✅ |
| `type alias` | `typealias Point = (Int, Int)` | 📋 |

### 2.4 Function Types

| Type | Syntax | Status |
|------|--------|--------|
| Function | `fn(Int, Int) -> Int` | ❓ TBD |
| Closure | `fn(Int) -> Int + captures` | ❓ TBD |

### 2.5 Generic Types

| Feature | Syntax | Status |
|---------|--------|--------|
| Generic struct | `struct Box[T] value: T end` | ❓ TBD |
| Generic enum | `enum Option[T] Some(v: T) None end` | ❓ TBD |
| Generic function | `fn identity[T](x: T) -> T` | ❓ TBD |
| Trait bounds | `fn print[T: Display](x: T)` | ❓ TBD |

---

## 3. Variables and Mutability

### 3.1 Variable Declaration

| Syntax | Meaning | Status |
|--------|---------|--------|
| `x = 10` | immutable variable | ✅ |
| `mut x = 10` | mutable variable | ❓ TBD |
| `const X = 10` | compile-time constant | ❓ TBD |
| `static X = 10` | static variable | ❓ TBD |

### 3.2 Mutability Rules

| Context | Mutable? | Syntax | Status |
|---------|----------|--------|--------|
| Local var | No | `x = 10` | ✅ |
| Local var | Yes | `mut x = 10` | ❓ TBD |
| Function param | No | `fn f(x: Int)` | ✅ |
| Function param | Yes | `fn f(mut x: Int)` | ✅ |
| Struct field | Yes | `s.field = val` | ✅ |
| Global | ❓ | | ❓ TBD |

### 3.3 Wrapper Pattern (Mutable State)

```u
struct Counter
    value: Int
end

c = Counter(value: 0)
c.value = 10   // ✅ mutate field
```

---

## 4. Functions

### 4.1 Function Definition

| Syntax | Status |
|--------|--------|
| `fn name() -> Type body end` | ✅ |
| `fn name() body end` | ✅ (no return) |
| `fn name(x: Int) -> Int body end` | ✅ |
| `fn name(mut x: Int) body end` | ✅ |
| `fn name[T](x: T) -> T body end` | ❓ TBD |
| `fn name(x: Int = 0) body end` | ❓ TBD (default params) |
| `fn name(args: Int...) body end` | ❓ TBD (variadic) |

### 4.2 Special Function Types

| Type | Syntax | Status |
|------|--------|--------|
| Test | `test fn name() end` | ✅ |
| Public | `pub fn name() end` | ✅ |
| Async | `async fn name() end` | ❓ TBD |
| Unsafe | `unsafe fn name() end` | ❓ TBD |
| Extern | `extern fn name()` | ❓ TBD |
| Method | `fn self.method() end` | ✅ (via impl) |

### 4.3 Lambda/Closures

| Syntax | Status |
|--------|--------|
| `fn(x) x + 1` | ✅ |
| `fn(x: Int) -> Int x + 1` | ❓ TBD (typed) |
| `|x| x + 1` | ❓ TBD (shorthand) |
| Closures with captures | ❓ TBD |

---

## 5. Control Flow

### 5.1 Conditionals

| Syntax | Status |
|--------|--------|
| `if cond body end` | ✅ |
| `if cond body else body end` | ✅ |
| `if cond body elif cond body end` | ✅ |
| `if cond body elif cond body else body end` | ✅ |
| `unless cond body end` | ❓ TBD |
| Ternary `cond ? a : b` | ❓ TBD |

### 5.2 Loops

| Syntax | Status |
|--------|--------|
| `for i in iterable body end` | ✅ |
| `while cond body end` | ✅ |
| `loop body end` | ✅ |
| `for i in 0..10 body end` | 📋 |
| `for i in 0..=10 body end` | 📋 |
| `break` | ✅ |
| `break value` | ❓ TBD (break with value) |
| `continue` | ✅ |
| `return` | ✅ |
| `return value` | ✅ |

### 5.3 Pattern Matching

| Syntax | Status |
|--------|--------|
| `match expr arm end` | ✅ |
| Literal pattern `1 =>` | ✅ |
| Variable pattern `x =>` | ✅ |
| Wildcard `_ =>` | 📋 |
| Constructor `Circle(r) =>` | ✅ |
| Guard `x if x > 0 =>` | 📋 |
| Multiple `1 | 2 =>` | ❓ TBD |
| Range `1..5 =>` | ❓ TBD |
| List `[a, b] =>` | ❓ TBD |
| Binding `x @ Pattern =>` | ❓ TBD |

---

## 6. Structs and Enums

### 6.1 Struct Definition

| Syntax | Status |
|--------|--------|
| `struct Name field: Type end` | ✅ |
| `pub struct Name field: Type end` | ✅ |
| `struct Name impl Trait end` | ❓ TBD |
| Tuple struct `struct Name(Int, Int)` | 📋 |
| Unit struct `struct Name` | 📋 |
| `struct Name[T] field: T end` | ❓ TBD |

### 6.2 Struct Construction

| Syntax | Status |
|--------|--------|
| `Name(field: value)` | ✅ |
| `Name { field: value }` | ❓ TBD |
| `Name(value, value)` | 📋 (tuple struct) |
| `Name` | 📋 (unit struct) |
| Struct update `Name { ..base, field: new }` | ❓ TBD |

### 6.3 Enum Definition

| Syntax | Status |
|--------|--------|
| `enum Name Variant(field: Type) end` | ✅ |
| `enum Name UnitVariant end` | ❓ TBD |
| `enum Name[T] Some(v: T) None end` | ❓ TBD |

### 6.4 Enum Construction

| Syntax | Status |
|--------|--------|
| `Variant(field: value)` | ✅ |
| `Name.Variant` | ❓ TBD |

---

## 7. Traits and Impl

### 7.1 Trait Definition

| Syntax | Status |
|--------|--------|
| `trait Name fn method() end end` | ✅ |
| `trait Name: SuperTrait end` | ❓ TBD (supertraits) |
| `trait Name[T] end` | ❓ TBD (generic traits) |
| Associated types `type Output` | ❓ TBD |
| Default implementations | 📋 |

### 7.2 Implementation

| Syntax | Status |
|--------|--------|
| `impl Trait for Type fn method() end end` | ✅ |
| `impl Type fn method() end end` | ✅ (inherent) |
| `impl Trait for Type where T: Bound` | ❓ TBD |
| Blanket impl `impl[T] Trait for T` | ❓ TBD |

---

## 8. Modules and Packages

### 8.1 Module System

| Feature | Syntax | Status |
|---------|--------|--------|
| Import items | `use module: item` | ✅ |
| Import multiple | `use module: a, b, c` | ✅ |
| Nested modules | `use a.b.c: item` | ✅ |
| Import all | `use module: *` | ❓ TBD |
| Import as | `use module: item as alias` | ❓ TBD |
| Module declaration | `module name` | ❓ TBD |
| Re-export | `pub use module: item` | ❓ TBD |

### 8.2 Visibility

| Modifier | Meaning | Status |
|----------|---------|--------|
| (none) | private | ✅ |
| `pub` | public | ✅ |
| `pub(crate)` | crate-public | ❓ TBD |
| `pub(super)` | parent-public | ❓ TBD |
| `pub(in path)` | restricted | ❓ TBD |

---

## 9. Concurrency

### 9.1 Threading

| Feature | Syntax | Status |
|---------|--------|--------|
| Spawn thread | `spawn function()` | ✅ |
| Spawn with args | `spawn func(arg)` | ✅ |
| Thread ID | ❓ | ❓ TBD |
| Thread join | ❓ | ❓ TBD |
| Thread local | `thread_local` | ❓ TBD |

### 9.2 Communication

| Feature | Syntax | Status |
|---------|--------|--------|
| Create channel | `Channel.new()` | ✅ |
| Send | `ch.send(value)` | ✅ |
| Receive | `ch.recv()` | ✅ |
| Try send | `ch.try_send(value)` | 📋 |
| Try receive | `ch.try_recv()` | 📋 |
| Select | `select { ch1.recv() => ... }` | ❓ TBD |
| Close channel | `ch.close()` | 📋 |

### 9.3 Synchronization

| Feature | Syntax | Status |
|---------|--------|--------|
| Mutex | `Mutex.new(value)` | ❓ TBD |
| RwLock | `RwLock.new(value)` | ❓ TBD |
| Condvar | `Condvar.new()` | ❓ TBD |
| Barrier | `Barrier.new(n)` | ❓ TBD |
| Atomic types | `AtomicInt`, etc. | ❓ TBD |

### 9.4 Async/Await

| Feature | Syntax | Status |
|---------|--------|--------|
| Async function | `async fn name() end` | ❓ TBD |
| Await | `await expression` | ❓ TBD |
| Future type | `Future[T]` | ❓ TBD |
| Async block | `async { ... }` | ❓ TBD |

---

## 10. Memory Management

### 10.1 Ownership

| Concept | Status | Notes |
|---------|--------|-------|
| Move semantics | ✅ | assignment moves value |
| Copy trait | ❓ TBD | implicit copy for primitives |
| Clone trait | 📋 | explicit `.clone()` |
| Drop trait | ❓ TBD | destructor |

### 10.2 Borrowing

| Concept | Syntax | Status |
|---------|--------|--------|
| Immutable borrow | `&x` | 📋 |
| Mutable borrow | `&mut x` | 📋 |
| Dereference | `*x` | 📋 |
| Borrow checker | | ❓ TBD |
| Lifetimes | `'a` | ❓ TBD |
| Lifetime elision | | ❓ TBD |

### 10.3 Smart Pointers

| Type | Syntax | Status |
|------|--------|--------|
| Box | `Box.new(value)` | ❓ TBD |
| Rc | `Rc.new(value)` | ❓ TBD |
| Arc | `Arc.new(value)` | ❓ TBD |
| RefCell | `RefCell.new(value)` | ❓ TBD |
| Weak | `Rc.downgrade()` | ❓ TBD |

### 10.4 Explicit Memory

| Feature | Syntax | Status |
|---------|--------|--------|
| Memory tracking | `memory("label")` | ✅ |
| Manual drop | `drop(value)` | ❓ TBD |
| Forget | `forget(value)` | ❓ TBD |
| Memory size | `size_of[T]()` | ❓ TBD |
| Alignment | `align_of[T]()` | ❓ TBD |

---

## 11. Error Handling

### 11.1 Current Approach (via enum)

```u
enum Result
    Ok(value: Int)
    Err(message: String)
end
```
✅ Работает сейчас

### 11.2 Planned Approaches

| Approach | Syntax | Status |
|----------|--------|--------|
| Result type | `Result[T, E]` | ❓ TBD |
| Option type | `Option[T]` | ❓ TBD |
| Try operator | `expr?` | ❓ TBD |
| Try block | `try { ... } catch { ... }` | ❓ TBD |
| Panic | `panic("msg")` | ❓ TBD |
| Unwrap | `value.unwrap()` | ❓ TBD |
| Expect | `value.expect("msg")` | ❓ TBD |

---

## 12. Standard Library

### 12.1 std.io (Input/Output)

| Function | Status |
|----------|--------|
| `print(s)` | ✅ |
| `println(s)` | ❓ TBD |
| `read_line()` | ❓ TBD |
| `read_file(path)` | 📋 |
| `write_file(path, content)` | 📋 |
| `append_file(path, content)` | 📋 |
| `file_exists(path)` | 📋 |
| `stdin()` | ❓ TBD |
| `stdout()` | ❓ TBD |
| `stderr()` | ❓ TBD |

### 12.2 std.string

| Method | Status |
|--------|--------|
| `s.len()` | ✅ |
| `s.char_at(i)` | 📋 |
| `s.substring(start, end)` | 📋 |
| `s.split(sep)` | 📋 |
| `s.split_lines()` | 📋 |
| `s.trim()` | 📋 |
| `s.starts_with(prefix)` | 📋 |
| `s.ends_with(suffix)` | 📋 |
| `s.contains(substr)` | 📋 |
| `s.find(substr)` | 📋 |
| `s.replace(old, new)` | 📋 |
| `s.to_upper()` | 📋 |
| `s.to_lower()` | 📋 |
| `s.parse_int()` | 📋 |
| `s.parse_float()` | 📋 |

### 12.3 std.list

| Method | Status |
|--------|--------|
| `lst.len()` | ❓ TBD |
| `lst.get(i)` | ❓ TBD |
| `lst.set(i, val)` | ❓ TBD |
| `lst.push(val)` | ❓ TBD |
| `lst.pop()` | ❓ TBD |
| `lst.insert(i, val)` | ❓ TBD |
| `lst.remove(i)` | ❓ TBD |
| `lst.clear()` | ❓ TBD |
| `lst.contains(val)` | ❓ TBD |
| `lst.find(pred)` | ❓ TBD |
| `lst.filter(pred)` | ❓ TBD |
| `lst.map(f)` | ❓ TBD |
| `lst.fold(init, f)` | ❓ TBD |
| `lst.reduce(f)` | ❓ TBD |
| `lst.sort()` | ❓ TBD |
| `lst.reverse()` | ❓ TBD |
| `lst.join(sep)` | ❓ TBD |

### 12.4 std.channel

| Method | Status |
|--------|--------|
| `Channel.new()` | ✅ |
| `ch.send(val)` | ✅ |
| `ch.recv()` | ✅ |
| `ch.try_send(val)` | 📋 |
| `ch.try_recv()` | 📋 |
| `ch.close()` | 📋 |
| `ch.is_closed()` | 📋 |

### 12.5 std.time

| Function | Status |
|----------|--------|
| `now()` | ❓ TBD |
| `sleep(ms)` | ❓ TBD |
| `duration_since(t)` | ❓ TBD |

### 12.6 std.random

| Function | Status |
|----------|--------|
| `random()` | ❓ TBD |
| `random_int(min, max)` | ❓ TBD |
| `random_float(min, max)` | ❓ TBD |
| `random_choice(list)` | ❓ TBD |
| `random_shuffle(list)` | ❓ TBD |

### 12.7 std.math

| Function | Status |
|----------|--------|
| `abs(x)` | ❓ TBD |
| `min(a, b)` | ❓ TBD |
| `max(a, b)` | ❓ TBD |
| `sqrt(x)` | ❓ TBD |
| `pow(x, y)` | ❓ TBD |
| `sin(x)` | ❓ TBD |
| `cos(x)` | ❓ TBD |
| `tan(x)` | ❓ TBD |
| `log(x)` | ❓ TBD |
| `exp(x)` | ❓ TBD |
| `floor(x)` | ❓ TBD |
| `ceil(x)` | ❓ TBD |
| `round(x)` | ❓ TBD |

### 12.8 std.fs (File System)

| Function | Status |
|----------|--------|
| `read_dir(path)` | ❓ TBD |
| `create_dir(path)` | ❓ TBD |
| `remove_dir(path)` | ❓ TBD |
| `remove_file(path)` | ❓ TBD |
| `rename(old, new)` | ❓ TBD |
| `copy(src, dst)` | ❓ TBD |
| `metadata(path)` | ❓ TBD |

---

## 13. Metaprogramming

### 13.1 Macros

| Feature | Syntax | Status |
|---------|--------|--------|
| Declarative macros | `macro! { ... }` | ❓ TBD |
| Procedural macros | `#[derive(...)]` | ❓ TBD |
| Hygiene | | ❓ TBD |
| Compile-time execution | `comptime { ... }` | ❓ TBD |

### 13.2 Reflection

| Feature | Status |
|---------|--------|
| Type introspection | ❓ TBD |
| Type name | ❓ TBD |
| Field access by name | ❓ TBD |
| Method call by name | ❓ TBD |

---

## 14. FFI (Foreign Function Interface)

| Feature | Syntax | Status |
|---------|--------|--------|
| C function declaration | `extern fn printf(fmt: String, ...)` | ❓ TBD |
| C library linking | `#[link("libm")]` | ❓ TBD |
| C types | `c_int`, `c_char`, etc. | ❓ TBD |
| Pointer types | `*T`, `*mut T` | ❓ TBD |
| Unsafe block | `unsafe { ... }` | ❓ TBD |
| Unsafe function | `unsafe fn name() end` | ❓ TBD |

---

## 15. Attributes/Annotations

| Attribute | Purpose | Status |
|-----------|---------|--------|
| `#[test]` | test function | ✅ (test fn) |
| `#[main]` | entry point | ❓ TBD |
| `#[inline]` | inline function | ❓ TBD |
| `#[no_mangle]` | FFI name | ❓ TBD |
| `#[derive(...)]` | auto-impl | ❓ TBD |
| `#[doc = "..."]` | documentation | ❓ TBD |
| `#[deprecated]` | deprecation | ❓ TBD |
| `#[cfg(...)]` | conditional compilation | ❓ TBD |

---

## Appendix: Implementation Priority

### Phase 1: Core (блокирует self-hosting)
1. ✅ Basic types (Int, Float, String, Bool)
2. ✅ Functions and control flow
3. ✅ Structs and enums
4. ✅ Pattern matching
5. ✅ Basic I/O (print)

### Phase 2: Essential (делает язык юзабельным)
1. 📋 String methods (char_at, substring, split)
2. 📋 File I/O (read_file, write_file)
3. 📋 List methods (push, pop, get)
4. ❓ TBD Error handling (Result, Option)
5. ❓ TBD Generics

### Phase 3: Advanced (для production)
1. ❓ TBD Traits and impl
2. ❓ TBD Borrow checker
3. ❓ TBD Generics with bounds
4. ❓ TBD Async/await
5. ❓ TBD FFI

### Phase 4: Ecosystem
1. ❓ TBD Package manager
2. ❓ TBD LSP server
3. ❓ TBD Formatter
4. ❓ TBD Linter
5. ❓ TBD Documentation generator

---

## Summary

| Category | ✅ | 📋 | ❓ TBD | Total |
|----------|-----|-----|--------|-------|
| Lexical | 10 | 5 | 10 | 25 |
| Types | 8 | 12 | 8 | 28 |
| Variables | 3 | 1 | 3 | 7 |
| Functions | 5 | 1 | 5 | 11 |
| Control Flow | 8 | 3 | 5 | 16 |
| Structs/Enums | 6 | 2 | 4 | 12 |
| Traits | 2 | 1 | 4 | 7 |
| Modules | 3 | 0 | 5 | 8 |
| Concurrency | 4 | 4 | 9 | 17 |
| Memory | 1 | 3 | 8 | 12 |
| Error Handling | 1 | 0 | 6 | 7 |
| Std Lib | 4 | 15 | 31 | 50 |
| **Total** | **55** | **47** | **98** | **200** |

**27.5% реализовано**, **23.5% в планах**, **49% TBD**
