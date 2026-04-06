# Примеры U-lang

## Being/Nothing (рекомендуется)
Философский подход к Option. Работает полностью.

```u
enum Maybe[T]
    Being(value: T)
    Nothing
end

fn divide(a: Int, b: Int) -> Maybe[Int]
    if b == 0
        return Nothing
    end
    return Being(value: a / b)
end

result = divide(10, 2)
match result
    Being(v) => print("Being: $v")
    Nothing => print("Nothing")
end
```

Файл: `maybe_being_nothing.u`

## Option Some/None (в разработке)
Стандартный подход. Типизатор дорабатывается.

Файл: `option_some_none.u`

## Другие примеры
- `basic_types.u` — базовые типы
- `list_types.u` — работа со списками
- `struct_types.u` — структуры
- `function_types.u` — функции
