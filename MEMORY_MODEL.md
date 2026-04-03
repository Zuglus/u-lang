# U-lang Memory Model

## Принцип: Single Owner + Ban Stored References

Только одна переменная владеет данными в каждый момент. Ссылки (borrow) разрешены только для **immediate use** — одно выражение. Хранить ссылки в переменных, полях структур или возвращать из функций — **запрещено**.

---

## 6 Операций

### 1. Create — создаём, владеем
```u
x = Type.new()       # создали, x — владелец
y = [1, 2, 3]        # литерал, y — владелец
```

### 2. Read — borrow (&T), zero-cost
```u
n = x.len()          # borrow: читаем, x жив
print(x.first())     # borrow: читаем, x жив
use(x)               # функция смотрит, x жив
```
**Правило:** borrow живёт только внутри одного выражения. Нельзя сохранить в переменную.

### 3. Mutate — mutable borrow, изменяем
```u
x.mut_push(4)        # mutable borrow: меняем, x жив
x.mut_sort()         # mutable borrow: меняем, x жив
```
**Правило:** только владелец может вызывать `mut_` методы.

### 4. Move — transfer ownership
```u
y = x.mut_into_string()   # move: x умирает, y владеет результатом
process(mut x)            # move: x умирает внутри process
```
**Правило:** `mut_` метод, возвращающий другой тип = move.

### 5. Clone — deep copy
```u
y = x.clone()        # копия, обе переменные живы и независимы
process(mut x.clone())    # копия move'ится, x жив
```
**Стоимость:** O(n), явно видно в коде.

### 6. Delete — implicit drop
```u
# x автоматически освобождается при выходе из scope
# Нет ручного free/delete
```

---

## Ключевое правило

> **Ссылки только для immediate use (одно выражение). Хранить ссылки нельзя.**

```u
# ✅ Разрешено (immediate use):
n = data.len()                    # borrow внутри выражения
process(data.first())             # borrow внутри выражения
for item in data { use(item) }    # borrow внутри цикла

# ❌ Запрещено (stored reference):
ref = data.first()                # сохраняем ссылку — ОШИБКА
return data.first()               # возвращаем ссылку — ОШИБКА
struct Wrapper { ref: &T }        # поле-ссылка — ОШИБКА
```

---

## Разрешённые противоречия

| Противоречие | Решение |
|--------------|---------|
| Read vs Move | Разные операции: read = borrow, `into_` = move |
| Single owner vs множественное чтение | Ссылки только временные, не хранятся |
| Итерация vs запрет ссылок | For = index-based доступ, не сохраняет ссылки |

---

## Нерешённые проблемы

### 1. Возврат из функции
```u
fn first(list: List[T]) -> ???    # что возвращаем?
# Варианты:
# - Копия (дорого для больших T)
# - Ссылка (нарушает правило immediate use)
# - Move (портит list, нельзя читать дальше)
# - Option/Result с копией (не zero-cost)
```

### 2. Вложенные структуры
```u
struct Server {
    config: Config          # как хранит?
}
# Варианты:
# - Вложенность (move Server = move Config, тяжело)
# - Ссылка (запрещено правилом)
# - Копия (дорого)
# - Box/Indirection (дополнительная аллокация)
```

### 3. Рекурсивные структуры
```u
struct Node {
    value: Int,
    next: Node?             # без ссылок невозможно!
}
# Требуется Box[T] или аналог: owned pointer
```

---

## Для ИИ: 3 правила

1. **Владеешь переменной** → можешь звать любые методы, она жива
2. **Видишь `mut_` в вызове** → проверь: возвращает другой тип? Если да — переменная умирает
3. **Нужно сохранить результат** → используй clone или передавай ownership

```u
# Примеры для AI:
data = [1, 2, 3]

n = data.len()                    # ✅ borrow, data жива
print(data)                       # ✅ можно использовать

data.mut_push(4)                  # ✅ mutate, data жива, изменена
print(data)                       # ✅ можно использовать

text = data.mut_into_string()     # ✅ move, data мертва, text жив
print(data)                       # ❌ ОШИБКА: data moved
```

---

## Что исследовали

- **Rust**: borrow checker — сложно для AI, lifetimes неинтуитивны
- **Swift**: COW (Copy-on-Write) — не zero-cost, непредсказуемая производительность
- **Nim**: ARC (Automatic Reference Counting) — не zero-cost, циклические ссылки
- **Zig**: explicit manual memory — небезопасно, сложно для AI
- **Odin**: value/reference разделение — promising, требует изучения

## Следующие шаги

1. [ ] Исследовать Pony/Christian languages (capabilities)
2. [ ] Проверить Odin на практических тестах
3. [ ] Прототипировать решение для возврата из функции
4. [ ] Решить рекурсивные структуры (Box[T])
5. [ ] Обновить SPECIFICATION.md с финальной моделью
