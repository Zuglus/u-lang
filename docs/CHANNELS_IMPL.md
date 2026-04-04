# Channels Implementation

**Дата:** 2026-04-05  
**Статус:** ✅ Реализовано

---

## Архитектура

### Runtime

```rust
pub mod async_int_channel {
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
    
    pub struct AsyncIntChannel;
    
    impl AsyncIntChannel {
        pub fn new(&self) -> AsyncIntChan {
            let (tx, rx) = unbounded_channel::<i64>();
            AsyncIntChan { 
                sender: tx, 
                receiver: Arc::new(Mutex::new(rx)) 
            }
        }
    }
    
    #[derive(Clone)]
    pub struct AsyncIntChan {
        sender: UnboundedSender<i64>,
        receiver: Arc<Mutex<UnboundedReceiver<i64>>>,
    }
    
    impl AsyncIntChan {
        pub fn send(&self, val: i64) {
            let _ = self.sender.send(val);
        }
        
        pub async fn recv(&self) -> i64 {
            self.receiver.lock().await.recv().await.unwrap_or(0)
        }
    }
}
```

### Генерация кода

```rust
// channel_new() -> u_runtime::async_int_channel::AsyncIntChannel.new()
if name == "channel_new" {
    out.push_str("u_runtime::async_int_channel::AsyncIntChannel.new()");
    return;
}

// ch.send(val) -> ch.send(val)
if method == "send" && args.len() == 1 {
    gen_expr(object, out, ctx);
    out.push_str(".send(");
    gen_expr(&args[0], out, ctx);
    out.push_str(")");
    return;
}

// ch.receive() -> ch.recv().await
if method == "receive" && args.is_empty() {
    gen_expr(object, out, ctx);
    out.push_str(".recv().await");
    return;
}
```

### Spawn с Move семантикой

```rust
Stmt::Spawn { expr, .. } => {
    let spawn_body = match expr {
        Expr::Lambda { body, .. } => body.as_ref(),
        _ => expr,
    };
    
    let mut vars = collect_free_vars(spawn_body);
    vars.sort(); vars.dedup();
    
    out.push_str("{\n");
    for v in &vars {
        out.push_str(&pad); out.push_str("    let ");
        out.push_str(v); out.push_str(" = ");
        
        // Copy или Move?
        let var_type = ctx.var_types.borrow().get(v).cloned().unwrap_or_default();
        if is_copy_type(&var_type) {
            // Int/Float/Bool — клонируем
            out.push_str(v); out.push_str(".clone();\n");
        } else {
            // String/struct — перемещаем
            out.push_str(v); out.push_str(";\n");
        }
    }
    out.push_str(&pad); out.push_str("    tokio::spawn(async move {\n");
    // ...
}
```

---

## Типизация

### Type Checker

```rust
// Встроенная структура Channel
let channel_fields = HashMap::new();
ctx.structs.insert("Channel".to_string(), channel_fields);

// Методы Channel
let mut channel_methods = HashMap::new();
channel_methods.insert(
    "send".to_string(), 
    (vec![Type::Int], Type::None)
);
channel_methods.insert(
    "receive".to_string(), 
    (vec![], Type::Int)
);
ctx.methods.insert("Channel".to_string(), channel_methods);

// Функция channel_new()
ctx.functions.insert(
    "channel_new".to_string(),
    (vec![], Type::Struct("Channel".to_string())),
);
```

### Маппинг типов

```rust
fn map_type(t: &str) -> String {
    match t {
        "Int" => "i64".to_string(),
        "Channel" => "u_runtime::async_int_channel::AsyncIntChan".to_string(),
        // ...
    }
}
```

---

## Пример использования

### U-lang код

```u
fn sender(ch: Channel)
    ch.send(42)
end

ch = channel_new()
spawn(fn() sender(ch))
result = ch.receive()
print("Got: $result")
```

### Сгенерированный Rust

```rust
fn sender(ch: u_runtime::async_int_channel::AsyncIntChan) {
    ch.send(42_i64);
}

#[tokio::main]
async fn main() {
    let ch = u_runtime::async_int_channel::AsyncIntChannel.new();
    
    {
        let ch = ch;  // Move (не clone!)
        tokio::spawn(async move {
            sender(ch);
        });
    }
    
    let result = ch.recv().await;
    println!("Got: {}", result);
}
```

---

## Проверка Ownership

```rust
// Проверка use-after-move
fn check_use(&self, 
    name: &str, 
    line: usize
) -> Result<(), String> {
    match self.get_state(name) {
        Some(VarState::Moved { to, at_line }) => {
            Err(format!(
                "ошибка: использование перемещённой переменной '{}'\n  \
                 = перемещена в '{}' на строке {}",
                name, to, at_line
            ))
        }
        Some(VarState::Live) => Ok(()),
        // ...
    }
}
```

---

## Ограничения

1. **Тип канала:** Пока только `Int` (не дженерики)
2. **Размер буфера:** Unbounded (может расти)
3. **Таймауты:** Нет (только blocking receive)

---

## Тесты

```u
// Базовый тест
fn sender(ch: Channel)
    ch.send(42)
end

ch = channel_new()
spawn(fn() sender(ch))
result = ch.receive()  # 42

// Множественные отправки
ch = channel_new()
for i in range(0, 10)
    spawn(fn() ch.send(i))
end

// Pipeline
ch1 = channel_new()
ch2 = channel_new()

spawn(fn()
    ch1.send(10)
end)

spawn(fn()
    val = ch1.receive()
    ch2.send(val * 2)
end)

result = ch2.receive()  # 20
```
