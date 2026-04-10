# U-lang Core Requirements (упрощённый)

## Без Map — списки с линейным поиском

### Символьная таблица через список

```u
// Вместо Map[String, Type]
struct Symbol
    name: String
    type: Type
end

// Линейный поиск — для небольших таблиц быстрее Map
fn lookup(symbols: List[Symbol], name: String) -> Option[Type]
    for sym in symbols
        if sym.name == name
            return Some(type: sym.type)
        end
    end
    return None
end

fn insert(symbols: List[Symbol], name: String, type: Type) -> List[Symbol]
    return symbols + [Symbol(name: name, type: type)]
end
```

---

## Минимальный набор для компилятора

### 1. String методы (нужны для Lexer)

```u
// Обязательно
s.char_at(i)           // символ по индексу
s.substring(start, end) // подстрока [start, end)
s.len()                // длина строки

// Желательно
s.starts_with(prefix)  // проверка префикса
s.split(sep)           // разбивка по разделителю
```

### 2. Char операции

```u
is_digit(c)    // '0'..'9'
is_alpha(c)    // 'a'..'z', 'A'..'Z'
is_whitespace(c)  // ' ', '\t', '\n'
```

### 3. File I/O

```u
content = read_file("input.u")   // String или Error
write_file("output.rs", code)    // Bool или Error
```

### 4. Рекурсивный enum (нужен для AST)

```u
enum Expr
    Number(value: Int)
    Binary(left: Expr, op: String, right: Expr)  // рекурсия!
    Call(name: String, args: List[Expr])
end
```

### 5. String concatenation

```u
// Уже есть через интерполяцию
result = "prefix" + value + "suffix"
```

---

## Чего НЕ нужно для компилятора

| Не нужно | Почему |
|----------|--------|
| Map | Список пар + линейный поиск |
| Hash | Не нужен для маленьких таблиц |
| Set | Список + проверка на contains |
| Float | Int достаточно для индексов |
| Generics | Явные типы проще |

---

## Порядок реализации

1. **String: char_at, substring, len**
2. **File: read_file, write_file**  
3. **Рекурсивный enum**
4. **Lexer на U-lang** (тест: токенизировать examples/hello.u)
5. **Parser на U-lang** (тест: распарсить в AST)
6. **Type checker** (списки вместо Map)
7. **Codegen** (генерация Rust)

---

## Размер символьных таблиц

- Переменные в функции: 10-20 шт.
- Функции в модуле: 10-50 шт.
- Поля структуры: 2-10 шт.

**Линейный поиск O(n) для n=20** — быстрее, чем хеш O(1) с накладными расходами.

---

## Итог

**Core для компилятора:**
- String методы (char_at, substring, len)
- File I/O (read_file, write_file)
- Рекурсивный enum
- Всё остальное — списками

Готов начать с `char_at` и `substring` для String? 🔥
