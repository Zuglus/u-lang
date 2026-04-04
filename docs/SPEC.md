# U-lang Specification

**Версия:** 0.2.0  
**Дата:** 2026-04-05

---

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
Total: ≤ 500 KB — compile-time проверка
```

**Почему stack-only:**
- Предсказуемое использование памяти
- Нет GC (не нужен)
- Нет фрагментации
- Автоматическое освобождение при return

---

## Модель памяти

### Copy vs Move семантика

| Тип | Размер | Семантика | Примеры |
|-----|--------|-----------|---------|
| Примитивы | ≤ 64 bytes | **Copy** | Int, Float, Bool |
| Структуры | ≤ 64 bytes | **Copy** | Малые struct |
| Структуры | 64-500 KB | **Move** | Большие struct |
| String, List | Dynamic | **Move** | heap-allocated |
| > 500 KB | — | **Error** | Compile-time ошибка |

### Copy типы

```u
# Эти типы копируются (не муваются)
x = 42
spawn(fn() process(x))  # x скопирован
print(x)                # ✅ x доступен — Int копируется
```

### Move типы

```u
# Эти типы перемещаются
msg = "Hello"
spawn(fn() process(msg))  # msg перемещён
print(msg)                # ❌ Ошибка: использование перемещённой переменной
```

### Compile-time проверка размеров

```u
# ❌ Ошибка компиляции: структура слишком большая
struct TooBig
    # 65000 полей Int = 520000 bytes > 500 KB
    f0: Int, f1: Int, f2: Int, ...
end

# Результат:
# ошибка: структура 'TooBig' слишком большая (520000 байт > 500 KB лимит)
#   = help: разбейте на части ≤ 500 KB или используйте каналы
```

---

## Ownership и Borrowing

### Правила владения

1. **Владение:** Каждое значение имеет одного владельца
2. **Move:** При передаче в spawn или функцию значение перемещается
3. **Copy:** Малые примитивы копируются вместо move
4. **Use-after-move:** Compile-time ошибка

### Примеры ошибок

```u
fn process(data: String)
    print(data)
end

data = "Hello"
spawn(fn() process(data))  # data перемещён
print(data)                # ❌ Ошибка компиляции

# ошибка: использование перемещённой переменной 'data'
#   --> строка 7
#   = перемещена в 'spawn' на строке 6
#   = help: переменная недоступна после move
```

### Корректное использование

```u
# Для Copy типов — используем как хотим
x = 42
spawn(fn() calc(x))
y = x + 1       # ✅ OK: Int копируется

# Для Move типов — передаём ownership
ch = channel_new()
spawn(fn() sender(ch))  # ch перемещён
# ch недоступен здесь — это правильно
```

---

## Channels

### API

```u
# Создание канала
ch = channel_new()

# Отправка данных (move semantics)
ch.send(value)

# Получение данных (blocking)
result = ch.receive()
```

### Пример

```u
fn worker(id: Int, ch: Channel)
    ch.send(id * 2)
end

ch = channel_new()

spawn(fn() worker(1, ch))
spawn(fn() worker(2, ch))

result1 = ch.receive()  # 2
result2 = ch.receive()  # 4
```

### Генерация в Rust

```rust
// Создание
let ch = u_runtime::async_int_channel::AsyncIntChannel.new();

// Отправка (async)
ch.send(42);

// Получение (async)
let result = ch.recv().await;
```

**Важно:** Данные копируются между стеками горутин (не shared memory).

---

## Being/Nothing

### Определение

```u
enum Maybe[T]
    Being(value: T)
    Nothing(phantom: Phantom[T])
end
```

### Phantom[T]

Zero-sized тип — несёт информацию о T в compile-time, runtime размер = 0.

```u
fn divide(a: Int, b: Int) -> Maybe[Int]
    if b == 0
        return Nothing(phantom: Phantom[Int])
    end
    return Being(value: a / b)
end

result = divide(10, 2)
match result
    Being(value) => print("Result: $value")
    Nothing(_)    => print("Division by zero")
end
```

---

## Ограничения

### Запрещено

- ❌ Heap allocation (нет `new`, `malloc`)
- ❌ Shared mutable state между горутинами
- ❌ Global variables
- ❌ Указатели между горутинами
- ❌ Данные > 500 KB в одной горутине
- ❌ Use-after-move (compile-time ошибка)

### Разрешено

- ✅ Stack allocation только
- ✅ Message passing через channels
- ✅ Immutable shared (read-only)
- ✅ Copy для малых типов (≤ 64 bytes)
- ✅ Move для больших типов (> 64 bytes)

---

## Работа с большими данными

### Chunking (разбиение на части)

```u
# Если данные > 500 KB — разбиваем на чанки
fn process_big_dataset()
    # Данные разбиты на части ≤ 500 KB
    chunks = load_chunks("huge_file.bin", max_size: 400000)
    
    for chunk in chunks
        spawn(fn() process_chunk(chunk))
    end
end
```

### Pipeline

```u
fn pipeline()
    ch1 = channel_new()
    ch2 = channel_new()
    
    # Stage 1: чтение
    spawn(fn() 
        for record in read_data()
            ch1.send(record)
        end
    end)
    
    # Stage 2: обработка
    spawn(fn()
        for record in ch1.receive_iter()
            processed = transform(record)
            ch2.send(processed)
        end
    end)
    
    # Stage 3: вывод
    for result in ch2.receive_iter()
        print(result)
    end
end
```

---

## Статус реализации

✅ **Готово:**
- Phantom[T] для zero-sized типов
- Being/Nothing с дженериками
- Spawn (горутины через tokio::spawn)
- Channel.new(), .send(), .receive()
- Compile-time проверка 500 KB limit
- Use-after-move detection
- Copy/Move семантика в кодогенерации

🚧 **В работе:**
- try_receive() → Maybe[T]
- select (множественный receive)
- Растущий стек (2 KB → 500 KB)

📋 **Запланировано:**
- Worker pools
- Bounded channels с backpressure
- Таймауты для операций
