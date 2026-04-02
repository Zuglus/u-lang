# Self-hosting: что нужно для компилятора на U-lang

## Минимальный набор (MVP)

### 1. Работа с файлами
```u
// Нужно добавить в std
text = read_file("input.u")      // чтение
write_file("output.rs", code)    // запись
```

### 2. Строковые операции
```u
// Нужно добавить методы String
lines = text.split("\n")         // разбивка
tokens = line.split(" ")         // токенизация
pos = text.find("fn")            // поиск подстроки
slice = text.substring(0, 10)    // подстрока
```

### 3. Рекурсивные структуры
```u
// Нужно для AST
enum Expr
    Number(value: Int)
    Binary(left: Expr, op: String, right: Expr)  // рекурсия!
    Call(name: String, args: List[Expr])
end
```

### 4. Работа с ошибками
```u
// Нужен Result для восстанавливаемых ошибок
fn parse(source: String) -> Result[AST, ParseError]
    if invalid
        return Err(ParseError(message: "..."))
    end
    return Ok(ast)
end
```

## Что можно сделать СЕЙЧАС

Начать с **lexer'а** (лексического анализатора) на U-lang:

```u
// lexer.u — токенизатор на U-lang

enum Token
    Identifier(name: String)
    Number(value: Int)
    String(value: String)
    Keyword(name: String)
    Symbol(value: String)
    Newline
    EOF
end

fn lex(input: String) -> List[Token]
    tokens = []
    i = 0
    while i < input.len()
        ch = input[i]
        if ch == " "
            i = i + 1
        else if is_digit(ch)
            num = parse_number(input, i)
            tokens = tokens + [Number(value: num)]
        else if is_alpha(ch)
            ident = parse_identifier(input, i)
            if is_keyword(ident)
                tokens = tokens + [Keyword(name: ident)]
            else
                tokens = tokens + [Identifier(name: ident)]
            end
        end
    end
    return tokens
end
```

## План

1. **Фаза 1**: Добавить `read_file`, базовые методы `String`
2. **Фаза 2**: Написать lexer на U-lang
3. **Фаза 3**: Добавить рекурсивные enum
4. **Фаза 4**: Написать parser на U-lang
5. **Фаза 5**: Генератор кода

## Итог

**Сейчас можно начать**: написать lexer, но без `read_file` и строковых методов он будет бесполезен.

**Рекомендация**: сначала доделать core (файлы, строки, рекурсия), потом self-hosting.

Готов начать с добавления `read_file` и методов `String`? 🔥
