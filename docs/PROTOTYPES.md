# U-lang Memory Model: Практические прототипы

## Проблема 1: Возврат из функции

### Вариант A: Копия (простой, O(n))

```u
fn first(list: List[T]) -> Option[T]
    if list.is_empty() return None
    return Some(list[0])    # копируем элемент
end

# Использование
data = ["hello", "world"]
first_opt = data.first()    # копирует String (аллокация!)
match first_opt
    Some(val) => print(val) # val — наша копия
    None => print("empty")
end
# data всё ещё жива
```

**AI понимает:**
- `first()` возвращает `Option[T]` — может быть None
- Элемент скопирован — можно менять, не влияет на оригинал
- Стоимость: O(size_of(T))

**Проблема:** Для String/List внутри List — копия дорогая

---

### Вариант B: Индекс (zero-cost, verbose)

```u
fn first_index(list: List[T]) -> Option[Int]
    if list.is_empty() return None
    return Some(0)          # просто число, zero-cost
end

# Использование
data = ["hello", "world"]
match data.first_index()
    Some(idx) => {
        val = data[idx]     # читаем по индексу (borrow)
        print(val)          # zero-cost!
    }
    None => print("empty")
end
```

**AI понимает:**
- `first_index()` возвращает `Option[Int]` — простое число
- Читаем через `data[idx]` — borrow, data жива
- Стоимость: O(1), нет копии

**Проблема:** Больше кода, два шага вместо одного

---

### Вариант C: Multiple return (компромисс)

```u
fn pop(list: List[T]) -> (List[T], Option[T])   # возвращаем пару
    if list.is_empty()
        return (list, None)     # list не изменился
    end
    val = list[list.len() - 1]  # копируем последний
    new_list = list.mut_pop()   # новый список без последнего
    return (new_list, Some(val))
end

# Использование
(data, popped) = data.pop()     # data перезаписывается
match popped
    Some(val) => print(val)
    None => print("was empty")
end
```

**AI понимает:**
- Функция возвращает кортеж `(new_list, element)`
- `data` переприсваивается — старая мертва, новая жива
- Стоимость: O(1) для List (внутри — move или realloc)

---

## Проблема 2: Вложенные структуры

### Вариант A: Вложенность (copy-on-write)

```u
struct Server {
    config: Config,      # вложенная структура
    port: Int
}

# Использование
cfg = Config.new(host: "localhost", port: 8080)
server = Server.new(config: cfg, port: 3000)
# cfg скопирована внутрь server!

# cfg всё ещё жива и независима
cfg.host = "other.com"   # не влияет на server
```

**AI понимает:**
- При создании Server — Config копируется (deep copy)
- Владелец Config и Server независимы
- Стоимость: O(size_of(Config))

**Проблема:** Для больших Config — дорого копировать

---

### Вариант B: Box (indirection, shared)

```u
struct Server {
    config: Box[Config],  # указатель на кучу
    port: Int
}

# Использование
cfg = Box.new(Config.new(...))   # Config в куче
server = Server.new(config: cfg, port: 3000)
# cfg — Box передаётся (move), не Config!

# cfg больше нельзя использовать — она внутри server
print(server.config.host)        # читаем через Box
```

**AI понимает:**
- `Box[T]` — owned pointer, владеет данными в куче
- Передача Box = move, не копия
- Стоимость: O(1) для передачи

**Проблема:** Box нужно dereference (`server.config` автоматически?)

---

### Вариант C: ID/Handle (no pointer, indirection)

```u
struct ConfigStore {
    configs: Map[Int, Config]   # все конфиги здесь
    next_id: Int
}

struct Server {
    config_id: Int,     # просто число, не указатель
    port: Int
}

# Использование
store = ConfigStore.new()
cfg_id = store.add(Config.new(...))  # получаем ID
server = Server.new(config_id: cfg_id, port: 3000)

# Читаем через store
config = store.get(server.config_id)  # возвращает Option[Config]
```

**AI понимает:**
- Нет ссылок вообще — только Int IDs
- Все данные в одном месте (ConfigStore)
- Стоимость: O(1) lookup по ID

**Проблема:** Нужно передавать store везде, где читаем config

---

## Проблема 3: Рекурсивные структуры

### Вариант A: Box (однозначное решение)

```u
struct Node {
    value: Int,
    left: Option[Box[Node]],   # Box для рекурсии
    right: Option[Box[Node]]
}

# Использование
root = Box.new(Node.new(
    value: 10,
    left: Some(Box.new(Node.new(value: 5, left: None, right: None))),
    right: None
))

# Читаем через Box
match root.left
    Some(boxed) => print(boxed.value)   # автоматический dereference?
    None => {}
end
```

**AI понимает:**
- `Box[T]` — единственный способ сделать рекурсию
- Размер структуры фиксирован (Box = один указатель)
- Стоимость: +1 аллокация на каждый Box

---

### Вариант B: Arena (всё в одной аллокации)

```u
arena = Arena.new()

# Создаём ноды в арене
root = arena.alloc(Node.new(value: 10))
left = arena.alloc(Node.new(value: 5))
root.left = Some(left)   # храним ссылку? или индекс?

# Всё освобождается разом
arena.free_all()         # root, left — всё мёртво
```

**AI понимает:**
- Arena = владелец всех нод
- Не нужно Box, но нужно передавать arena
- Стоимость: одна аллокация, bulk free

**Проблема:** Как хранить связи? Ссылки запрещены → только индексы?

---

## Сравнение для AI

| Проблема | Решение | Простота для AI | Стоимость | Выбор |
|----------|---------|-----------------|-----------|-------|
| Возврат | Копия | ✅ Просто | O(n) | Малые T |
| Возврат | Индекс | ⚠️ Два шага | O(1) | Большие T |
| Вложенность | Copy | ✅ Просто | O(n) | Малые структуры |
| Вложенность | Box | ⚠️ Indirection | O(1) + alloc | Большие структуры |
| Рекурсия | Box | ✅ Явно | O(1) per node | ✅ Стандарт |
| Рекурсия | Arena | ⚠️ Управление ареной | O(1) bulk | Опционально |

## Предложение для u-lang

1. **Small T (Int, Float, Bool, Char)** — копия по умолчанию
2. **Large T (String, List, Map)** — индексы или Box
3. **Рекурсия** — Box[T] обязательно
4. **Вложенность** — выбор: копия (просто) или Box (быстро)

```u
# Примеры по категориям:

# 1. Small T — копия
n = list.first()                    # Option[Int]
r = rect.top_left()                 # Point (struct с 2 Int)

# 2. Large T — индекс
idx = text_lines.first_index()      # Option[Int]
line = text_lines[idx]              # String (borrow)

# 3. Рекурсия — Box
node = Box.new(Node { value: 1, children: [] })

# 4. Вложенность — по выбору
# Простой:
server = Server { config: cfg, port: 80 }  # cfg скопирована
# Быстрый:
server = Server { config: Box.new(cfg), port: 80 }  # Box move
```

Такой подход фиксируем?
