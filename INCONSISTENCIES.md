# U-lang Specification Inconsistencies Report

## Critical Issues (Несоответствие реальности)

### 1. String.len() метод
**Спецификация:** `s.len()` ✅ (в 12.2)
**Реальность:** 
- Type checker считает что метод существует и возвращает `Int`
- Но код не выполняется - методы String не реализованы в runtime

```u
s = "hello"
print(s.len())  // Type checker: OK, Runtime: ???
```

**Вердикт:** 📋 Должен быть 📋 (Planned), не ✅

---

### 2. List[T] тип в полях структур
**Спецификация:** `List[T]` ✅ с выводом типа (в 2.2)
**Реальность:** 
```u
struct Data
    items: List[Int]  // ОШИБКА: тип поля = только identifier
end

struct Data
    items: List       // OK, но без параметра типа
end
```

**Вердикт:** Нужно указать что `List` без `[T]` в полях

---

### 3. Mutable переменные (`mut x = 10`)
**Спецификация:** `mut x = 10` ❓ TBD (в 3.1)
**Реальность:** Переменные вообще immutable, Wrapper pattern вместо этого
**AI_RULES:** Нет `mut` для локальных переменных

**Противоречие:** В 3.1 есть строка `mut x = 10` но в AI_RULES этого нет

**Вердикт:** Убрать `mut x = 10` из спецификации, оставить только Wrapper pattern

---

### 4. Option[T] и Result[T, E] как ✅
**Спецификация:** `Option[T]` и `Result[T, E]` ✅ (via enum) (в 2.2)
**Реальность:**
```u
// Это НЕ generic типы!
enum MyResult
    Val(value: Int)    // конкретный тип
    Fail(msg: String)
end

// Нельзя сделать:
// Result[Int, String] - нет generics!
```

**Вердикт:** Поменять на "implemented as concrete enums, not generic types"

---

### 5. Borrow operators (`&`, `&mut`)
**Спецификация:** `&x`, `&mut x` 📋 (в 1.3 и 10.2)
**Реальность:** В грамматике нет `&` как оператора, только в pest файле добавлены но не везде

```pest
prefix_op  = { "-" | not_kw }  // & не здесь!
```

**Вердикт:** ❓ TBD или убрать пока не реализовано

---

### 6. Wildcard pattern `_`
**Спецификация:** Wildcard `_ =>` 📋 (в 5.3)
**Реальность:** Неизвестно работает ли - не тестировалось

```u
match x
    1 => ...
    _ => ...  // ?
end
```

**Вердикт:** Нужно проверить и пометить соответственно

---

### 7. Method syntax inconsistency
**Спецификация:** `fn self.method() end` ✅ (в 4.2)
**Реальность:** 
```u
impl Rectangle
    fn area(self: Rectangle) -> Int  // self как параметр
        return self.width * self.height
    end
end
```

**Противоречие:** В спеке `fn self.method()` но в примерах `fn method(self: Type)`

**Вердикт:** Исправить спецификацию на реальный синтаксис

---

### 8. Lambda closures
**Спецификация:** `fn(x) x + 1` ✅ (в 4.3)
**Реальность:** Парсится, но неизвестно работает ли захват переменных

```u
x = 10
f = fn(y) x + y  // Захватит ли x?
```

**Вердикт:** Нужно проверить и уточнить статус

---

## Minor Issues (Неточности)

### 9. Enum unit variants
**Спецификация:** `enum Name UnitVariant end` ❓ TBD (в 6.3)
**Реальность:** В примерах только варианты с полями

```u
// Не используется:
enum Status
    Pending      // unit variant
    Processing
end

// Используется:
enum Status
    Pending(code: Int)   // всегда с полем
end
```

**Вердикт:** Указать что unit variants не поддерживаются

---

### 10. String interpolation in spec
**Спецификация:** `"value: $x"` ✅ (в 1.2)
**Недостаток:** Не указан синтаксис для выражений: `"$(x + y)"`

**Вердикт:** Добавить `$(expr)` в спецификацию

---

### 11. List methods all ❓ TBD
**Спецификация:** Все методы List ❓ TBD (в 12.3)
**Реальность:** `for i in list` работает (итерация)

**Вердикт:** Указать что итерация работает, методы - нет

---

## Recommendations

1. **Провести аудит** каждой ✅ фичи - реально ли работает
2. **Разделить статусы:**
   - ✅ Парсится + работает
   - 🔄 Парсится + неполная реализация
   - ❌ Парсится но не работает
3. **Уточнить синтаксис** method definitions
4. **Убрать** `mut x = 10` до реализации
5. **Добавить** раздел "Known Limitations"

---

## Quick Fix List

| Issue | Action | Priority |
|-------|--------|----------|
| String.len() | Change ✅ to 📋 | High |
| List[T] in fields | Clarify syntax | High |
| mut x = 10 | Remove from spec | High |
| Option/Result | Clarify "concrete enum" | Medium |
| & and &mut | Change to ❓ TBD | Medium |
| Method syntax | Fix to `fn name(self: Type)` | Medium |
| Wildcard `_` | Test and update status | Low |

---

## Summary

**Всего проблем:** 11  
**Критических:** 7  
**Minor:** 4

**Главное противоречие:** Спецификация опережает реализацию. Многое помечено как ✅ но либо не работает полностью, либо синтаксис в спеке отличается от реального.
