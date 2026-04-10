# U-lang — Возможности (статус реализации)

## ✅ Реализовано

### Базовый синтаксис
- [x] Переменные (`name = value`)
- [x] Типы в сигнатурах (`fn foo(x: Int) -> Int`)
- [x] Вывод типов (внутри функций)
- [x] `if/elif/else/end`
- [x] `for item in collection`
- [x] `while condition`
- [x] `loop` (бесконечный)
- [x] `break`, `continue`
- [x] `return`

### Типы
- [x] `Int`, `Float`, `String`, `Bool`
- [x] `List[T]`
- [x] `struct`
- [x] `enum` (алгебраические типы)
- [x] `trait`
- [x] `impl` для struct
- [x] `impl Trait for Type`
- [x] Generics `[T]` в сигнатурах

### Контроль мутации
- [x] `mut` в параметрах функций
- [x] Автоматический borrow (`&` / `&mut`)
- [x] Проверка мутации внешних данных в `spawn`

### Конкурентность
- [x] `spawn`
- [x] `Channel` (через std.channel)

### Строки
- [x] Интерполяция `$var` и `$(expr)`
- [x] Raw strings `#"..."#`
- [x] Многострочные строки

### Методы
- [x] Dot-методы для String, List, Int
- [x] Методы структур через `impl`

### Тесты
- [x] `test fn`
- [x] `assert`, `assert_eq`

### Модули
- [x] `use имя_файла: имя` (файловая система)
- [x] `.rs` модули (через use имя: функция)

### Память
- [x] Compile-time cycle detection
- [x] Stack-first размещение
- [x] Move semantics

### I/O (базовое)
- [x] `print`
- [x] `read_file`
- [x] `write_file`
- [x] `list_dir`
- [x] `create_dir`
- [x] `copy_file`
- [x] `is_dir`

### CLI
- [x] `u run` — компиляция + запуск
- [x] `u build` — бинарник
- [x] `u check` — проверка

---

## 🚧 Реализовать

### Язык
- [ ] Pattern matching на списках `[x, ..xs]`
- [ ] `range(start, end)` функция
- [ ] Файловая система модулей (`use math: add`)

### Стандартная библиотека
- [ ] `Channel.recv_timeout(ms)` — возвращает `Option[T]`
- [ ] HTTP через `.rs` модули
- [ ] Базы данных через `.rs` модули

### Методы
- [ ] `list.filter(fn)`, `list.map(fn)`, `list.join(str)`
- [ ] `s.trim()`, `s.split(str)`, `s.contains(str)`
- [ ] `s.to_int()`, `s.to_float()`
- [ ] `n.to_string()`, `n.abs()`

---

## ❌ Убрано из языка

- [x] `Map[K, V]` — использовать `List[(K, V)]` или `.rs`
- [x] `?` оператор — только явный `match`
- [x] `pub` — всё публично в файле
- [x] `std.http`, `std.db` — заменено на `.rs` модули

---

## Принципы для ИИ

1. **Предсказуемость** — ИИ может «в уме» проследить выполнение
2. **Явность** — нет скрытого поведения (`?`, implicit conversions)
3. **Локальность ошибок** — ошибка на месте вызова
4. **Минимум токенов** — `end` вместо `{}`, нет `let`

---

*Обновлено: 2026-03-30*