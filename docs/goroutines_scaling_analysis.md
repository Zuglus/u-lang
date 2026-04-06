# Масштабирование горутин: от 1 до 100M

## Параметры
- RAM сервера: типично 64 GB
- Stack на горутину: минимум ~2 KB
- CPU cores: 16-64

## Сценарии

### 1 горутина (1)
**Модель:** Неважна — всё работает
```u
main()
    data = load_big_file()
    process(data)   # всё в одном потоке
    save(data)
end
```
**Память:** ~10 MB
**Синхронизация:** не нужна
**Лучший выбор:** Copy или Move — без разницы

---

### 10 горутин (10¹)
**Модель:** Worker pool, небольшой parallelism
```u
for i in 0..10
    spawn(fn() work(i))
end
```
**Память:** ~10 × 2 KB = 20 KB stacks + heap
**Синхронизация:** минимальная
**Лучший выбор:** Message passing или shared immutable

**Проблемы:** нет

---

### 100 горутин (10²)
**Модель:** Сервер с concurrency
```u
server = new_server()
for client in connections
    spawn(fn() handle(client))
end
```
**Память:** ~100 × 2 KB = 200 KB stacks
**Синхронизация:** начинается contention

**Варианты:**
- Copy small data: OK
- Shared immutable config: OK
- Mutable shared: начинаются проблемы

**Лучший выбор:** 
- Config: `@immutable`
- State: message passing

---

### 1,000 горутин (10³)
**Модель:** High-concurrency сервер
```u
for i in 0..1000
    spawn(fn() worker(i))
end
```
**Память:** ~1,000 × 2 KB = 2 MB stacks
**Контекст switches:** частые

**Проблемы:**
- Mutex contention становится заметным
- Cache coherency: false sharing

**Бенчмарки:**
- Mutex: ~100 нс без contention → ~1-10 мкс с contention
- Channel send: ~50-100 нс
- Arc increment: ~20-100 нс

**Лучший выбор:**
- Message passing (channels)
- Immutable shared (read-only)
- Avoid: mutable shared state

---

### 10,000 горутин (10⁴)
**Модель:** Микросервис, data pipeline
```u
# 10K workers processing stream
for i in 0..10000
    spawn(fn() process_chunk(i))
end
```
**Память:** ~10,000 × 2 KB = 20 MB stacks
**Контекст switches:** очень частые

**Критический момент:**
- OS threads (1MB stack каждый): 10 GB — слишком много
- Goroutines (2KB stack каждая): 20 MB — ок

**Проблемы:**
- **Cache thrashing:** каждый switch сбрасывает кэш
- **Lock contention:** 10K горутин ждут одного mutex
- **Memory allocator contention:** 10K потоков аллоцируют

**Benchmark (estimates):**
```
Mutex lock/unlock:
- 1 горутина: ~20 нс
- 10 горутин: ~50 нс
- 100 горутин: ~200 нс
- 1,000 горутин: ~1-2 мкс
- 10,000 горутин: ~10-100 мкс (degrades badly)
```

**Лучший выбор:**
- ✅ Message passing (нет shared state)
- ✅ Immutable shared (RCU, no atomics для чтения)
- ✅ Arena allocation (bulk free)
- ❌ Mutable shared + locks: деградация

---

### 100,000 горутин (10⁵)
**Модель:** Massive concurrency (IoT, WebSocket сервер)
```u
# 100K одновременных соединений
for conn in connections
    spawn(fn() handle_ws(conn))
end
```
**Память:** ~100,000 × 2 KB = 200 MB stacks
**Scheduler overhead:** высокий

**Проблемы:**
- 100K стеков — много памяти
- Context switch overhead
- OS scheduler struggles

**Оптимизации:**
- Stackless coroutines (async/await)
- Work-stealing scheduler
- NUMA-aware allocation

**Лучший выбор:**
- Message passing (zero-copy где возможно)
- Immutable data (no RC at all)
- Batch processing

**Что ломается:**
- ❌ RefCell runtime checks: ~100 нс × 100K = 10 секунд!
- ❌ Mutex: полная деградация
- ❌ GC: stop-the-world паузы в минуты

---

### 1,000,000 горутин (10⁶)
**Модель:** Massive scale (simulation, game server)
```u
# 1M entities
for i in 0..1000000
    spawn(fn() entity_loop(i))
end
```
**Память:** ~1,000,000 × 2 KB = 2 GB stacks
**Требования:** специальный runtime

**Ограничения:**
- RAM: 2 GB только на стеки
- Scheduler: custom required
- Memory allocator: scalable (jemalloc/tcmalloc)

**Лучший выбор:**
- Actor model (Erlang-style)
- Immutable everything
- No shared state вообще
- Batch message processing

**Архитектура:**
- Несколько scheduler threads
- Work-stealing между ними
- Message queues per actor

---

### 10,000,000 горутин (10⁷)
**Модель:** Extreme scale (SIMD в ширину)
**Память:** ~20 GB стеков
**Требования:** distributed system

**Реальность:** 
- Одна машина не справится
- Нужен cluster
- Distributed actor system (Akka, Orleans)

**Модель:**
- Actor per «grain»
- Location transparency
- Distributed GC или ARC

---

### 100M+ горутин (10⁸)
**Память:** ~200 GB
**Архитектура:** Distributed cluster

Это уже не «горутины» в привычном смысле, а:
- Virtual actors (Microsoft Orleans)
- Distributed entities
- Database-backed state

---

## Выводы для U-lang

| Горутины | Модель | Синхронизация | Память |
|----------|--------|---------------|--------|
| 1-10 | Any | None | ~MB |
| 10-100 | Message passing | Channels | ~MB |
| 100-1K | Message passing | Channels | ~MB |
| 1K-10K | Message passing | Zero-copy | ~10 MB |
| 10K-100K | Immutable + channels | NUMA-aware | ~100 MB |
| 100K-1M | Actor model | Custom scheduler | ~GB |
| 1M+ | Distributed | Cluster | Distributed |

## Критический вывод

**Для 10,000+ горутин:**
- ❌ Runtime borrow checks — слишком дорого
- ❌ Mutex/locks — contention
- ❌ GC — stop-the-world
- ✅ Immutable data — zero-cost чтение
- ✅ Message passing — нет shared state
- ✅ Arena allocation — bulk free

**Для 100,000+ горутин:**
Требуется полная изоляция — каждая горутина как «micro-service»
