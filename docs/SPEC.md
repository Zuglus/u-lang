# U-lang Specification

## Архитектурные решения

### Конкурентность: Горутины со stack-only памятью

**Решение:** Использовать горутины с ограниченным стеком (максимум 500 KB).

**Обоснование:**

| Альтернатива | Проблема | Горутины |
|--------------|----------|----------|
| OS threads (1MB stack) | 10,000 × 1MB = 10GB RAM | 10,000 × 500KB = 5GB ✅ |
| Async/await | Сложно для ИИ (state machines) | Простой spawn(fn()) ✅ |
| Callbacks | Callback hell | Линейный код ✅ |
| Heap allocation | GC, фрагментация, сложность | Stack-only: предсказуемо ✅ |

**Ограничение:** Максимум **500 KB stack** на горутину. Без heap.

```
Stack: 500 KB максимум (растёт от 2 KB до 500 KB)
Heap:  Нет ❌
Total: ≤ 500 KB — runtime проверка (stack overflow)
```

**Почему stack-only:**
- Предсказуемое использование памяти
- Нет GC (не нужен)
- Нет фрагментации
- Автоматическое освобождение при return

**Модели памяти:**

| Размер | Модель | Когда |
|--------|--------|-------|
| ≤ 64 bytes | Copy | Всегда |
| 64 bytes — 500 KB | Move (stack) | Внутри горутины |
| > 500 KB | Невозможно | Compile-time error или runtime stack overflow |

**Большие данные:**

Если данные > 500 KB — их нельзя обработать в одной горутине.

```u
# ❌ Ошибка: data > 500 KB, не влезет в stack
fn process_big(data: HugeStruct)  # HugeStruct = 2 MB
    ...
end

# ✅ Решение: разбиваем на чанки ≤ 500 KB
fn process_chunk(chunk: Chunk)    # Chunk = 400 KB
    ...
end

# ✅ Решение: несколько горутин обрабатывают части
for part in split_big_data(big_data, chunk_size: 400000)
    spawn(fn() process_chunk(part))
end
```

### Channels

**API:**

```u
# Создание канала
ch = Channel.new()

# Отправка данных (move semantics)
ch.send(value)

# Получение данных (blocking)
result = ch.receive()
```

**Пример:**

```u
ch = Channel.new()

spawn(fn()
    data = compute()
    ch.send(data)
end)

result = ch.receive()
print("Result: $result")
```

**Генерация в Rust:**

```rust
// Channel.new()
let ch = { let (tx, rx) = tokio::sync::mpsc::channel(100); (tx, rx) };

// ch.send(data)
{ let (tx, _) = ch; tx.send(data).await.unwrap() }

// ch.receive()
{ let (_, rx) = ch; rx.recv().await.unwrap() }
```

**Важно:** При send/receive данные копируются между стеками (не shared memory).

### Being/Nothing

```u
enum Maybe[T]
    Being(value: T)
    Nothing(phantom: Phantom[T])
end
```

**Phantom[T]:** zero-sized тип, несёт информацию о T в compile-time.

**Пример:**
```u
fn divide(a: Int, b: Int) -> Maybe[Int]
    if b == 0
        return Nothing(phantom: Phantom[Int])  # Ничто от Int
    end
    return Being(value: a / b)
end
```

### Запрещено

- Heap allocation (нет `new`, `malloc`)
- Shared mutable state между горутинами
- Global variables
- Указатели между горутинами
- Данные > 500 KB в одной горутине

### Разрешено

- Stack allocation только
- Message passing через channels (copy между стеками)
- Immutable shared (read-only через copy)
- Spawn горутин с автоматическим клонированием captured variables

### Следующие шаги

- [ ] Реализовать compile-time проверку размера типов (≤ 500 KB)
- [ ] Протестировать channels с реальным runtime
- [ ] Добавить try_receive() → Maybe[T]
- [ ] Реализовать растущий стек (2 KB → 500 KB)
- [ ] Runtime проверка stack overflow

---

## Статус реализации

✅ **Готово:**
- Phantom[T] для zero-sized типов
- Being/Nothing с дженериками
- Spawn (горутины через tokio::spawn)
- Channel.new(), .send(), .receive()

🚧 **В работе:**
- Compile-time проверка 500 KB limit
- Тестирование с реальным runtime
- Растущий стек

📋 **Запланировано:**
- try_receive() → Maybe[T]
- Worker pools
- Bounded channels с backpressure
