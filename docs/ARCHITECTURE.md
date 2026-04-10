# U-lang Compiler Architecture

## Общая схема (Phases)

```
Исходный код (.u)
    ↓
[Lexer] → Tokens
    ↓
[Parser] → AST (Abstract Syntax Tree)
    ↓
[Type Checker] → Typed AST
    ↓
[IR Generator] → Intermediate Representation
    ↓
[Code Generator] → Целевой код (Rust / LLVM / ASM)
    ↓
Исполняемый файл
```

## Phase 1: Lexer (Лексический анализ)

**Вход:** строка с исходным кодом  
**Выход:** список токенов

### Токены

```u
enum Token
    // Литералы
    IntLiteral(value: Int)
    FloatLiteral(value: Float)
    StringLiteral(value: String)
    BoolLiteral(value: Bool)
    
    // Ключевые слова
    Fn | Struct | Enum | If | Else | End | For | In | While | Loop
    Match | Spawn | Return | Break | Continue | Mut | Pub | Test
    Trait | Impl | Not | And | Or
    
    // Идентификаторы и операторы
    Identifier(name: String)
    Plus | Minus | Star | Slash | Percent
    Eq | NotEq | Lt | Gt | Le | Ge
    Assign | Arrow | FatArrow
    LParen | RParen | LBracket | RBracket
    Dot | Comma | Colon
    
    // Специальные
    Newline
    EOF
end
```

### Требования к языку

- `string.char_at(i)` — доступ к символу
- `string.substring(start, end)` — подстрока
- `string.len()` — длина
- `is_digit(c)`, `is_alpha(c)` — проверки символов

---

## Phase 2: Parser (Синтаксический анализ)

**Вход:** список токенов  
**Выход:** AST

### AST структуры

```u
// Программа
struct Program
    statements: List[Stmt]
end

// Инструкции
enum Stmt
    Assignment(name: String, value: Expr)
    FunctionDef(
        name: String,
        params: List[Param],
        return_type: Option[String],
        body: List[Stmt],
        is_test: Bool,
        is_pub: Bool
    )
    StructDef(name: String, fields: List[Field], is_pub: Bool)
    EnumDef(name: String, variants: List[Variant], is_pub: Bool)
    If(condition: Expr, then_body: List[Stmt], else_body: List[Stmt])
    For(pattern: String, iterable: Expr, body: List[Stmt])
    While(condition: Expr, body: List[Stmt])
    Match(expr: Expr, arms: List[MatchArm])
    Return(value: Option[Expr])
    Break
    Continue
    Spawn(call: Expr)
    ExpressionStmt(expr: Expr)
end

// Выражения
enum Expr
    IntLiteral(value: Int)
    FloatLiteral(value: Float)
    StringLiteral(value: String)
    BoolLiteral(value: Bool)
    ListLiteral(elements: List[Expr])
    Identifier(name: String)
    BinaryOp(left: Expr, op: String, right: Expr)
    UnaryOp(op: String, expr: Expr)
    FunctionCall(name: String, args: List[Expr])
    MethodCall(object: Expr, method: String, args: List[Expr], is_mut: Bool)
    FieldAccess(object: Expr, field: String)
    IndexAccess(object: Expr, index: Expr)
    StructInit(name: String, fields: List[NamedArg])
    Lambda(params: List[String], body: Expr)
end

struct Param
    name: String
    type_ann: Option[String]
    is_mut: Bool
end

struct Field
    name: String
    type_name: String
end

struct Variant
    name: String
    fields: List[Field]
end

struct MatchArm
    pattern: Pattern
    body: List[Stmt]
end

enum Pattern
    Wildcard
    Literal(value: Expr)
    Variable(name: String)
    Constructor(name: String, fields: List[Pattern])
end

struct NamedArg
    name: String
    value: Expr
end
```

### Алгоритм

Рекурсивный спуск с Pratt parsing для выражений:

```u
fn parse(tokens: List[Token]) -> Program
fn parse_stmt() -> Stmt
fn parse_expr(precedence: Int) -> Expr
fn parse_primary() -> Expr
fn parse_function_def() -> Stmt
fn parse_struct_def() -> Stmt
fn parse_enum_def() -> Stmt
```

### Требования к языку

- Рекурсивные enum
- Рекурсивные функции
- Pattern matching на enum
- Работа с Option (Some/None)

---

## Phase 3: Type Checker (Проверка типов)

**Вход:** AST  
**Выход:** Typed AST или ошибки типизации

### Типы

```u
enum Type
    Unknown
    Int
    Float
    String
    Bool
    List(element: Type)
    Function(params: List[Type], ret: Type)
    Struct(name: String, fields: List[TypedField])
    Enum(name: String, variants: List[TypedVariant])
    UserDefined(name: String)
end

struct TypedField
    name: String
    type: Type
end

struct TypedVariant
    name: String
    fields: List[TypedField]
end
```

### Контекст

```u
struct TypeContext
    // Переменные в текущем scope
    variables: Map[String, Type]
    // Функции
    functions: Map[String, FunctionSig]
    // Структуры
    structs: Map[String, StructDef]
    // Enum'ы
    enums: Map[String, EnumDef]
    // Вариант enum → имя enum (для unit-вариантов: None, Nothing)
    variant_to_enum: Map[String, String]
    // Родительский scope (для вложенности)
    parent: Option[TypeContext]
end

struct FunctionSig
    params: List[Type]
    return_type: Type
end
```

### Алгоритм

```u
fn check_program(program: Program) -> Result[TypedProgram, List[TypeError]]
fn check_stmt(stmt: Stmt, ctx: TypeContext) -> Result[TypedStmt, TypeError]
fn check_expr(expr: Expr, ctx: TypeContext) -> Result[TypedExpr, TypeError]
fn unify(type1: Type, type2: Type) -> Result[Type, TypeError]
```

### Требования к языку

- Map (ассоциативный массив)
- Result для ошибок
- Работа с Option

---

## Phase 4: IR Generator

**Вход:** Typed AST  
**Выход:** Intermediate Representation

### IR (промежуточное представление)

```u
enum IR
    // Инструкции
    LoadConst(value: Value)
    LoadVar(name: String)
    StoreVar(name: String)
    
    // Арифметика
    Add | Sub | Mul | Div | Mod
    
    // Сравнение
    Eq | Ne | Lt | Le | Gt | Ge
    
    // Логика
    And | Or | Not
    
    // Управление потоком
    Jump(label: String)
    JumpIf(label: String)
    JumpIfNot(label: String)
    Label(name: String)
    
    // Функции
    Call(name: String, arg_count: Int)
    Return
    
    // Структуры
    FieldAccess(field: String)
    FieldAssign(field: String)
    
    // Списки
    ListNew
    ListPush
    ListGet
    ListSet
end

struct BasicBlock
    label: String
    instructions: List[IR]
end

struct FunctionIR
    name: String
    params: List[String]
    locals: List[String]
    blocks: List[BasicBlock]
end
```

### SSA (Static Single Assignment)

Опционально — преобразование в SSA-форму для оптимизаций.

---

## Phase 5: Code Generator

**Вход:** IR  
**Выход:** целевой код

### Backend опции

1. **Rust backend** (текущий)
   - Простой
   - Безопасный
   - Но зависимость от Rust

2. **LLVM backend**
   - Производительный
   - Много платформ
   - Сложный API

3. **Direct x86/ARM**
   - Максимальный контроль
   - Много работы

### Rust backend (простейший)

```u
fn generate_rust(ir: List[FunctionIR]) -> String
    code = ""
    for func in ir
        code = code + generate_function(func)
    end
    return code
end

fn generate_function(func: FunctionIR) -> String
    // Генерируем код функции на Rust
end
```

---

## Структура проекта

```
u-lang/
├── compiler/              # Компилятор на U-lang
│   ├── main.u
│   ├── lexer.u
│   ├── parser.u
│   ├── type_checker.u
│   ├── ir.u
│   └── codegen_rust.u
├── std/                   # Стандартная библиотека U-lang
│   ├── string.u
│   ├── list.u
│   ├── map.u
│   ├── file.u
│   └── channel.u
├── runtime/               # Рантайм (на Rust)
│   └── src/
└── examples/
```

---

## Roadmap к self-hosting

### Этап 1: Core (нужно для компилятора)
- [ ] Map (ассоциативный массив)
- [ ] Рекурсивные enum
- [ ] String: char_at, substring, split, find
- [ ] File: read_file, write_file
- [ ] Result для ошибок

### Этап 2: Lexer
- [ ] Token enum
- [ ] Lexer struct с состоянием
- [ ] Токенизация всех конструкций

### Этап 3: Parser ✅ (Частично)
- [x] AST enum/struct
- [x] Рекурсивный спуск
- [x] Обработка ошибок
- [x] **Дженерики** — `enum Maybe[T]`
- [x] **Unit-варианты** — `Nothing` без полей

### Этап 4: Type Checker ✅ (Частично)
- [x] Type enum
- [x] TypeContext
- [x] Проверка всех конструкций
- [x] **Дженерики для enum** — `Maybe[T]`
- [x] **Unit-варианты** — `None`, `Nothing` без полей

### Этап 5: Codegen
- [ ] IR
- [ ] Генератор Rust-кода

### Этап 6: Bootstrap
- [ ] Компилятор компилирует сам себя
- [ ] Убираем Rust-версию

---

## Что делать сейчас?

1. **Выбрать backend**: Rust (проще) или LLVM (быстрее)
2. **Реализовать Core**: Map, строки, файлы
3. **Начать с Lexer**: проще всего, хороший тест для строк
4. **Итеративно**: каждая фаза тестируется отдельно

**Рекомендация**: Сначала Core, потом Lexer, потом всё остальное.
