# U-lang Runtime Implementation Status

## Что РЕАЛЬНО работает в runtime (проверено)

### ✅ String Methods (реализованы в generator.rs)

| Method | Status | Notes |
|--------|--------|-------|
| `s.len()` | ✅ | Работает! Возвращает Int |
| `s.find(sub)` | ✅ | Возвращает позицию или -1 |
| `s.find_from(sub, from)` | ✅ | Поиск с позиции |
| `s.slice(start, end)` | ✅ | Подстрока [start, end) |
| `s.slice_from(start)` | ✅ | Подстрока от позиции |
| `s.split(delim)` | ✅ | Возвращает List[String] |
| `s.split_lines()` | ✅ | По \n |
| `s.first()` | ✅ | Первый элемент или 0 для чисел |
| `s.last()` | ✅ | Последний элемент или 0 |

### 📋 String Methods (в runtime, но не подключены к generator)

| Method | Runtime | Generator | Status |
|--------|---------|-----------|--------|
| `s.trim()` | ✅ | ❌ | 📋 Нужно добавить в generator |
| `s.contains(sub)` | ✅ | ❌ | 📋 Нужно добавить |
| `s.starts_with(prefix)` | ✅ | ❌ | 📋 Нужно добавить |
| `s.ends_with(suffix)` | ✅ | ❌ | 📋 Нужно добавить |
| `s.replace(old, new)` | ✅ | ❌ | 📋 Нужно добавить |

### ❌ String Methods (не реализованы)

| Method | Status | Обходной путь |
|--------|--------|---------------|
| `s.char_at(i)` | ❌ | `s.slice(i, i+1)` |
| `s.substring(start, len)` | ❌ | `s.slice(start, start+len)` |
| `s.to_upper()` | ❌ | — |
| `s.to_lower()` | ❌ | — |
| `s.parse_int()` | ✅ | `str_to_int(s)` (функция) |
| `s.parse_float()` | ✅ | `str_to_float(s)` (функция) |

---

## ✅ File I/O (реализовано!)

```u
content = read_file("input.txt")    // ✅ Работает!
write_file("output.txt", data)      // ✅ Работает!
create_dir("mydir")                 // ✅ Работает!
list_dir(".")                       // ✅ Работает!
```

**Важно:** Эти функции НЕ методы, а свободные функции из runtime.

---

## ✅ List Methods (частично реализовано)

| Method | Status | Notes |
|--------|--------|-------|
| `lst.len()` | ✅ | Работает! |
| `lst.first()` | ✅ | Работает! |
| `lst.last()` | ✅ | Работает! |
| `lst[index]` | ✅ | Индексация! |
| `lst.filter(fn)` | ❌ | Не реализовано |
| `lst.map(fn)` | ❌ | Не реализовано |
| `lst.push(val)` | ❌ | Не реализовано |
| `lst.pop()` | ❌ | Не реализовано |
| `for x in lst` | ✅ | Итерация работает! |

---

## Итог: Противоречия в спецификации

### Что помечено ❌ но работает:
1. `read_file`, `write_file` — ✅ работают!
2. `s.len()` — ✅ работает!
3. `s.slice()` — ✅ работает! (это и есть substring)
4. `s.find()` — ✅ работает!
5. `s.split()` — ✅ работает!

### Что помечено ✅ но не полностью:
1. `char_at` — не реализован, но `slice(i, i+1)` работает

### Что нужно добавить в generator:
1. `trim`, `contains`, `starts_with`, `ends_with`, `replace`

---

## Рекомендации

1. **Обновить спецификацию**: многое уже работает!
2. **Добавить в generator**: оставшиеся string методы (5 штук)
3. **Документировать**: `slice` вместо `substring`

---

## Тестовый файл для проверки

```u
// ЭТО ВСЁ РАБОТАЕТ:

// String
s = "hello world"
print(s.len())              // 11
print(s.find("world"))      // 6
print(s.slice(0, 5))        // "hello"
print(s.split(" "))         // ["hello", "world"]

// List
lst = [1, 2, 3]
print(lst.len())            // 3
print(lst.first())          // 1
print(lst[1])               // 2

// File
content = read_file("hello.u")
write_file("output.txt", content)
```
