# U-lang — Результаты работы 2026-03-31

## Реализованные методы

### List
| Метод | Описание | Пример |
|-------|----------|--------|
| `filter(fn)` | Отфильтровать элементы | `[1,2,3].filter(fn(x) x > 1)` |
| `map(fn)` | Преобразовать элементы | `[1,2,3].map(fn(x) x * 2)` |
| `join(str)` | Объединить в строку | `["a","b"].join(",")` |
| `sum()` | Сумма | `[1,2,3].sum()` → 6 |
| `first()` | Первый элемент | `[1,2,3].first()` → 1 |
| `last()` | Последний элемент | `[1,2,3].last()` → 3 |
| `sort()` | Сортировка | `[3,1,2].sort()` → [1,2,3] |
| `reverse()` | Реверс | `[1,2,3].reverse()` → [3,2,1] |

### String
| Метод | Описание | Пример |
|-------|----------|--------|
| `len()` | Длина | `"hello".len()` → 5 |
| `trim()` | Убрать пробелы | `"  x  ".trim()` → "x" |
| `split(str)` | Разбить по разделителю | `"a,b".split(",")` |
| `contains(str)` | Проверить подстроку | `"abc".contains("b")` |
| `starts_with(str)` | Начинается с | `"abc".starts_with("a")` |
| `ends_with(str)` | Заканчивается на | `"abc".ends_with("c")` |
| `replace(old, new)` | Замена | `"a,b".replace(",", "-")` |
| `to_upper()` | В верхний регистр | `"hi".to_upper()` → "HI" |
| `to_lower()` | В нижний регистр | `"HI".to_lower()` → "hi" |
| `to_int()` | В Int (Option) | `"42".to_int()` → Some(42) |
| `to_float()` | В Float (Option) | `"3.14".to_float()` → Some(3.14) |

### Number
| Метод | Описание | Пример |
|-------|----------|--------|
| `to_string()` | В строку | `42.to_string()` → "42" |
| `abs()` | Модуль | `(-5).abs()` → 5 |

## Option[T] — реализован

```u
num = "42".to_int()
match num
    Some(n) => print("OK: $n")
    None => print("Error")
end
```

## Обновленные файлы

- `/tmp/u-lang/transpiler/src/generator.rs` — генерация методов
- `/tmp/u-lang/transpiler/src/parser.rs` — поддержка `Some`/`None`
- `/tmp/u-lang/transpiler/src/u.pest` — грамматика для Option
- `/tmp/u-lang/runtime/src/lib.rs` — `str_to_int`, `str_to_float`
- `/tmp/u-lang/spec/SPEC.md` — документация методов

## Тесты

Все методы протестированы и работают:
- ✅ List: filter, map, join, sum, first, last, sort, reverse
- ✅ String: trim, split, contains, replace, to_upper, to_lower, to_int, to_float
- ✅ Number: to_string, abs
- ✅ Option: Some, None в match

---

## Дополнительно реализовано (2026-03-31 поздно)

### Новые методы
- `List.is_empty()` — проверка пустоты
- `List.append(item)` — добавление элемента

### Фичи
- **File-based modules**: `use math: add` → загрузка из `math.u`
- **List pattern matching**: `[x, ..xs]` — разбор списка
- **recv_timeout**: приём из канала с таймаутом

### Исправления
- Индексация `list[0]` работает с `String`
- Все предыдущие примеры продолжают работать