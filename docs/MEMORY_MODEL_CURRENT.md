# Memory Model — Current Implementation

**Дата:** 2026-04-05  
**Версия:** 0.2.0

---

## Обзор

U-lang использует **stack-only** модель памяти с явной семантикой Copy/Move.

```
┌─────────────────────────────────────────┐
│           Stack (500 KB max)            │
│  ┌─────────┐  ┌─────────┐  ┌────────┐  │
│  │ Copy    │  │ Move    │  │ Error  │  │
│  │ ≤64 B   │  │ 64-500KB│  │ >500KB│  │
│  │ (clone) │  │ (move)  │  │        │  │
│  └─────────┘  └─────────┘  └────────┘  │
└─────────────────────────────────────────┘
```

---

## Размеры типов

| Тип | Размер | Семантика |
|-----|--------|-----------|
| Int | 8 bytes | Copy |
| Float | 8 bytes | Copy |
| Bool | 1 byte | Copy |
| None | 0 bytes | Copy |
| Phantom[T] | 0 bytes | Copy |
| String | dynamic | Move |
| List[T] | dynamic | Move |
| Channel | ~24 bytes | Copy |
| Struct | varies | Copy/Move |

---

## Compile-Time Проверки

### 1. Проверка размера структур

```rust
// size_checker.rs
pub fn check_program_sizes(program: &Program) -> Result<(), String> {
    const STACK_LIMIT: usize = 500 * 1024;
    
    for stmt in &program.statements {
        if let Stmt::StructDef { name, fields, .. } = stmt {
            let total_size = calculate_struct_size(fields);
            
            if total_size > STACK_LIMIT {
                return Err(format!(
                    "ошибка: структура '{}' слишком большая \
                     ({} байт > 500 KB)",
                    name, total_size
                ));
            }
        }
    }
    Ok(())
}
```

### 2. Use-after-move проверка

```rust
// ownership.rs
pub enum VarState {
    Live,
    Moved { to: String, at_line: usize },
}

pub fn check_use(&self, 
    name: &str
) -> Result<(), String> {
    match self.get_state(name) {
        Some(VarState::Moved { to, .. }) => {
            Err("ошибка: использование перемещённой переменной")
        }
        _ => Ok(())
    }
}
```

---

## Кодогенерация

### Copy типы (Int, Float, Bool)

```u
# U-lang
x = 42
spawn(fn() process(x))
print(x)  # OK
```

```rust
// Сгенерированный Rust
let x = 42_i64;
{
    let x = x.clone();  // Copy
    tokio::spawn(async move {
        process(x);
    });
}
println!("{}", x);  // OK
```

### Move типы (String, Struct)

```u
# U-lang
msg = "Hello"
spawn(fn() process(msg))
# msg недоступен здесь
```

```rust
// Сгенерированный Rust
let msg = "Hello".to_string();
{
    let msg = msg;  // Move (без clone!)
    tokio::spawn(async move {
        process(msg);
    });
}
// msg недоступен — это правильно
```

---

## Примеры ошибок

### Структура > 500 KB

```u
struct TooBig
    # 65000 полей Int
    f0: Int, f1: Int, ...
end
```

**Ошибка:**
```
ошибка: структура 'TooBig' слишком большая (520000 байт > 500 KB лимит)
  = help: разбейте на части ≤ 500 KB или используйте каналы
```

### Use-after-move

```u
data = "Hello"
spawn(fn() process(data))
print(data)  # ❌
```

**Ошибка:**
```
ошибка: использование перемещённой переменной 'data'
  --> строка 3
  = перемещена в 'spawn' на строке 2
  = help: переменная недоступна после move
```

---

## Алгоритм Copy vs Move

```rust
fn should_copy(typ: &Type, ctx: &TypeCtx) -> bool {
    const COPY_THRESHOLD: usize = 64;
    
    match typ {
        // Примитивы всегда Copy
        Type::Int | Type::Float | Type::Bool | Type::None => true,
        
        // Структуры — по размеру
        Type::Struct(name) => {
            let size = calculate_struct_size(name, ctx);
            size.fixed_size.map_or(false, |s| s <= COPY_THRESHOLD)
        }
        
        // Остальное — Move
        _ => false,
    }
}
```

---

## Производительность

| Операция | Стоимость | Когда |
|----------|-----------|-------|
| Copy | O(n) | ≤64 bytes |
| Move | O(1) | >64 bytes (pointer swap) |
| Channel send | O(n) | copy между стеками |
| Spawn | O(1) | новая горутина |

---

## Ограничения

1. **Нет heap allocation** — только stack
2. **Нет shared mutable state** — только message passing
3. **Нет global variables** — только локальные
4. **Нет указателей между горутинами** — только channels

---

## Сравнение с другими языками

| Язык | Модель | GC | Shared mutable |
|------|--------|----|----------------|
| Go | Stack + Heap | ✅ | ✅ |
| Rust | Stack + Heap | ❌ | ❌ (borrow checker) |
| U-lang | Stack only | ❌ | ❌ (ownership) |
| Java | Heap only | ✅ | ✅ |

---

## Следующие шаги

- [ ] Дженерические каналы (Channel[T])
- [ ] Bounded channels с backpressure
- [ ] Try-send / Try-receive → Maybe[T]
- [ ] Растущий стек (2 KB → 500 KB)
