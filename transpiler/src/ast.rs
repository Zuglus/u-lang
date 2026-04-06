#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct TypedField {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<TypedField>,
}

#[derive(Debug, Clone)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Stmt,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    Variant { name: String, bindings: Vec<String> },
    StringLit(String),
    Wildcard,
    List(ListPattern),
}

#[derive(Debug, Clone)]
pub enum ListPattern {
    Empty,
    Single(String),           // [x]
    Cons(String, String),     // [head, ..tail]
}

#[derive(Debug, Clone)]
pub struct FnParam {
    pub name: String,
    pub type_ann: Option<String>,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assignment { name: String, value: Expr, span: Span },
    ExprStmt { expr: Expr, span: Span },
    FnDef {
        name: String,
        params: Vec<FnParam>,
        return_type: Option<String>,
        body: Vec<Stmt>,
        is_test: bool,
        is_pub: bool,
        span: Span,
    },
    ForLoop { pattern: ForPattern, iter: Expr, body: Vec<Stmt>, span: Span },
    If {
        condition: Expr,
        body: Vec<Stmt>,
        elifs: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },
    Return { value: Option<Expr>, span: Span },
    StructDef { name: String, fields: Vec<TypedField>, is_pub: bool, span: Span },
    TypeDef { name: String, variants: Vec<Variant>, type_params: Option<Vec<String>>, is_pub: bool, span: Span },
    Match { expr: Expr, arms: Vec<MatchArm>, span: Span },
    MutAssign { object: Expr, field: String, value: Expr, span: Span },
    Spawn { expr: Expr, span: Span },
    Loop { body: Vec<Stmt>, span: Span },
    MemoryDecl { mode: String, span: Span },
    UseDecl { path: String, imports: Vec<String>, span: Span },
    TraitDef { name: String, methods: Vec<TraitMethodSig>, span: Span },
    ImplBlock { trait_name: Option<String>, target: String, methods: Vec<Stmt>, span: Span },
    WhileLoop { condition: Expr, body: Vec<Stmt>, span: Span },
    Break { span: Span },
    Continue { span: Span },
}

#[derive(Debug, Clone)]
pub enum ForPattern {
    Single(String),
    Tuple(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    StringLiteral { parts: Vec<StringPart>, span: Span },
    IntLiteral { value: i64, span: Span },
    FloatLiteral { value: f64, span: Span },
    BoolLiteral { value: bool, span: Span },
    NoneLiteral { span: Span },
    Identifier { name: String, span: Span },
    FunctionCall { name: String, args: Vec<Expr>, span: Span },
    Lambda { params: Vec<String>, body: Box<Expr>, span: Span },
    MethodCall { object: Box<Expr>, method: String, args: Vec<Expr>, is_mut: bool, span: Span },
    FieldAccess { object: Box<Expr>, field: String, span: Span },
    PostfixOp { expr: Box<Expr>, op: String, span: Span },
    BinaryOp { left: Box<Expr>, op: String, right: Box<Expr>, span: Span },
    UnaryOp { op: String, expr: Box<Expr>, span: Span },
    List { elements: Vec<Expr>, span: Span },
    Index { object: Box<Expr>, index: Box<Expr>, span: Span },
    StructInit { name: String, fields: Vec<(String, Expr)>, span: Span },
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Text(String),
    Interpolation(Expr),
}

#[derive(Debug, Clone, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
