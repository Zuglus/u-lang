# U-lang Channels — Реализация

## Статус
✅ Добавлена поддержка Channel в type_checker.rs
✅ Добавлена генерация кода в generator.rs
✅ Создан тестовый пример channel_test.u

## API

```u
# Создание канала
ch = Channel.new()  # -> Channel[T] (возвращает (Sender<T>, Receiver<T>))

# Отправка данных
ch.send(value)      # -> None

# Получение данных  
result = ch.receive()  # -> T (blocking)
```

## Генерация в Rust

```rust
// Channel.new()
let ch = { let (tx, rx) = tokio::sync::mpsc::channel(100); (tx, rx) };

// ch.send(42)
{ let (tx, _) = ch; tx.send(42).await.unwrap() }

// ch.receive()
{ let (_, rx) = ch; rx.recv().await.unwrap() }
```

## Ограничение стека 500 KB

Устанавливается в runtime через Builder:

```rust
tokio::runtime::Builder::new_multi_thread()
    .thread_stack_size(512 * 1024)  // 500 KB
    .worker_threads(10000)          // до 10K горутин
    .build()
    .unwrap()
```

## Тестовый пример

```u
ch = Channel.new()

spawn(fn()
    ch.send(42)
end)

result = ch.receive()
print("Received: $result")  # Received: 42
```

## Следующие шаги

1. [ ] Исправить проблему с clone() для ch в spawn
2. [ ] Добавить compile-time проверку размера типов (≤ 500 KB)
3. [ ] Протестировать с реальным runtime
4. [ ] Добавить try_receive() -> Maybe[T]
