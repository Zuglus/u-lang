# U-lang Memory Model

## Решение: Move Semantics с явным `mut`

### Две категории типов

| Категория | Типы | Размер | Передача |
|-----------|------|--------|----------|
| **Small** (Copy) | `Int`, `Float`, `Bool` | 1-8 байт | Копируется |
| **Large** (Move) | `String`, `List`, `Struct`, `Enum` | Неизвестен | Перемещается (move) |

### Правило `mut`

| Вызов | Семантика | Стоимость | Reuse |
|-------|-----------|-----------|-------|
| `fn(x: T)` | Readonly | Zero-cost (указатель) | ✅ Сколько угодно |
| `fn(mut x: T)` | Move ownership | Zero-cost | ❌ Переменная мертва |

### Когда `mut` НЕ нужен (95% кода)

```u
// Чтение immutable данных
config = load_config()
port = config.get("port")           // читаем
server = start_server(config)       // читаем
// config доступна дальше — никаких копий!

// Input данные
data = read_file("input.txt")
result = parse(data)                // читаем
// data доступна — просто указатель!
```

**Config, settings, input — только читаем, `mut` не нужен!**

### Когда `mut` нужен (5% кода)

```u
// 1. Строим/меняем данные (accumulator)
result = []
result = push(mut result, item1)    // строим список
result = push(mut result, item2)

// 2. Consuming transform (старое → новое)
raw = read_bytes("data.bin")
text = decode(mut raw)              // забираем raw, даём text
// raw мертв, text — новая переменная
```

### Примеры

#### Readonly — сколько угодно раз
```u
data = [1, 2, 3]

len(data)           // ✅
first(data)         // ✅
for x in data       // ✅
print(data)         // ✅
// data всё ещё тут!
```

#### Move — один раз
```u
data = [1, 2, 3]
result = into_sorted(mut data)      // забираем data
// ❌ data мертва!
print(result)                       // ✅ result наша
```

#### Явная копия когда нужно reuse
```u
config = load_config()
server = start_server(mut config.clone())  // копия ушла
db = connect_db(config)                    // оригинал тут
```

### Для ИИ

**Простое правило:**
1. Пишешь функцию — добавляй `mut` если она "забирает" данные
2. Вызываешь функцию — добавляй `mut` если готов "отдать" переменную
3. Большинство функций — readonly, `mut` не нужен

**IDE/компилятор подскажет:**
```
process(data)           // "Добавь 'mut' если функция изменяет"
process(mut data)       // ✅ "Переменная будет перемещена"
```

### Почему это работает

| Сценарий | Частота | Move подходит? |
|----------|---------|----------------|
| Config/settings | Часто | ✅ Не меняется — просто указатель |
| Input processing | Часто | ✅ Один проход — move экономит |
| Accumulator/builder | Редко | ✅ Нужен mut, но это ожидаемо |
| Multi-use state | Редко | ⚠️ Нужен .clone() или redesign |

**Вывод:** `mut` только для consuming/building, остальное — readonly (zero-cost).

---

## Открытые вопросы

1. **Box тип** — нужен для рекурсивных структур (AST)
2. **Arena** — оптимизация для компиляторов (batch allocation)
3. **Clone метод** — явное копирование когда нужно reuse

### Следующие шаги

- [ ] Добавить `mut` в синтаксис
- [ ] Реализовать move semantics в генераторе
- [ ] Добавить `Box[T]` тип
- [ ] Добавить `.clone()` метод
