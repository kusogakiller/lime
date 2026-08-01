use std::fs;
use std::collections::HashMap;
const RUNTIME_C_SOURCE: &str = include_str!("codegen/runtime/runtime.c");
const RUNTIME_H_SOURCE: &str = include_str!("codegen/runtime/runtime.h");
use std::sync::{Mutex, OnceLock};
use std::hash::{Hash, Hasher};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Symbol(u32);
impl Symbol {
    const AMBIGUOUS: Symbol = Symbol(u32::MAX);
}
struct Interner {
    map: HashMap<String, Symbol>,
    values: Vec<String>,
}
impl Interner {
    fn new() -> Self {
        Interner {
            map: HashMap::new(),
            values: Vec::new(),
        }
    }
    fn intern(&mut self, name: &str) -> Symbol {
        if let Some(s) = self.map.get(name) {
            return *s;
        }
        let id = self.values.len() as u32;
        let sym = Symbol(id);
        self.values.push(name.to_string());
        self.map.insert(name.to_string(), sym);
        sym
    }
    fn resolve(&self, symbol: Symbol) -> &str {
        &self.values[symbol.0 as usize]
    }
}
fn global_interner() -> &'static Mutex<Interner> {
    static INTERNER: OnceLock<Mutex<Interner>> = OnceLock::new();
    INTERNER.get_or_init(|| Mutex::new(Interner::new()))
}
fn intern(name: &str) -> Symbol {
    let mut i = global_interner().lock().unwrap();
    i.intern(name)
}
struct StageTimer {
    name: &'static str,
    start: std::time::Instant,
}
impl StageTimer {
    fn new(name: &'static str) -> Self {
        let _ = std::env::var("LIME_PROFILE");
        StageTimer {
            name,
            start: std::time::Instant::now(),
        }
    }
}
impl Drop for StageTimer {
    fn drop(&mut self) {
        if std::env::var("LIME_PROFILE").is_ok() {
            let d = self.start.elapsed();
            let micros = d.as_micros();
            eprintln!("[profile] {}: {} us", self.name, micros);
        }
    }
}
fn resolve_sym(symbol: Symbol) -> String {
    let i = global_interner().lock().unwrap();
    i.resolve(symbol).to_string()
}
fn global_type_cache() -> &'static Mutex<HashMap<String, Type>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Type>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}
fn global_pkg_cache() -> &'static Mutex<HashMap<String, Defs>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Defs>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}
#[derive(Clone)]
struct MonoKey {
    function: Symbol,
    types: Vec<String>,
    mangled: String,
}
impl PartialEq for MonoKey {
    fn eq(&self, other: &Self) -> bool {
        self.function == other.function && self.types == other.types
    }
}
impl Eq for MonoKey {}
impl Hash for MonoKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.function.hash(state);
        self.types.hash(state);
    }
}
#[path = "codegen/mod.rs"]
mod codegen;
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Fn, Lime, Struct, Interface, State, Enum, Let, Mut, If, Else, Match, Return,
    Async, Await, Unsafe, True, False, Where, For, While, Defer,
    Int, Float, Str, Bool, Option,
    Plus, Minus, Star, Slash, Percent,
    Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign,
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
    And, Or, Not,
    Dot, DoubleDot,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Colon, DoubleColon, Semicolon, Comma, Arrow, FatArrow, Question,
    Indent, Dedent, Newline,
    IntLit(i64), LongLit(i64), FloatLit(f64), StringLit(String), Ident(String),
    Eof,
}
fn tokenize(source: &str) -> Result<(Vec<Token>, Vec<(usize, usize)>), String> {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut locs: Vec<(usize, usize)> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut emit = |t: Token, l: usize, c: usize| {
        tokens.push(t);
        locs.push((l, c));
    };
    while i < n {
        let ch = chars[i];
        if ch == '/' && i + 1 < n && chars[i + 1] == '/' {
            i += 2;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && i + 1 < n && chars[i + 1] == '*' {
            let comment_start = line;
            i += 2;
            let mut closed = false;
            while i + 1 < n {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    closed = true;
                    i += 2;
                    break;
                }
                if chars[i] == '\n' {
                    line += 1;
                    col = 1;
                }
                i += 1;
            }
            if !closed {
                return Err(format!(
                    "Unterminated block comment (opened on line {})",
                    comment_start
                ));
            }
            continue;
        }
        if ch == '\n' {
            emit(Token::Newline, line, col);
            line += 1;
            col = 1;
            i += 1;
            let mut indent = 0usize;
            loop {
                indent = 0;
                while i < n && (chars[i] == ' ' || chars[i] == '\t') {
                    indent += 1;
                    i += 1;
                }
                if i < n && chars[i] == '#' {
                    while i < n && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                if i < n && chars[i] == '\n' {
                    emit(Token::Newline, line, col);
                    line += 1;
                    i += 1;
                    continue;
                }
                break;
            }
            if i < n {
                let current = *indent_stack.last().unwrap();
                if indent > current {
                    indent_stack.push(indent);
                    emit(Token::Indent, line, col);
                } else if indent < current {
                    while indent_stack.len() > 1 && *indent_stack.last().unwrap() > indent {
                        indent_stack.pop();
                        emit(Token::Dedent, line, col);
                    }
                }
            }
            continue;
        }
        if ch == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == ' ' || ch == '\t' {
            i += 1;
            col += 1;
            continue;
        }
        if ch == '"' {
            i += 1;
            let mut s = String::new();
            while i < n && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                    if i < n {
                        match chars[i] {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            other => s.push(other),
                        }
                        i += 1;
                    }
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            if i >= n {
                return Err("Unterminated string literal".to_string());
            }
            i += 1; 
            emit(Token::StringLit(s), line, col);
            col += 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let start = i;
            let mut is_float = false;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < n && chars[i] == '.' && i + 1 < n && chars[i + 1].is_ascii_digit() {
                is_float = true;
                i += 1; 
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let num: String = chars[start..i].iter().collect();
            if is_float {
                match num.parse::<f64>() {
                    Ok(f) => emit(Token::FloatLit(f), line, col),
                    Err(_) => return Err(format!("Invalid float literal: {}", num)),
                }
            } else {
                if i < n && chars[i] == 'L' {
                    i += 1;
                    match num.parse::<i64>() {
                        Ok(v) => emit(Token::LongLit(v), line, col),
                        Err(_) => return Err(format!("Invalid long integer literal: {}", num)),
                    }
                } else {
                    match num.parse::<i64>() {
                        Ok(v) => emit(Token::IntLit(v), line, col),
                        Err(_) => return Err(format!("Invalid integer literal: {}", num)),
                    }
                }
            }
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let token = match ident.as_str() {
                "fn" => Token::Fn,
                "lime" => Token::Lime,
                "struct" => Token::Struct,
                "interface" => Token::Interface,
                "state" => Token::State,
                "enum" => Token::Enum,
                "let" => Token::Let,
                "mut" => Token::Mut,
                "if" => Token::If,
                "else" => Token::Else,
                "match" => Token::Match,
                "return" => Token::Return,
                "async" => Token::Async,
                "await" => Token::Await,
                "unsafe" => Token::Unsafe,
                "true" => Token::True,
                "false" => Token::False,
                "where" => Token::Where,
                "int" => Token::Int,
                "float" => Token::Float,
                "str" => Token::Str,
                "bool" => Token::Bool,
                "Option" => Token::Option,
                "for" => Token::For,
                "while" => Token::While,
                "defer" => Token::Defer,
                "and" => Token::And,
                "or" => Token::Or,
                "not" => Token::Not,
                _ => Token::Ident(ident),
            };
            emit(token, line, col);
            col += i - start;
            continue;
        }
        let op = match ch {
            '+' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::PlusAssign
                } else {
                    Token::Plus
                }
            }
            '-' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::MinusAssign
                } else if i < n && chars[i] == '>' {
                    i += 1;
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            '*' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::StarAssign
                } else {
                    Token::Star
                }
            }
            '/' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::SlashAssign
                } else {
                    Token::Slash
                }
            }
            '%' => {
                i += 1;
                Token::Percent
            }
            '=' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::Eq
                } else {
                    Token::Assign
                }
            }
            '!' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::NotEq
                } else {
                    Token::Not
                }
            }
            '<' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                i += 1;
                if i < n && chars[i] == '=' {
                    i += 1;
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            '&' => {
                i += 1;
                if i < n && chars[i] == '&' {
                    i += 1;
                    Token::And
                } else {
                    return Err(format!("Unexpected character '&' at {}:{}", line, col));
                }
            }
            '|' => {
                i += 1;
                if i < n && chars[i] == '|' {
                    i += 1;
                    Token::Or
                } else {
                    return Err(format!("Unexpected character '|' at {}:{}", line, col));
                }
            }
            '.' => {
                i += 1;
                if i < n && chars[i] == '.' {
                    i += 1;
                    Token::DoubleDot
                } else {
                    Token::Dot
                }
            }
            ':' => {
                i += 1;
                if i < n && chars[i] == ':' {
                    i += 1;
                    Token::DoubleColon
                } else {
                    Token::Colon
                }
            }
            ';' => {
                i += 1;
                Token::Semicolon
            }
            ',' => {
                i += 1;
                Token::Comma
            }
            '?' => {
                i += 1;
                Token::Question
            }
            '(' => {
                i += 1;
                Token::LParen
            }
            ')' => {
                i += 1;
                Token::RParen
            }
            '{' => {
                i += 1;
                Token::LBrace
            }
            '}' => {
                i += 1;
                Token::RBrace
            }
            '[' => {
                i += 1;
                Token::LBracket
            }
            ']' => {
                i += 1;
                Token::RBracket
            }
            _ => {
                return Err(format!(
                    "Unexpected character '{}' at {}:{}",
                    ch, line, col
                ));
            }
        };
        emit(op, line, col);
        col += 1;
    }
    while indent_stack.len() > 1 {
        indent_stack.pop();
        emit(Token::Dedent, line, col);
    }
    emit(Token::Eof, line, col);
    Ok((tokens, locs))
}
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedOperator {
    Builtin,
    MethodCall { method: String, op: String },
}
#[derive(Debug, Clone)]
enum Expr {
    IntLit(i64),
    LongLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    Ident(String),
    BinOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
        resolved_operator: Option<ResolvedOperator>,
    },
    UnOp {
        op: String,
        operand: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    Array(Vec<Expr>),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        target: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
    Tuple(Vec<Expr>),
    TupleAccess {
        tuple: Box<Expr>,
        index: usize,
    },
    Await(Box<Expr>),
}
#[derive(Debug, Clone)]
enum Pattern {
    Variant {
        name: String,
        bindings: Vec<String>,
    },
    Try {
        elems: Vec<Pattern>,
    },
    Error,
    Catch,
    Tuple(Vec<Pattern>),
}
#[derive(Debug, Clone)]
struct InterfaceMethod {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
}
#[derive(Debug, Clone)]
struct InterfaceDefAst {
    name: String,
    type_params: Vec<String>,
    methods: Vec<InterfaceMethod>,
}
#[derive(Debug, Clone)]
enum Stmt {
    Let {
        mutable: bool,
        name: String,
        type_hint: Option<String>,
        value: Expr,
        place: Option<MemoryPlace>,
    },
    Fn {
        name: String,
        type_params: Vec<String>,
        constraints: Vec<(String, String)>,
        params: Vec<(String, String)>,
        body: Vec<Stmt>,
        is_async: bool,
    },
    Struct {
        name: String,
        type_params: Vec<String>,
        constraints: Vec<(String, String)>,
        fields: Vec<(String, String)>,
        methods: Vec<Stmt>,
    },
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    For {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Match {
        expr: Expr,
        arms: Vec<(Pattern, Vec<Stmt>)>,
    },
    State {
        name: String,
        type_params: Vec<String>,
        variants: Vec<String>,
    },
    Enum {
        name: String,
        type_params: Vec<String>,
        variants: Vec<(String, Vec<(String, String)>)>,
        methods: Vec<Stmt>,
    },
    Interface {
        name: String,
        type_params: Vec<String>,
        constraints: Vec<(String, String)>,
        methods: Vec<InterfaceMethod>,
    },
    Return {
        explicit_type: Option<Type>,
        value: Option<Expr>,
    },
    Expr(Expr),
    Assign {
        name: String,
        value: Expr,
    },
    Destructure {
        vars: Vec<String>,
        value: Expr,
    },
    Defer {
        body: Vec<Stmt>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum MemoryPlace {
    Stack,
    Heap,
}
struct Parser {
    tokens: Vec<Token>,
    locs: Vec<(usize, usize)>,
    pos: usize,
    stmt_locs: Vec<(usize, usize)>,
    package_names: std::collections::HashSet<String>,
}
impl Parser {
    fn new(tokens: Vec<Token>, locs: Vec<(usize, usize)>) -> Self {
        Parser {
            tokens,
            locs,
            pos: 0,
            stmt_locs: Vec::new(),
            package_names: std::collections::HashSet::new(),
        }
    }
    fn set_package_names(&mut self, names: impl IntoIterator<Item = String>) {
        self.package_names = names.into_iter().collect();
    }
    fn loc(&self) -> (usize, usize) {
        self.locs.get(self.pos).copied().unwrap_or((0, 0))
    }
    fn error(&self, msg: &str) -> String {
        let (line, col) = self.loc();
        format!("{} (at line {}, col {})", msg, line, col)
    }
    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }
    fn peek_at(&self, n: usize) -> &Token {
        self.tokens.get(self.pos + n).unwrap_or(&Token::Eof)
    }
    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }
    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if std::mem::discriminant(self.current()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!(
                "Expected {:?}, got {:?}",
                expected,
                self.current()
            )))
        }
    }
    fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.current() != &Token::Eof {
            if matches!(
                self.current(),
                Token::Newline | Token::Indent | Token::Dedent
            ) {
                self.advance();
                continue;
            }
            stmts.push(self.parse_stmt().map_err(|e| self.error(&e))?);
        }
        Ok(stmts)
    }
    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let start_loc = self.loc();
        let loc_idx = self.stmt_locs.len();
        self.stmt_locs.push(start_loc);
        let _ = loc_idx;
        match self.current() {
            Token::Let => self.parse_let(),
            Token::Fn | Token::Lime => self.parse_fn(),
            Token::Struct => self.parse_struct(),
            Token::State => self.parse_state(),
            Token::Enum => self.parse_enum(),
            Token::Interface => self.parse_interface(),
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::Return => self.parse_return(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            Token::Defer => self.parse_defer(),
            _ => {
                if let Token::Ident(name) = self.current().clone() {
                    if self.peek() == &Token::Assign {
                        self.advance(); 
                        self.advance(); 
                        let value = self.parse_expr()?;
                        if self.current() == &Token::Semicolon {
                            self.advance();
                        }
                        return Ok(Stmt::Assign { name, value });
                    }
                }
                let expr = self.parse_expr()?;
                if self.current() == &Token::Semicolon {
                    self.advance();
                }
                Ok(Stmt::Expr(expr))
            }
        }
    }
    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Let)?;
        let mutable = if self.current() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        if self.current() == &Token::LParen {
            if mutable {
                return Err("Cannot use 'mut' with destructure".to_string());
            }
            self.advance();
            let mut vars = Vec::new();
            while self.current() != &Token::RParen {
                let name = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err(format!("Expected variable name in destructure, got {:?}", self.current())),
                };
                self.advance();
                vars.push(name);
                if self.current() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
            self.expect(Token::Assign)?;
            let value = self.parse_expr()?;
            if self.current() == &Token::Semicolon {
                self.advance();
            }
            return Ok(Stmt::Destructure { vars, value });
        }
        let has_type = match self.current() {
            Token::Int | Token::Float | Token::Str | Token::Bool | Token::Option => true,
            Token::Ident(_) => {
                if self.peek() == &Token::Colon {
                    true
                } else if self.peek() == &Token::LParen {
                    let mut depth = 0i32;
                    let mut i = 1usize;
                    let mut found = false;
                    loop {
                        match self.peek_at(i) {
                            Token::LParen => depth += 1,
                            Token::RParen => {
                                depth -= 1;
                                if depth == 0 {
                                    found = *self.peek_at(i + 1) == Token::Colon;
                                    break;
                                }
                            }
                            Token::Eof => break,
                            _ => {}
                        }
                        i += 1;
                    }
                    found
                } else {
                    false
                }
            }
            _ => false,
        };
        let mut place: Option<MemoryPlace> = None;
        let type_hint = if has_type {
            let mut th: Option<String> = None;
            if let (Token::Ident(base), Token::LParen, Token::Ident(kw), Token::RParen) = (
                self.current().clone(),
                self.peek().clone(),
                self.peek_at(2).clone(),
                self.peek_at(3).clone(),
            ) {
                if kw == "heap" {
                    place = Some(MemoryPlace::Heap);
                } else if kw == "stack" {
                    place = Some(MemoryPlace::Stack);
                }
                if place.is_some() {
                    self.advance(); 
                    self.advance(); 
                    self.advance(); 
                    self.advance(); 
                    th = Some(base);
                }
            }
            if th.is_none() {
                let mut ch = Vec::new();
                th = Some(self.parse_type(&mut ch)?);
            }
            self.expect(Token::Colon)?;
            th
        } else {
            None
        };
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err(format!("Expected variable name, got {:?}", self.current())),
        };
        self.advance();
        self.expect(Token::Assign)?;
        let value = self.parse_expr()?;
        if self.current() == &Token::Semicolon {
            self.advance();
        }
        Ok(Stmt::Let {
            mutable,
            name,
            type_hint,
            value,
            place,
        })
    }
    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented block".to_string());
        }
        while self.current() != &Token::Dedent
            && self.current() != &Token::Eof
        {
            if matches!(
                self.current(),
                Token::Newline | Token::Indent
            ) {
                self.advance();
                continue;
            }
            stmts.push(self.parse_stmt()?);
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(stmts)
    }
    fn parse_defer(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Defer)?;
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::Defer { body })
    }
    fn parse_fn(&mut self) -> Result<Stmt, String> {
        let is_async = match self.current() {
            Token::Fn => false,
            Token::Lime => true,
            other => return Err(format!("Expected 'fn' or 'lime', got {:?}", other)),
        };
        self.advance();
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected function name".to_string()),
        };
        self.advance();
        let mut constraints = Vec::new();
        let mut type_params = self.parse_type_params(true, &mut constraints)?;
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while self.current() != &Token::RParen {
            let mut is_untyped = false;
            let tok = self.current().clone();
            if let Token::Ident(name) = &tok {
                let saved = self.pos;
                self.advance();
                match self.current() {
                    Token::Comma | Token::RParen => {
                        params.push((name.clone(), "_".to_string()));
                        is_untyped = true;
                    }
                    _ => {
                        self.pos = saved;
                    }
                }
            }
            if is_untyped {
                if self.current() == &Token::Comma {
                    self.advance();
                }
                continue;
            }
            let param_type = self.parse_type(&mut constraints)?;
            if self.current() == &Token::Colon {
                self.advance();
                let param_name = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err("Expected parameter name".to_string()),
                };
                self.advance();
                params.push((param_name, param_type));
            } else {
                params.push(("_".to_string(), param_type));
            }
            if self.current() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
        for (tv, _) in &constraints {
            if !type_params.contains(tv) {
                type_params.push(tv.clone());
            }
        }
        Ok(Stmt::Fn {
            name,
            type_params,
            constraints,
            params,
            body,
            is_async,
        })
    }
    fn parse_type(&mut self, constraints: &mut Vec<(String, String)>) -> Result<String, String> {
        if let Token::Option = self.current() {
            self.advance();
            self.expect(Token::LParen)?;
            let inner = self.parse_type(constraints)?;
            self.expect(Token::RParen)?;
            return Ok(format!("Option({})", inner));
        }
        let base = match self.current() {
            Token::Int => {
                self.advance();
                "int".to_string()
            }
            Token::Float => {
                self.advance();
                "float".to_string()
            }
            Token::Str => {
                self.advance();
                "str".to_string()
            }
            Token::Bool => {
                self.advance();
                "bool".to_string()
            }
            Token::Ident(name) => {
                let t = name.clone();
                self.advance();
                t
            }
            _ => return Err(format!("Expected type, got {:?}", self.current())),
        };
        if base == "void" || base == "unit" || base == "u" {
            return Err(format!(
                "'{}' is not a user-facing type; omit the annotation and let Lime infer the type",
                base
            ));
        }
        if self.current() == &Token::Question {
            self.advance();
            return Ok(format!("Option({})", base));
        }
        if self.current() == &Token::LParen {
            self.advance();
            let mut inner = Vec::new();
            if self.current() != &Token::RParen {
                loop {
                    let t = self.parse_type(constraints)?;
                    inner.push(t);
                    if self.current() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.expect(Token::RParen)?;
            return Ok(format!("{}({})", base, inner.join(", ")));
        }
        if self.current() == &Token::Where {
            let tv = base.clone();
            self.advance();
            loop {
                let ctp = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err("Expected constrained type parameter".to_string()),
                };
                self.advance();
                if ctp != tv {
                    return Err(format!(
                        "Type constraint '{}' must match the declared type parameter '{}'",
                        ctp, tv
                    ));
                }
                self.expect(Token::Colon)?;
                let iface = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err("Expected interface name in constraint".to_string()),
                };
                self.advance();
                constraints.push((tv.clone(), iface));
                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        Ok(base)
    }
    fn try_parse_type_args(&mut self, constraints: &mut Vec<(String, String)>) -> Option<Vec<String>> {
        let save = self.pos;
        if self.current() != &Token::LParen {
            return None;
        }
        self.advance(); 
        let mut args = Vec::new();
        if self.current() == &Token::RParen {
            self.advance();
            return Some(args);
        }
        loop {
            match self.parse_type(constraints) {
                Ok(t) => args.push(t),
                Err(_) => {
                    self.pos = save;
                    return None;
                }
            }
            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        if self.current() == &Token::RParen {
            self.advance();
            Some(args)
        } else {
            self.pos = save;
            None
        }
    }
    fn parse_type_params(
        &mut self,
        require_paren_after: bool,
        constraints: &mut Vec<(String, String)>,
    ) -> Result<Vec<String>, String> {
        if self.current() == &Token::LParen {
            let mut depth = 0usize;
            let mut i = self.pos;
            let tokens = &self.tokens;
            loop {
                if i >= tokens.len() {
                    break;
                }
                match &tokens[i] {
                    Token::LParen => depth += 1,
                    Token::RParen => {
                        depth -= 1;
                        if depth == 0 {
                            let next_is_paren =
                                i + 1 < tokens.len() && tokens[i + 1] == Token::LParen;
                            if require_paren_after {
                                if next_is_paren {
                                    break;
                                } else {
                                    return Ok(Vec::new());
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        } else {
            return Ok(Vec::new());
        }
        self.advance(); 
        let mut params = Vec::new();
        while self.current() != &Token::RParen && self.current() != &Token::Eof {
            match self.current() {
                Token::Ident(n) => {
                    let tv = n.clone();
                    self.advance();
                    if self.current() == &Token::Where {
                        self.advance();
                        loop {
                            let ctp = match self.current() {
                                Token::Ident(c) => c.clone(),
                                _ => return Err("Expected constrained type parameter".to_string()),
                            };
                            self.advance();
                            if ctp != tv {
                                return Err(format!(
                                    "Type constraint '{}' must match the declared type parameter '{}'",
                                    ctp, tv
                                ));
                            }
                            self.expect(Token::Colon)?;
                            let iface = match self.current() {
                                Token::Ident(f) => f.clone(),
                                _ => return Err("Expected interface name in constraint".to_string()),
                            };
                            self.advance();
                            constraints.push((tv.clone(), iface));
                            if self.current() == &Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    params.push(tv);
                }
                _ => return Err(format!("Expected type parameter, got {:?}", self.current())),
            }
            if self.current() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;
        Ok(params)
    }
    fn parse_struct(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Struct)?;
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected struct name".to_string()),
        };
        self.advance();
        let mut constraints = Vec::new();
        let type_params = self.parse_type_params(false, &mut constraints)?;
        self.expect(Token::Colon)?;
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented struct body".to_string());
        }
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            if matches!(self.current(), Token::Newline | Token::Indent) {
                self.advance();
                continue;
            }
            if self.current() == &Token::Fn {
                methods.push(self.parse_fn()?);
                continue;
            }
            let field_type = self.parse_type(&mut constraints)?;
            self.expect(Token::Colon)?;
            let field_name = match self.current() {
                Token::Ident(n) => n.clone(),
                _ => return Err("Expected field name".to_string()),
            };
            self.advance();
            fields.push((field_name, field_type));
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(Stmt::Struct {
            name,
            type_params,
            constraints,
            fields,
            methods,
        })
    }
    fn parse_interface(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Interface)?;
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected interface name".to_string()),
        };
        self.advance();
        let mut constraints = Vec::new();
        let type_params = self.parse_type_params(false, &mut constraints)?;
        self.expect(Token::Colon)?;
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented interface body".to_string());
        }
        let mut methods = Vec::new();
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            if matches!(self.current(), Token::Newline | Token::Indent) {
                self.advance();
                continue;
            }
            if self.current() == &Token::Fn {
                self.advance();
                let mname = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err("Expected method name".to_string()),
                };
                self.advance();
                self.expect(Token::LParen)?;
                let mut params = Vec::new();
                while self.current() != &Token::RParen && self.current() != &Token::Eof {
                    let param_type = self.parse_type(&mut constraints)?;
                    if self.current() == &Token::Colon {
                        self.advance();
                        let pname = match self.current() {
                            Token::Ident(n) => n.clone(),
                            _ => return Err("Expected parameter name".to_string()),
                        };
                        self.advance();
                        params.push((pname, param_type));
                    } else {
                        params.push(("_".to_string(), param_type));
                    }
                    if self.current() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(Token::RParen)?;
                self.expect(Token::Colon)?;
                let return_type = match self.current() {
                    Token::Int | Token::Float | Token::Str | Token::Bool => {
                        Some(self.parse_type(&mut constraints)?)
                    }
                    Token::Ident(rn) => {
                        let t = rn.clone();
                        self.advance();
                        Some(t)
                    }
                    _ => None,
                };
                if return_type.is_some() {
                    self.expect(Token::Colon)?;
                }
                methods.push(InterfaceMethod {
                    name: mname,
                    params,
                    return_type,
                });
            } else {
                return Err(format!(
                    "Expected 'fn' in interface body, got {:?}",
                    self.current()
                ));
            }
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(Stmt::Interface {
            name,
            type_params,
            constraints,
            methods,
        })
    }
    fn parse_state(&mut self) -> Result<Stmt, String> {
        self.expect(Token::State)?;
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            other => return Err(format!("Expected state name, got {:?}", other)),
        };
        self.advance();
        let type_params = self.parse_type_params(false, &mut Vec::new())?;
        self.expect(Token::Colon)?;
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented state body".to_string());
        }
        let mut variants = Vec::new();
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            if matches!(self.current(), Token::Newline | Token::Indent) {
                self.advance();
                continue;
            }
            let variant = match self.current() {
                Token::Ident(n) => n.clone(),
                other => return Err(format!("Expected variant name, got {:?}", other)),
            };
            self.advance();
            if self.current() == &Token::LParen {
                self.advance();
                let mut depth = 1;
                while depth > 0 && self.current() != &Token::Eof {
                    match self.current() {
                        Token::LParen => depth += 1,
                        Token::RParen => depth -= 1,
                        _ => {}
                    }
                    self.advance();
                }
            }
            variants.push(variant);
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(Stmt::State {
            name,
            type_params,
            variants,
        })
    }
    fn parse_enum(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Enum)?;
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            other => return Err(format!("Expected enum name, got {:?}", other)),
        };
        self.advance();
        let type_params = self.parse_type_params(false, &mut Vec::new())?;
        self.expect(Token::Colon)?;
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented enum body".to_string());
        }
        let mut variants = Vec::new();
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            if matches!(self.current(), Token::Newline | Token::Indent) {
                self.advance();
                continue;
            }
            let variant_name = match self.current() {
                Token::Ident(n) => n.clone(),
                other => return Err(format!("Expected variant name, got {:?}", other)),
            };
            self.advance();
            let fields = if self.current() == &Token::LParen {
                self.advance();
                let mut flds = Vec::new();
                let mut field_idx = 0;
                loop {
                    if self.current() == &Token::RParen {
                        self.advance();
                        break;
                    }
                    let field_name = format!("_{}", field_idx);
                    field_idx += 1;
                    let field_type = self.parse_type(&mut Vec::new())?;
                    flds.push((field_name, field_type));
                    if self.current() == &Token::Comma {
                        self.advance();
                    } else if self.current() != &Token::RParen {
                        return Err("Expected `,` or `)` in enum variant fields".to_string());
                    }
                }
                flds
            } else {
                Vec::new()
            };
            variants.push((variant_name, fields));
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(Stmt::Enum {
            name,
            type_params,
            variants,
            methods: Vec::new(),
        })
    }
    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;
        let cond = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.current() == &Token::Else {
            self.advance();
            self.expect(Token::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::If { cond, then_branch, else_branch })
    }
    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Return)?;
        let explicit_type = self.parse_return_type();
        let expr = if matches!(
            self.current(),
            Token::Newline | Token::Indent | Token::Dedent | Token::Eof
        ) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        if self.current() == &Token::Semicolon {
            self.advance();
        }
        Ok(Stmt::Return { explicit_type, value: expr })
    }
    fn parse_return_type(&mut self) -> Option<Type> {
        let saved = self.pos;
        let ty = match self.current() {
            Token::Int => { self.advance(); Type::Int }
            Token::Float => { self.advance(); Type::Float }
            Token::Bool => { self.advance(); Type::Bool }
            Token::Str => { self.advance(); Type::String }
            Token::Ident(n) => {
                let name = n.clone();
                self.advance();
                Type::Var(name)
            }
            _ => return None,
        };
        if self.current() == &Token::Colon {
            self.advance();
            Some(ty)
        } else {
            self.pos = saved;
            None
        }
    }
    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(Token::For)?;
        let var = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err(format!("Expected loop variable, got {:?}", self.current())),
        };
        self.advance();
        self.expect(Token::Ident("in".to_string())).map_err(|_| {
            "Expected 'in' in for loop".to_string()
        })?;
        let iterable = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::For { var, iterable, body })
    }
    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(Token::While)?;
        let cond = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }
    fn parse_match(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Match)?;
        let expr = self.parse_expr()?;
        self.expect(Token::Colon)?;
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented match body".to_string());
        }
        let mut arms = Vec::new();
        let mut seen_try = false;
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            if matches!(self.current(), Token::Newline | Token::Indent) {
                self.advance();
                continue;
            }
            if self.current() == &Token::Else {
                return Err("Match else is not allowed (exhaustive match required)".to_string());
            }
            let pattern = if self.current() == &Token::LParen {
                return Err(
                    "Expected a variant or `try (...)` in match arm, got a bare tuple pattern (write `try (...)` to match a tuple)"
                        .to_string(),
                );
            } else {
                let name = match self.current() {
                    Token::Ident(n) => n.clone(),
                    other => return Err(format!("Expected variant name in match, got {:?}", other)),
                };
                self.advance();
                if name == "_" {
                    return Err(
                        "`_` is no longer a wildcard pattern; use `catch:` instead".to_string(),
                    );
                } else if name == "catch" {
                    if matches!(self.current(), Token::Ident(_) | Token::LParen) {
                        return Err(
                            "`catch` is a catch-all arm and does not bind values".to_string(),
                        );
                    }
                    Pattern::Catch
                } else if name == "try" {
                    if self.current() != &Token::LParen {
                        return Err(
                            "`try` must be followed by a tuple pattern, e.g. `try (a, b):`"
                                .to_string(),
                        );
                    }
                    self.advance();
                    let elems = self.parse_tuple_elems()?;
                    self.expect(Token::RParen)?;
                    seen_try = true;
                    Pattern::Try { elems }
                } else if name == "error" {
                    if self.current() == &Token::Ident("as".to_string()) {
                        return Err(
                            "`error as x` is deprecated; write `error:` and use the failure payload inside the body"
                                .to_string(),
                        );
                    }
                    if !seen_try {
                        return Err(
                            "`error:` is only valid in a match arm following a `try (...)` arm"
                                .to_string(),
                        );
                    }
                    Pattern::Error
                } else {
                    let mut bindings = Vec::new();
                    if self.current() == &Token::LParen {
                        self.advance();
                        while self.current() != &Token::RParen {
                            let b = match self.current() {
                                Token::Ident(n) => n.clone(),
                                other => {
                                    return Err(format!("Expected binding name, got {:?}", other))
                                }
                            };
                            self.advance();
                            bindings.push(b);
                            if self.current() == &Token::Comma {
                                self.advance();
                            }
                        }
                        self.expect(Token::RParen)?;
                    }
                    Pattern::Variant { name, bindings }
                }
            };
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
            arms.push((pattern, body));
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(Stmt::Match { expr, arms })
    }
    fn parse_tuple_pattern(&mut self) -> Result<Pattern, String> {
        self.expect(Token::LParen)?;
        let elems = self.parse_tuple_elems()?;
        self.expect(Token::RParen)?;
        Ok(Pattern::Tuple(elems))
    }
    fn parse_tuple_elems(&mut self) -> Result<Vec<Pattern>, String> {
        let mut elems = Vec::new();
        while self.current() != &Token::RParen {
            let elem = if self.current() == &Token::LParen {
                self.parse_tuple_pattern()?
            } else {
                let name = match self.current() {
                    Token::Ident(n) => n.clone(),
                    other => return Err(format!("Expected pattern element, got {:?}", other)),
                };
                self.advance();
                if name == "_" {
                    return Err(
                        "`_` is no longer a wildcard; use `catch` to ignore a tuple element"
                            .to_string(),
                    );
                } else if name == "catch" {
                    Pattern::Catch
                } else {
                    Pattern::Variant { name, bindings: vec![] }
                }
            };
            elems.push(elem);
            if self.current() == &Token::Comma {
                self.advance();
            }
        }
        Ok(elems)
    }
    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_binary(0)
    }
    fn bin_prec(op: &str) -> Option<u8> {
        match op {
            "or" => Some(1),
            "and" => Some(2),
            "==" | "!=" => Some(3),
            "<" | ">" | "<=" | ">=" => Some(4),
            "+" | "-" => Some(5),
            "*" | "/" | "%" => Some(6),
            _ => None,
        }
    }
    fn parse_binary(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current() {
                Token::Plus => "+".to_string(),
                Token::Minus => "-".to_string(),
                Token::Star => "*".to_string(),
                Token::Slash => "/".to_string(),
                Token::Percent => "%".to_string(),
                Token::Eq => "==".to_string(),
                Token::NotEq => "!=".to_string(),
                Token::Lt => "<".to_string(),
                Token::Gt => ">".to_string(),
                Token::LtEq => "<=".to_string(),
                Token::GtEq => ">=".to_string(),
                Token::And => "and".to_string(),
                Token::Or => "or".to_string(),
                _ => break,
            };
            let prec = match Self::bin_prec(&op) {
                Some(p) => p,
                None => break,
            };
            if prec < min_prec {
                break;
            }
            self.advance();
            let right = self.parse_binary(prec + 1)?;
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                resolved_operator: None,
            };
        }
        Ok(left)
    }
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.current() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnOp { op: "-".to_string(), operand: Box::new(operand) })
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnOp { op: "not".to_string(), operand: Box::new(operand) })
            }
            Token::Await => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Await(Box::new(operand)))
            }
            _ => self.parse_postfix(),
        }
    }
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_atom()?;
        let mut dotted_path: Vec<String> = Vec::new();
        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let name = match self.current().clone() {
                        Token::Ident(n) => n,
                        Token::Int => "int".to_string(),
                        Token::Float => "float".to_string(),
                        Token::Str => "str".to_string(),
                        Token::IntLit(n) => n.to_string(),
                        other => return Err(format!("Expected method/field name, got {:?}", other)),
                    };
                    self.advance();
                    let is_call = self.current() == &Token::LParen;
                    if let Expr::Ident(base) = &expr {
                        dotted_path = vec![base.clone()];
                    }
                    dotted_path.push(name.clone());
                    if is_call && (dotted_path.len() >= 3
                        || (dotted_path.len() == 2
                            && self.package_names.contains(&dotted_path[0])))
                    {
                        let func = dotted_path.join(".");
                        self.advance();
                        let mut args = Vec::new();
                        while self.current() != &Token::RParen && self.current() != &Token::Eof {
                            args.push(self.parse_expr()?);
                            if self.current() == &Token::Comma {
                                self.advance();
                            }
                        }
                        self.expect(Token::RParen)?;
                        expr = Expr::Call { func, args };
                    } else if is_call {
                        let obj = expr.clone();
                        self.advance();
                        let mut args = Vec::new();
                        while self.current() != &Token::RParen && self.current() != &Token::Eof {
                            args.push(self.parse_expr()?);
                            if self.current() == &Token::Comma {
                                self.advance();
                            }
                        }
                        self.expect(Token::RParen)?;
                        expr = Expr::MethodCall {
                            object: Box::new(obj),
                            method: name,
                            args,
                        };
                    } else if name.chars().all(|c| c.is_ascii_digit()) {
                        let index: usize = name.parse().unwrap_or(0);
                        expr = Expr::TupleAccess {
                            tuple: Box::new(expr),
                            index,
                        };
                    } else {
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field: name,
                        };
                    }
                }
                Token::LParen => {
                    if let Expr::Ident(func) = &expr {
                        let save = self.pos;
                        if let Some(type_args) =
                            self.try_parse_type_args(&mut Vec::new())
                        {
                            let typed_name = format!("{}({})", func, type_args.join(", "));
                            if self.current() == &Token::LParen {
                                self.advance();
                                let mut args = Vec::new();
                                while self.current() != &Token::RParen
                                    && self.current() != &Token::Eof
                                {
                                    args.push(self.parse_expr()?);
                                    if self.current() == &Token::Comma {
                                        self.advance();
                                    }
                                }
                                self.expect(Token::RParen)?;
                                expr = Expr::Call {
                                    func: typed_name,
                                    args,
                                };
                            } else {
                                self.pos = save;
                                self.advance();
                                let mut args = Vec::new();
                                while self.current() != &Token::RParen
                                    && self.current() != &Token::Eof
                                {
                                    args.push(self.parse_expr()?);
                                    if self.current() == &Token::Comma {
                                        self.advance();
                                    }
                                }
                                self.expect(Token::RParen)?;
                                expr = Expr::Call {
                                    func: func.clone(),
                                    args,
                                };
                            }
                        } else {
                            self.pos = save;
                            self.advance();
                            let mut args = Vec::new();
                            while self.current() != &Token::RParen
                                && self.current() != &Token::Eof
                            {
                                args.push(self.parse_expr()?);
                                if self.current() == &Token::Comma {
                                    self.advance();
                                }
                            }
                            self.expect(Token::RParen)?;
                            expr = Expr::Call {
                                func: func.clone(),
                                args,
                            };
                        }
                    } else {
                        break;
                    }
                }
                Token::DoubleDot => {
                    self.advance();
                    let end = self.parse_expr()?;
                    expr = Expr::Range {
                        start: Box::new(expr),
                        end: Box::new(end),
                    };
                }
                Token::LBracket => {
                    self.advance();
                    if self.current() == &Token::Colon {
                        self.advance();
                        let end = if self.current() == &Token::RBracket {
                            None
                        } else {
                            Some(Box::new(self.parse_expr()?))
                        };
                        self.expect(Token::RBracket)?;
                        expr = Expr::Slice {
                            target: Box::new(expr),
                            start: None,
                            end,
                        };
                    } else {
                        let first = self.parse_expr()?;
                        if self.current() == &Token::Colon {
                            self.advance();
                            let end = if self.current() == &Token::RBracket {
                                None
                            } else {
                                Some(Box::new(self.parse_expr()?))
                            };
                            self.expect(Token::RBracket)?;
                            expr = Expr::Slice {
                                target: Box::new(expr),
                                start: Some(Box::new(first)),
                                end,
                            };
                        } else {
                            self.expect(Token::RBracket)?;
                            expr = Expr::Index {
                                target: Box::new(expr),
                                index: Box::new(first),
                            };
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }
    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            Token::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::LongLit(n) => {
                self.advance();
                Ok(Expr::LongLit(n))
            }
            Token::FloatLit(n) => {
                self.advance();
                Ok(Expr::FloatLit(n))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Token::True => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            Token::Ident(name) => {
                self.advance();
                if name == "None" {
                    Ok(Expr::Call {
                        func: "None".to_string(),
                        args: vec![],
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Token::Int | Token::Float | Token::Str => {
                if self.peek() == &Token::LParen {
                    let name = match self.current() {
                        Token::Int => "int".to_string(),
                        Token::Float => "float".to_string(),
                        Token::Str => "str".to_string(),
                        _ => unreachable!(),
                    };
                    self.advance(); 
                    self.advance(); 
                    let mut args = Vec::new();
                    while self.current() != &Token::RParen && self.current() != &Token::Eof {
                        args.push(self.parse_expr()?);
                        if self.current() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { func: name, args })
                } else {
                    Err(format!(
                        "Unexpected token in expression position: {:?}",
                        self.current()
                    ))
                }
            }
            Token::LParen => {
                self.advance();
                let first = self.parse_expr()?;
                if self.current() == &Token::Comma {
                    let mut elems = vec![first];
                    while self.current() == &Token::Comma {
                        self.advance();
                        if self.current() == &Token::RParen {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Tuple(elems))
                } else {
                    self.expect(Token::RParen)?;
                    Ok(first)
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while self.current() != &Token::RBracket {
                    elements.push(self.parse_expr()?);
                    if self.current() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(elements))
            }
            _ => Err(self.error(&format!("Unexpected token: {:?}", self.current()))),
        }
    }
}
fn parse(
    tokens: Vec<Token>,
    locs: Vec<(usize, usize)>,
) -> Result<Vec<Stmt>, String> {
    let mut parser = Parser::new(tokens, locs);
    parser.parse_program()
}
fn parse_with_locs(
    tokens: Vec<Token>,
    locs: Vec<(usize, usize)>,
    package_names: &std::collections::HashSet<String>,
) -> Result<(Vec<Stmt>, Vec<(usize, usize)>), String> {
    let mut parser = Parser::new(tokens, locs);
    parser.set_package_names(package_names.iter().cloned());
    let stmts = parser.parse_program()?;
    Ok((stmts, parser.stmt_locs))
}
const REGISTRY_ROOT: &str = "packages";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}
impl Version {
    fn as_string(&self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }
}
fn parse_version(s: &str) -> Result<Version, String> {
    let s = s.trim();
    let s = s.strip_prefix('v').unwrap_or(s);
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid version '{}' (expected vMAJOR.MINOR.PATCH)", s));
    }
    let major = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("Invalid version major in '{}'", s))?;
    let minor = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid version minor in '{}'", s))?;
    let patch = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid version patch in '{}'", s))?;
    Ok(Version { major, minor, patch })
}
#[derive(Debug, Clone)]
pub struct CitrusToml {
    pub name: String,
    pub version: String,
    pub files: HashMap<String, String>,
    pub imports: HashMap<String, String>,
}
pub fn parse_citrus_toml(path: &str) -> Result<CitrusToml, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read citrus.toml '{}': {}", path, e))?;
    let mut name = String::new();
    let mut version = String::new();
    let mut files: HashMap<String, String> = HashMap::new();
    let mut imports: HashMap<String, String> = HashMap::new();
    let mut current_section = String::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let val = line[eq + 1..].trim().trim_matches('"').to_string();
            match current_section.as_str() {
                "package" => match key.as_str() {
                    "name" => name = val,
                    "version" => version = val,
                    _ => {}
                },
                "files" => {
                    files.insert(key, val);
                }
                "import" => {
                    imports.insert(key, val);
                }
                _ => {}
            }
        }
    }
    if name.is_empty() {
        return Err(format!("citrus.toml '{}' missing [package].name", path));
    }
    Ok(CitrusToml {
        name,
        version,
        files,
        imports,
    })
}
#[derive(Debug, Clone)]
struct SourceFile {
    path: String,
    statements: Vec<Stmt>,
    locs: Vec<(usize, usize)>,
}
#[derive(Debug, Clone)]
struct Project {
    files: Vec<SourceFile>,
}
fn load_project_stmts(citrus_path: &str) -> Result<Project, String> {
    let cfg = parse_citrus_toml(citrus_path)?;
    let base_dir = std::path::Path::new(citrus_path)
        .parent()
        .map(|p| {
            let s = p.to_string_lossy().to_string();
            if s.is_empty() { ".".to_string() } else { s }
        })
        .unwrap_or_else(|| ".".to_string());
    let mut files = Vec::new();
    for (name, rel) in &cfg.files {
        let full = if rel.starts_with('/') || rel.contains(':') {
            rel.clone()
        } else {
            format!("{}/{}", base_dir, rel)
        };
        let source = fs::read_to_string(&full)
            .map_err(|e| format!("Error reading file '{}' (from [files].{}): {}", full, name, e))?;
        let (tokens, locs) = {
            let _t = StageTimer::new("tokenize");
            tokenize(&source)
                .map_err(|e| format!("Lexer error in '{}': {}", full, e))?
        };
        let pkg_names: std::collections::HashSet<String> =
            cfg.imports.keys().cloned().collect();
        let (stmts, stmt_locs) = {
            let _t = StageTimer::new("parse");
            parse_with_locs(tokens, locs, &pkg_names)
                .map_err(|e| format!("Parser error in '{}': {}", full, e))?
        };
        files.push(SourceFile {
            path: full,
            statements: stmts,
            locs: stmt_locs,
        });
    }
    Ok(Project { files })
}
fn resolve_local_package(pkg_name: &str, version: &str, registry_root: &str) -> Option<String> {
    let dir = format!("{}/{}/{}", registry_root, pkg_name, version);
    let toml_path = format!("{}/citrus.toml", dir);
    if std::path::Path::new(&toml_path).exists() {
        Some(toml_path)
    } else {
        None
    }
}
fn list_registry_packages(registry_root: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(registry_root) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}
struct ImportResolver<'a> {
    cfg: &'a CitrusToml,
    import_from: Vec<(String, String)>, 
    alias_table: HashMap<String, String>,
}
impl<'a> ImportResolver<'a> {
    fn new(cfg: &'a CitrusToml) -> Self {
        let mut import_from: Vec<(String, String)> = Vec::new();
        let mut alias_table: HashMap<String, String> = HashMap::new();
        for (pkg, ver) in &cfg.imports {
            import_from.push((pkg.clone(), ver.clone()));
            alias_table.insert(pkg.clone(), pkg.clone());
        }
        ImportResolver {
            cfg,
            import_from,
            alias_table,
        }
    }
    fn validate_imports(&self, registry_root: &str) -> Result<(), String> {
        for (pkg, ver) in &self.import_from {
            if resolve_local_package(pkg, ver, registry_root).is_none() {
                let hint = nearest(pkg.as_str(), list_registry_packages(registry_root));
                let mut msg = format!(
                    "Unresolved import: '{}' version '{}' not found in registry '{}'",
                    pkg, ver, registry_root
                );
                if let Some(s) = hint {
                    msg.push_str(&format!("\n  did you mean '{}'?", s));
                }
                return Err(msg);
        }
    }
    Ok(())
}
    fn build_dependency_graph(
        &self,
        registry_root: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let mut edges: Vec<(String, String)> = Vec::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut in_stack: std::collections::HashSet<String> = std::collections::HashSet::new();
        fn visit(
            node: &str,
            edges: &mut Vec<(String, String)>,
            visited: &mut std::collections::HashSet<String>,
            in_stack: &mut std::collections::HashSet<String>,
            registry_root: &str,
        ) -> Result<(), String> {
            if in_stack.contains(node) {
                return Err(format!("Cyclic dependency detected involving '{}'", node));
            }
            if visited.contains(node) {
                return Ok(());
            }
            in_stack.insert(node.to_string());
            if let Some(toml) = resolve_local_package(
                node,
                &read_pkg_version(node, registry_root),
                registry_root,
            ) {
                if let Ok(cfg) = parse_citrus_toml(&toml) {
                    for (dep, _ver) in &cfg.imports {
                        edges.push((node.to_string(), dep.clone()));
                        visit(dep, edges, visited, in_stack, registry_root)?;
                    }
                }
            }
            in_stack.remove(node);
            visited.insert(node.to_string());
            Ok(())
        }
        for (pkg, _ver) in &self.import_from {
            visit(pkg, &mut edges, &mut visited, &mut in_stack, registry_root)?;
        }
        Ok(edges)
    }
    fn alias_table(&self) -> HashMap<String, String> {
        self.alias_table.clone()
    }
    fn apply_to_defs(&self, defs: &mut Defs, registry_root: &str) {
        let mut queue: std::collections::VecDeque<(String, String)> =
            self.import_from.iter().cloned().collect();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        while let Some((pkg, ver)) = queue.pop_front() {
            if !seen.insert(format!("{}={}", pkg, ver)) {
                continue;
            }
            if let Some(toml) = resolve_local_package(&pkg, &ver, registry_root) {
                let pkg_defs = pkg_defs_for(&pkg, &toml);
                for (old, fdef) in pkg_defs.functions.iter() {
                    let name = format!("{}.{}", pkg, old);
                    defs.functions.insert(name.clone(), fdef.clone());
                    defs.add_function_index_only(name);
                }
                for (old, sdef) in pkg_defs.structs.iter() {
                    let name = format!("{}.{}", pkg, old);
                    defs.structs.insert(name.clone(), sdef.clone());
                    defs.add_type_index_only(name);
                }
                for (old, idef) in pkg_defs.interfaces.iter() {
                    let name = format!("{}.{}", pkg, old);
                    defs.interfaces.insert(name.clone(), idef.clone());
                    defs.add_type_index_only(name);
                }
                for (old, variants) in pkg_defs.states.iter() {
                    let name = format!("{}.{}", pkg, old);
                    defs.states.insert(name.clone(), variants.clone());
                    defs.add_type_index_only(name);
                }
            }
        }
    }
}
fn pkg_defs_for(_pkg_name: &str, toml_path: &str) -> Defs {
    if let Some(cached) = global_pkg_cache().lock().unwrap().get(toml_path) {
        return cached.clone();
    }
    let _t = StageTimer::new("pkg_parse");
    let mut pkg_defs = Defs::new();
    if let Ok(project) = load_project_stmts(toml_path) {
        for file in &project.files {
            collect_defs(&file.statements, &mut pkg_defs);
        }
    }
    drop(_t);
    global_pkg_cache()
        .lock()
        .unwrap()
        .insert(toml_path.to_string(), pkg_defs.clone());
    pkg_defs
}
fn read_pkg_version(pkg: &str, registry_root: &str) -> String {
    let dir = format!("{}/{}", registry_root, pkg);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    return name.to_string();
                }
            }
        }
    }
    "v0.1.0".to_string()
}
fn collect_defs_from_project(project: &Project, defs: &mut Defs) {
    for file in &project.files {
        collect_defs(&file.statements, defs);
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompileMode {
    Build,
    Run,
    Check,
}
#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub emit_ll: bool,
    pub emit_object: bool,
    pub optimize: bool,
    pub release: bool,
    pub verbose: bool,
}
impl Default for CompileOptions {
    fn default() -> Self {
        CompileOptions {
            emit_ll: false,
            emit_object: false,
            optimize: true,
            release: false,
            verbose: false,
        }
    }
}
#[derive(Debug, Default, Clone)]
pub struct CompileReport {
    pub removed_functions: usize,
    pub lock_written: bool,
    pub cache_populated: bool,
    pub emitted_ll: Option<String>,
    pub emitted_obj: Option<String>,
    pub emitted_exe: Option<String>,
    pub codegen_warnings: Vec<String>,
    pub warnings: Vec<String>,
    pub executed: bool,
}
struct LoadedProject {
    stmts: Vec<Stmt>,
    defs: Defs,
    cfg: Option<CitrusToml>,
    edges: Vec<(String, String)>,
    base_dir: Option<String>,
    stmt_locs: Option<Vec<(usize, usize)>>,
    file: Option<String>,
}
fn load_target(path: &str) -> Result<LoadedProject, String> {
    if path.ends_with("citrus.toml") {
        let cfg = parse_citrus_toml(path)?;
        let project = {
            let _t = StageTimer::new("load_project_stmts");
            load_project_stmts(path)?
        };
        let mut stmts: Vec<Stmt> = Vec::new();
        let mut all_locs: Vec<(usize, usize)> = Vec::new();
        let mut main_file: Option<String> = None;
        for file in &project.files {
            stmts.extend(file.statements.iter().cloned());
            all_locs.extend(file.locs.iter().cloned());
            if main_file.is_none() {
                main_file = Some(file.path.clone());
            }
        }
        let mut defs = Defs::new();
        collect_defs_from_project(&project, &mut defs);
        let resolver = ImportResolver::new(&cfg);
        {
            let _t = StageTimer::new("import_resolve");
            resolver.validate_imports(REGISTRY_ROOT)?;
            let edges = {
                let _t = StageTimer::new("dep_graph");
                resolver.build_dependency_graph(REGISTRY_ROOT)?
            };
            {
                let _t = StageTimer::new("apply_to_defs");
                resolver.apply_to_defs(&mut defs, REGISTRY_ROOT);
            }
            drop(_t);
            return Ok(LoadedProject {
                stmts,
                defs,
                cfg: Some(cfg),
                edges,
                base_dir: Some(
                    std::path::Path::new(path)
                        .parent()
                        .map(|p| {
                            let s = p.to_string_lossy().to_string();
                            if s.is_empty() { ".".to_string() } else { s }
                        })
                        .unwrap_or_else(|| ".".to_string()),
                ),
                stmt_locs: Some(all_locs),
                file: main_file,
            });
        }
    } else {
        let source = fs::read_to_string(path)
            .map_err(|e| format!("Error reading file '{}': {}", path, e))?;
        let (tokens, locs) = {
            let _t = StageTimer::new("tokenize");
            tokenize(&source).map_err(|e| format!("Lexer error: {}", e))?
        };
        let pkg_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        let (stmts, stmt_locs) = {
            let _t = StageTimer::new("parse");
            parse_with_locs(tokens, locs, &pkg_names)
                .map_err(|e| format!("Parser error: {}", e))?
        };
        let mut defs = Defs::new();
        collect_defs(&stmts, &mut defs);
        Ok(LoadedProject {
            stmts,
            defs,
            cfg: None,
            edges: Vec::new(),
            base_dir: None,
            stmt_locs: Some(stmt_locs),
            file: Some(path.to_string()),
        })
    }
}
fn infer_function_return_types(defs: &mut Defs) -> Result<(), String> {
    let fnames: Vec<String> = defs.functions.keys().cloned().collect();
    for fname in &fnames {
        let (body, params, constraints, type_params) = {
            let fdef = match defs.functions.get(fname) {
                Some(f) => f,
                None => continue,
            };
            (fdef.body.clone(), fdef.params.clone(), fdef.constraints.clone(), fdef.type_params.clone())
        };
        let mut env_vars: HashMap<String, Type> = HashMap::new();
        let mut env_cons: HashMap<String, Vec<String>> = HashMap::new();
        for (tv, iface) in &constraints {
            env_cons.entry(tv.clone()).or_default().push(iface.clone());
        }
        for (pname, ptype) in &params {
            env_vars.insert(pname.clone(), type_from_str(ptype, defs));
        }
        let mut ret_type: Option<Type> = None;
        let mut env = env_vars.clone();
        scan_return_types_env(&body, defs, &mut env, &env_cons, &mut ret_type);
        if let Some(ref t) = ret_type {
            if t != &Type::Unknown && t != &Type::Unit {
                let struct_tp: Option<(String, Vec<String>)> = if !type_params.is_empty() {
                    match t {
                        Type::Struct(sname) if !sname.contains('(') => {
                            defs.structs.get(sname).and_then(|sd| {
                                if !sd.type_params.is_empty() {
                                    Some((sname.clone(), sd.type_params.clone()))
                                } else { None }
                            })
                        }
                        Type::State(sname) if !sname.contains('(') => {
                            defs.enum_type_params.get(sname).and_then(|etp| {
                                if !etp.is_empty() {
                                    Some((sname.clone(), etp.clone()))
                                } else { None }
                            })
                        }
                        _ => None,
                    }
                } else { None };
                if let Some(fdef) = defs.functions.get_mut(fname) {
                    if fdef.return_type.is_none() {
                        let mut rt_str = type_to_string(t);
                        if let Some((ref sname, _)) = struct_tp {
                            rt_str = format!("{}({})", sname, type_params.join(","));
                        }
                        fdef.return_type = Some(rt_str);
                    }
                }
            }
        }
    }
    let sname_list: Vec<String> = defs.structs.keys().cloned().collect();
    for sname in &sname_list {
        let methods: Vec<(String, Vec<Stmt>, Vec<(String, String)>, Vec<(String, String)>)> = {
            let sdef = match defs.structs.get(sname) {
                Some(s) => s,
                None => continue,
            };
            sdef.methods.iter().map(|(mname, mdef)| {
                (mname.clone(), mdef.body.clone(), mdef.params.clone(), mdef.constraints.clone())
            }).collect()
        };
        let methods_with_tp: Vec<(String, Vec<Stmt>, Vec<(String, String)>, Vec<(String, String)>, Vec<String>)> = {
            let sdef = match defs.structs.get(sname) {
                Some(s) => s,
                None => continue,
            };
            sdef.methods.iter().map(|(mname, mdef)| {
                (mname.clone(), mdef.body.clone(), mdef.params.clone(), mdef.constraints.clone(), mdef.type_params.clone())
            }).collect()
        };
        for (mname, body, params, constraints, type_params) in &methods_with_tp {
            let mut env_vars: HashMap<String, Type> = HashMap::new();
            let mut env_cons: HashMap<String, Vec<String>> = HashMap::new();
            for (tv, iface) in constraints {
                env_cons.entry(tv.clone()).or_default().push(iface.clone());
            }
            for (pname, ptype) in params {
                env_vars.insert(pname.clone(), type_from_str(ptype, defs));
            }
            let mut ret_type: Option<Type> = None;
            let mut env = env_vars.clone();
            scan_return_types_env(&body, defs, &mut env, &env_cons, &mut ret_type);
            if let Some(ref t) = ret_type {
                if t != &Type::Unknown && t != &Type::Unit {
                    let struct_tp: Option<(String, Vec<String>)> = if !type_params.is_empty() {
                        match t {
                            Type::Struct(sname2) if !sname2.contains('(') => {
                                defs.structs.get(sname2).and_then(|sd| {
                                    if !sd.type_params.is_empty() {
                                        Some((sname2.clone(), sd.type_params.clone()))
                                    } else { None }
                                })
                            }
                            Type::State(sname2) if !sname2.contains('(') => {
                                defs.enum_type_params.get(sname2).and_then(|etp| {
                                    if !etp.is_empty() {
                                        Some((sname2.clone(), etp.clone()))
                                    } else { None }
                                })
                            }
                            _ => None,
                        }
                    } else { None };
                    if let Some(sdef) = defs.structs.get_mut(sname) {
                        if let Some(mdef) = sdef.methods.get_mut(mname) {
                            if mdef.return_type.is_none() {
                                let mut rt_str = type_to_string(t);
                                if let Some((ref sname2, _)) = struct_tp {
                                    rt_str = format!("{}({})", sname2, type_params.join(","));
                                }
                                mdef.return_type = Some(rt_str);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
fn infer_untyped_params(stmts: &[Stmt], defs: &mut Defs) -> Result<(), String> {
    let mut call_info: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    {
        let fnames: Vec<String> = defs.functions.keys().cloned().collect();
        for fname in &fnames {
            let (body, params, constraints) = {
                let fdef = match defs.functions.get(fname) {
                    Some(f) => f,
                    None => continue,
                };
                if !fdef.params.iter().any(|(_, pt)| pt == "_") {
                    continue;
                }
                (fdef.body.clone(), fdef.params.clone(), fdef.constraints.clone())
            };
            let mut env: HashMap<String, Type> = HashMap::new();
            let mut env_cons: HashMap<String, Vec<String>> = HashMap::new();
            for (tv, iface) in &constraints {
                env_cons.entry(tv.clone()).or_default().push(iface.clone());
            }
            let mut untyped_indices: Vec<(usize, String)> = Vec::new();
            for (i, (pname, ptype)) in params.iter().enumerate() {
                if ptype == "_" {
                    let tv_name = format!("__inf_{}_{}", fname, i);
                    env.insert(pname.clone(), Type::Var(tv_name.clone()));
                    untyped_indices.push((i, tv_name));
                } else {
                    env.insert(pname.clone(), type_from_str(ptype, defs));
                }
            }
            infer_params_from_body(&body, &mut env, defs, &env_cons, &mut call_info, fname, &untyped_indices);
        }
    }
    fn infer_params_from_body(
        stmts: &[Stmt],
        env: &mut HashMap<String, Type>,
        defs: &Defs,
        env_cons: &HashMap<String, Vec<String>>,
        call_info: &mut HashMap<String, Vec<(usize, String)>>,
        fname: &str,
        untyped_indices: &[(usize, String)],
    ) {
        for stmt in stmts {
            match stmt {
                Stmt::Expr(e) | Stmt::Return { value: Some(e), .. } => {
                    infer_expr_params(e, env, defs, env_cons, call_info, fname, untyped_indices);
                }
                Stmt::Let { name, value, type_hint, .. } => {
                    infer_expr_params(value, env, defs, env_cons, call_info, fname, untyped_indices);
                    if let Some(th) = type_hint {
                        env.insert(name.clone(), type_from_str(th, defs));
                    } else if let Ok(t) = infer_type(value, env, defs, env_cons) {
                        if t != Type::Unknown {
                            env.insert(name.clone(), t);
                        }
                    }
                }
                Stmt::Assign { name, value, .. } => {
                    infer_expr_params(value, env, defs, env_cons, call_info, fname, untyped_indices);
                    if let Ok(t) = infer_type(value, env, defs, env_cons) {
                        env.insert(name.clone(), t);
                    }
                }
                Stmt::If { then_branch, else_branch, .. } => {
                    infer_params_from_body(then_branch, env, defs, env_cons, call_info, fname, untyped_indices);
                    if let Some(eb) = else_branch {
                        infer_params_from_body(eb, env, defs, env_cons, call_info, fname, untyped_indices);
                    }
                }
                Stmt::While { body, .. } => {
                    infer_params_from_body(body, env, defs, env_cons, call_info, fname, untyped_indices);
                }
                Stmt::For { body, .. } => {
                    infer_params_from_body(body, env, defs, env_cons, call_info, fname, untyped_indices);
                }
                Stmt::Match { arms, .. } => {
                    for (_, arm_body) in arms {
                        infer_params_from_body(arm_body, env, defs, env_cons, call_info, fname, untyped_indices);
                    }
                }
                _ => {}
            }
        }
    }
    fn infer_expr_params(
        e: &Expr,
        env: &mut HashMap<String, Type>,
        defs: &Defs,
        env_cons: &HashMap<String, Vec<String>>,
        call_info: &mut HashMap<String, Vec<(usize, String)>>,
        fname: &str,
        untyped_indices: &[(usize, String)],
    ) {
        match e {
            Expr::BinOp { left, op, right, .. } => {
                let lt = infer_type(left, env, defs, env_cons).unwrap_or(Type::Unknown);
                let rt = infer_type(right, env, defs, env_cons).unwrap_or(Type::Unknown);
                if let Some((_, result_ty)) = resolve_operator_interface(defs, &lt, &rt, op, env_cons) {
                    let resolved_lt = if matches!(&lt, Type::Var(_)) { &result_ty } else { &lt };
                    let resolved_rt = if matches!(&rt, Type::Var(_)) { &result_ty } else { &rt };
                    if let Type::Var(tv) = &lt {
                        if !matches!(resolved_lt, Type::Var(_)) && *resolved_lt != Type::Unknown {
                            if let Some((idx, _)) = untyped_indices.iter().find(|(_, tv2)| tv2 == tv) {
                                call_info.entry(fname.to_string())
                                    .or_default()
                                    .push((*idx, type_to_string(resolved_lt)));
                            }
                        }
                    }
                    if let Type::Var(tv) = &rt {
                        if !matches!(resolved_rt, Type::Var(_)) && *resolved_rt != Type::Unknown {
                            if let Some((idx, _)) = untyped_indices.iter().find(|(_, tv2)| tv2 == tv) {
                                call_info.entry(fname.to_string())
                                    .or_default()
                                    .push((*idx, type_to_string(resolved_rt)));
                            }
                        }
                    }
                }
                infer_expr_params(left, env, defs, env_cons, call_info, fname, untyped_indices);
                infer_expr_params(right, env, defs, env_cons, call_info, fname, untyped_indices);
            }
            Expr::UnOp { operand, .. } => {
                infer_expr_params(operand, env, defs, env_cons, call_info, fname, untyped_indices);
            }
            Expr::Call { args, .. } => {
                for a in args {
                    infer_expr_params(a, env, defs, env_cons, call_info, fname, untyped_indices);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                infer_expr_params(object, env, defs, env_cons, call_info, fname, untyped_indices);
                for a in args {
                    infer_expr_params(a, env, defs, env_cons, call_info, fname, untyped_indices);
                }
            }
            Expr::FieldAccess { object, .. } => {
                infer_expr_params(object, env, defs, env_cons, call_info, fname, untyped_indices);
            }
            Expr::Array(items) => {
                for it in items {
                    infer_expr_params(it, env, defs, env_cons, call_info, fname, untyped_indices);
                }
            }
            Expr::Range { start, end } => {
                infer_expr_params(start, env, defs, env_cons, call_info, fname, untyped_indices);
                infer_expr_params(end, env, defs, env_cons, call_info, fname, untyped_indices);
            }
            Expr::Await(inner) => {
                infer_expr_params(inner, env, defs, env_cons, call_info, fname, untyped_indices);
            }
            _ => {}
        }
    }
    fn scan_stmts_for_calls(
        stmts: &[Stmt],
        env: &mut HashMap<String, Type>,
        defs: &Defs,
        call_info: &mut HashMap<String, Vec<(usize, String)>>,
    ) {
        for stmt in stmts {
            match stmt {
                Stmt::Expr(e) | Stmt::Return { value: Some(e), .. } => {
                    scan_expr_for_calls(e, env, defs, call_info);
                }
                Stmt::Let { name, value, type_hint, .. } => {
                    scan_expr_for_calls(value, env, defs, call_info);
                    if let Some(th) = type_hint {
                        env.insert(name.clone(), type_from_str(th, defs));
                    } else if let Ok(t) = infer_type(value, env, defs, &HashMap::new()) {
                        if t != Type::Unknown {
                            env.insert(name.clone(), t);
                        }
                    }
                }
                Stmt::Assign { name, value, .. } => {
                    scan_expr_for_calls(value, env, defs, call_info);
                    if let Ok(t) = infer_type(value, env, defs, &HashMap::new()) {
                        env.insert(name.clone(), t);
                    }
                }
                Stmt::If { cond, then_branch, else_branch } => {
                    scan_expr_for_calls(cond, env, defs, call_info);
                    let mut then_env = env.clone();
                    scan_stmts_for_calls(then_branch, &mut then_env, defs, call_info);
                    if let Some(eb) = else_branch {
                        let mut else_env = env.clone();
                        scan_stmts_for_calls(eb, &mut else_env, defs, call_info);
                    }
                }
                Stmt::While { cond, body } => {
                    scan_expr_for_calls(cond, env, defs, call_info);
                    scan_stmts_for_calls(body, env, defs, call_info);
                }
                Stmt::For { var, iterable, body } => {
                    scan_expr_for_calls(iterable, env, defs, call_info);
                    let elem = if let Ok(it_ty) = infer_type(iterable, env, defs, &HashMap::new()) {
                        match &it_ty { Type::List(e) => (**e).clone(), _ => Type::Unknown }
                    } else { Type::Unknown };
                    env.insert(var.clone(), elem);
                    scan_stmts_for_calls(body, env, defs, call_info);
                }
                Stmt::Match { expr, arms } => {
                    scan_expr_for_calls(expr, env, defs, call_info);
                    for (_, arm_body) in arms {
                        let mut arm_env = env.clone();
                        scan_stmts_for_calls(arm_body, &mut arm_env, defs, call_info);
                    }
                }
                _ => {}
            }
        }
    }
    fn scan_expr_for_calls(
        e: &Expr,
        env: &HashMap<String, Type>,
        defs: &Defs,
        call_info: &mut HashMap<String, Vec<(usize, String)>>,
    ) {
        match e {
            Expr::Call { func, args } => {
                let resolved = resolve_pkg_name(defs, func)
                    .unwrap_or_else(|| func.clone());
                if let Some(fdef) = defs.functions.get(&resolved) {
                    if fdef.params.iter().any(|(_, pt)| pt == "_") {
                        for (i, ((_, pt), arg)) in fdef.params.iter().zip(args.iter()).enumerate() {
                            if pt == "_" {
                                if let Ok(at) = infer_type(arg, env, defs, &HashMap::new()) {
                                    if at != Type::Unknown && at != Type::Var("_".to_string()) {
                                        call_info.entry(resolved.clone())
                                            .or_default()
                                            .push((i, type_to_string(&at)));
                                    }
                                }
                            }
                        }
                    }
                }
                for a in args {
                    scan_expr_for_calls(a, env, defs, call_info);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                scan_expr_for_calls(object, env, defs, call_info);
                for a in args {
                    scan_expr_for_calls(a, env, defs, call_info);
                }
            }
            Expr::BinOp { left, right, .. } => {
                scan_expr_for_calls(left, env, defs, call_info);
                scan_expr_for_calls(right, env, defs, call_info);
            }
            Expr::UnOp { operand, .. } => {
                scan_expr_for_calls(operand, env, defs, call_info);
            }
            Expr::FieldAccess { object, .. } => {
                scan_expr_for_calls(object, env, defs, call_info);
            }
            Expr::Array(items) => {
                for it in items {
                    scan_expr_for_calls(it, env, defs, call_info);
                }
            }
            Expr::Range { start, end } => {
                scan_expr_for_calls(start, env, defs, call_info);
                scan_expr_for_calls(end, env, defs, call_info);
            }
            Expr::Await(inner) => {
                scan_expr_for_calls(inner, env, defs, call_info);
            }
            _ => {}
        }
    }
    for (fname, fdef) in defs.functions.iter() {
        let mut env: HashMap<String, Type> = HashMap::new();
        for (pname, ptype) in &fdef.params {
            env.insert(pname.clone(), type_from_str(ptype, defs));
        }
        scan_stmts_for_calls(&fdef.body, &mut env, defs, &mut call_info);
    }
    let mut empty_env: HashMap<String, Type> = HashMap::new();
    scan_stmts_for_calls(stmts, &mut empty_env, defs, &mut call_info);
    let mut updates: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for (fname, entries) in &call_info {
        let mut per_param: HashMap<usize, Vec<&str>> = HashMap::new();
        for (idx, ts) in entries {
            per_param.entry(*idx).or_default().push(ts.as_str());
        }
        for (idx, type_strs) in &per_param {
            let first = type_strs[0];
            let all_same = type_strs.iter().all(|s| *s == first);
            if all_same && first != "_" {
                updates.entry(fname.clone())
                    .or_default()
                    .push((*idx, first.to_string()));
            }
        }
    }
    for (fname, param_updates) in &updates {
        if let Some(fdef) = defs.functions.get_mut(fname) {
            for (idx, ts) in param_updates {
                if *idx < fdef.params.len() {
                    fdef.params[*idx].1 = ts.clone();
                }
            }
        }
    }
    Ok(())
}
fn scan_return_types_env(
    stmts: &[Stmt],
    defs: &Defs,
    env_vars: &mut HashMap<String, Type>,
    env_cons: &HashMap<String, Vec<String>>,
    ret_type: &mut Option<Type>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Return { explicit_type, value } => {
                let t = match (explicit_type, value) {
                    (Some(et), _) => {
                        if value.is_none() { continue; }
                        et.clone()
                    }
                    (_, Some(e)) => {
                        match infer_type(e, env_vars, defs, env_cons) {
                            Ok(t) => t,
                            Err(_) => continue,
                        }
                    }
                    (None, None) => Type::Unit,
                };
                if t == Type::Unknown { continue; }
                match ret_type {
                    Some(prev) if !type_eq(prev, &t) => {
                    }
                    None => *ret_type = Some(t),
                    _ => {}
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                scan_return_types_env(then_branch, defs, env_vars, env_cons, ret_type);
                if let Some(eb) = else_branch {
                    scan_return_types_env(eb, defs, env_vars, env_cons, ret_type);
                }
            }
            Stmt::Expr(e) => {
                match infer_type(e, env_vars, defs, env_cons) {
                    Ok(t) if t != Type::Unknown => {
                        match ret_type {
                            Some(prev) if !type_eq(prev, &t) => {}
                            None => *ret_type = Some(t),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Stmt::Match { arms, .. } => {
                for (_, arm_body) in arms {
                    scan_return_types_env(arm_body, defs, env_vars, env_cons, ret_type);
                }
            }
            Stmt::While { body, .. } => {
                scan_return_types_env(body, defs, env_vars, env_cons, ret_type);
            }
            Stmt::For { body, .. } => {
                scan_return_types_env(body, defs, env_vars, env_cons, ret_type);
            }
            Stmt::Let { name, value, type_hint, .. } => {
                if let Some(th) = type_hint {
                    env_vars.insert(name.clone(), type_from_str(th, defs));
                } else if let Ok(t) = infer_type(value, env_vars, defs, env_cons) {
                    if t != Type::Unknown {
                        env_vars.insert(name.clone(), t);
                    }
                }
            }
            _ => {}
        }
    }
}
pub fn compile_pipeline(
    path: &str,
    mode: CompileMode,
    options: &CompileOptions,
) -> Result<CompileReport, String> {
    let mut report = CompileReport::default();
    let _t_load = StageTimer::new("load+parse+import");
    let LoadedProject {
        mut stmts,
        mut defs,
        cfg,
        edges,
        base_dir,
        stmt_locs,
        file: file_name,
    } = load_target(path)?;
    drop(_t_load);
    if let (Some(cfg), Some(base_dir)) = (cfg.as_ref(), base_dir.as_ref()) {
        if write_lock_file(base_dir, cfg, &edges).is_ok() {
            report.lock_written = true;
        }
        populate_package_cache(cfg, REGISTRY_ROOT);
        report.cache_populated = true;
    }
    {
        let _t = StageTimer::new("resolve_operators_defs");
        resolve_operators_defs(&mut defs);
    }
    {
        let _t = StageTimer::new("infer_return_types");
        infer_function_return_types(&mut defs).map_err(|e| format!("error[type]: {}", e))?;
    }
    {
        let _t = StageTimer::new("check_interface_conformance");
        check_interface_conformance(&defs).map_err(|e| format!("error[type]: {}", e))?;
    }
    {
        let _t = StageTimer::new("resolve_operators_stmts");
        let empty_cons: HashMap<String, Vec<String>> = HashMap::new();
        let empty_env: HashMap<String, Type> = HashMap::new();
        resolve_operators_stmts(&mut stmts, &defs, &empty_cons, &empty_env);
    }
    {
        let _t = StageTimer::new("infer_untyped_params");
        infer_untyped_params(&stmts, &mut defs)?;
        infer_function_return_types(&mut defs)?;
    }
    let loc = match (&stmt_locs, &file_name) {
        (Some(locs), Some(f)) => make_loc_map(&stmts, locs, f),
        _ => LocMap::default(),
    };
    {
        let _t = StageTimer::new("type_check_located");
        type_check_located(&stmts, &mut defs, &loc)?;
    }
    {
        let mut warn_diags: Vec<Diagnostic> = Vec::new();
        collect_warnings(&stmts, &cfg, &loc, &mut warn_diags);
        report.warnings = warn_diags.iter().map(render_diagnostic).collect();
    }
    if mode == CompileMode::Check {
        return Ok(report);
    }
    {
        let _t = StageTimer::new("monomorphize_all");
        monomorphize_all(&mut defs, &mut stmts).map_err(|e| format!("error[type]: {}", e))?;
    }
    if options.optimize {
        report.removed_functions = eliminate_dead_functions(&mut defs, &stmts);
    }
    let memory = {
        let _t = StageTimer::new("memory_analyze");
        memory_analyze(&stmts, &defs).map_err(|e| format!("error[memory]: {}", e))?
    };
    if options.verbose {
        report_memory(&memory);
    }
    let base = path
        .trim_end_matches(".lime")
        .trim_end_matches("citrus.toml")
        .trim_end_matches('/')
        .trim_end_matches('\\');
    let base = if base.is_empty() { "output" } else { base };
    let mut ll_path: Option<String> = None;
    if options.emit_ll || options.emit_object {
        let _t_codegen = StageTimer::new("codegen_ll");
        let (out, warnings) = codegen::emit_llvm(&stmts, &defs, &memory);
        report.codegen_warnings = warnings;
        let ll_path_str = format!("{}.ll", base);
        fs::write(&ll_path_str, &out)
            .map_err(|e| format!("error[codegen]: failed to write {}: {}", ll_path_str, e))?;
        report.emitted_ll = Some(ll_path_str.clone());
        ll_path = Some(ll_path_str);
        let n = report.codegen_warnings.len();
        if n > 0 {
            eprintln!(
                "codegen: {} warning(s): some IR was emitted incompletely",
                n
            );
            if options.verbose {
                for w in &report.codegen_warnings {
                    eprintln!("  codegen warning: {}", w);
                }
            } else {
                eprintln!("  (re-run with --verbose for details)");
            }
            if options.emit_object {
                return Err(format!(
                    "error[codegen]: {} function(s) could not be fully lowered; refusing to emit object file:\n  - {}",
                    report.codegen_warnings.len(),
                    report.codegen_warnings.join("\n  - ")
                ));
            }
        }
    }
    if let Some(ref ll) = ll_path {
        if options.emit_object {
            let opt_level = if options.release { "2" } else { "0" };
            {
                let _t = StageTimer::new("compile_ir");
                let obj_ext = if cfg!(target_os = "windows") { "obj" } else { "o" };
                let obj_path = format!("{}.{}", base, obj_ext);
                let status = std::process::Command::new(llvm_tool("clang"))
                    .arg(&format!("-O{}", opt_level))
                    .arg("-c")
                    .arg(ll)
                    .arg("-o")
                    .arg(&obj_path)
                    .status();
                match status {
                    Ok(s) if s.success() => {
                        report.emitted_obj = Some(obj_path.clone());
                        {
                            let _t_link = StageTimer::new("link");
                            let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
                            let exe_path = format!("{}{}", base, exe_suffix);
                            if cfg!(target_os = "windows") {
                                let runtime_obj = compile_runtime_c()?;
                                let link_result = std::process::Command::new(llvm_tool("lld-link"))
                                    .arg(&obj_path)
                                    .arg(&runtime_obj)
                                    .arg(&format!("/out:{}", exe_path))
                                    .arg("/subsystem:console")
                                    .arg("/defaultlib:libcmt")
                                    .arg("/defaultlib:oldnames")
                                    .status();
                                match link_result {
                                    Ok(s) if s.success() => {
                                        report.emitted_exe = Some(exe_path);
                                    }
                                    _ => {
                                        eprintln!("warning: `lld-link` not found or failed");
                                    }
                                }
                            } else {
                                let link_result = std::process::Command::new(llvm_tool("ld.lld"))
                                    .arg(&obj_path)
                                    .arg("-o")
                                    .arg(&exe_path)
                                    .arg("-lc")
                                    .status();
                                match link_result {
                                    Ok(s) if s.success() => {
                                        report.emitted_exe = Some(exe_path);
                                    }
                                    _ => {
                                        eprintln!("warning: `ld.lld` not found or failed");
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        eprintln!("warning: `clang` not found or failed, no object file produced");
                    }
                }
            }
        }
    }
    if mode == CompileMode::Run {
        if defs.functions.contains_key("main") {
            call_function("main", Vec::new(), &defs)
                .map_err(|e| format!("error[runtime]: {}", e))?;
        } else {
            let mut env = HashMap::new();
            execute_stmts(&stmts, &mut env, &defs)
                .map_err(|e| format!("error[runtime]: {}", e))?;
        }
        report.executed = true;
    }
    Ok(report)
}
fn compile_runtime_c() -> Result<String, String> {
    let tmp_dir = std::env::temp_dir().join("lime_runtime");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let c_path = tmp_dir.join("runtime.c");
    let h_path = tmp_dir.join("runtime.h");
    let obj_path = tmp_dir.join("runtime.obj");
    let need_compile = || -> bool {
        if !obj_path.exists() { return true; }
        let src_modified = std::fs::metadata(&c_path).and_then(|m| m.modified()).ok();
        let obj_modified = std::fs::metadata(&obj_path).and_then(|m| m.modified()).ok();
        match (src_modified, obj_modified) {
            (Some(s), Some(o)) => s > o,
            _ => true,
        }
    };
    if need_compile() {
        std::fs::write(&c_path, RUNTIME_C_SOURCE)
            .map_err(|e| format!("failed to write runtime.c: {}", e))?;
        std::fs::write(&h_path, RUNTIME_H_SOURCE)
            .map_err(|e| format!("failed to write runtime.h: {}", e))?;
        let clang = llvm_tool("clang");
        let status = std::process::Command::new(&clang)
            .arg("-O2")
            .arg("-c")
            .arg(c_path.to_str().unwrap())
            .arg("-o")
            .arg(obj_path.to_str().unwrap())
            .status()
            .map_err(|e| format!("failed to launch clang: {}", e))?;
        if !status.success() {
            return Err("clang compilation of runtime.c failed".to_string());
        }
    }
    Ok(obj_path.to_str().unwrap().to_string())
}
fn llvm_bindir() -> Option<String> {
    for var in &["LLVM_SYS_221_PREFIX", "LIME_LLVM_PREFIX"] {
        if let Ok(prefix) = std::env::var(var) {
            let bindir = std::path::Path::new(&prefix).join("bin");
            if bindir.join("opt.exe").exists() || bindir.join("opt").exists() {
                return Some(bindir.to_str().unwrap().to_string());
            }
        }
    }
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            if dir.join("opt.exe").exists() || dir.join("opt").exists() {
                return Some(dir.to_str().unwrap().to_string());
            }
        }
        for dir in std::env::split_paths(&paths) {
            let llvm_config = dir.join("llvm-config.exe");
            if llvm_config.exists() {
                if let Ok(out) = std::process::Command::new(&llvm_config).arg("--bindir").output() {
                    if let Ok(s) = String::from_utf8(out.stdout) {
                        let bindir = s.trim();
                        if !bindir.is_empty() {
                            return Some(bindir.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
fn llvm_tool(name: &str) -> String {
    if let Some(bindir) = llvm_bindir() {
        let bindir_path = std::path::Path::new(&bindir);
        for candidate in &[name, &format!("{}.exe", name)] {
            let path = bindir_path.join(candidate);
            if path.exists() {
                return path.to_str().unwrap().to_string();
            }
        }
    }
    name.to_string()
}
fn eliminate_dead_functions(defs: &mut Defs, top_stmts: &[Stmt]) -> usize {
    let mut reachable: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut worklist: Vec<String> = Vec::new();
    if defs.functions.contains_key("main") {
        worklist.push("main".to_string());
    }
    let mut root_calls: std::collections::HashSet<String> = std::collections::HashSet::new();
    for s in top_stmts {
        collect_called_names_stmt(s, &mut root_calls);
    }
    for c in root_calls {
        if defs.functions.contains_key(&c) {
            worklist.push(c);
        }
    }
    while let Some(name) = worklist.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(fdef) = defs.functions.get(&name) {
            let mut calls: std::collections::HashSet<String> = std::collections::HashSet::new();
            for s in &fdef.body {
                collect_called_names_stmt(s, &mut calls);
            }
            for c in calls {
                if defs.functions.contains_key(&c) && !reachable.contains(&c) {
                    worklist.push(c);
                }
            }
        }
    }
    if reachable.is_empty() {
        return 0;
    }
    let before = defs.functions.len();
    defs.functions.retain(|name, _| reachable.contains(name));
    before - defs.functions.len()
}
fn collect_called_names_stmt(s: &Stmt, out: &mut std::collections::HashSet<String>) {
    match s {
        Stmt::Let { value, .. } => collect_called_names_expr(value, out),
        Stmt::Assign { value, .. } => collect_called_names_expr(value, out),
        Stmt::Return { explicit_type: _, value: Some(e) } => collect_called_names_expr(e, out),
        Stmt::Return { explicit_type: _, value: None } => {}
        Stmt::Expr(e) => collect_called_names_expr(e, out),
        Stmt::If { cond, then_branch, else_branch } => {
            collect_called_names_expr(cond, out);
            for st in then_branch { collect_called_names_stmt(st, out); }
            if let Some(eb) = else_branch {
                for st in eb { collect_called_names_stmt(st, out); }
            }
        }
        Stmt::While { cond, body } => {
            collect_called_names_expr(cond, out);
            for st in body { collect_called_names_stmt(st, out); }
        }
        Stmt::For { iterable, body, .. } => {
            collect_called_names_expr(iterable, out);
            for st in body { collect_called_names_stmt(st, out); }
        }
        Stmt::Match { expr, arms } => {
            collect_called_names_expr(expr, out);
            for (_, body) in arms {
                for st in body { collect_called_names_stmt(st, out); }
            }
        }
        Stmt::Defer { body } => {
            for st in body { collect_called_names_stmt(st, out); }
        }
        _ => {}
    }
}
fn collect_called_names_expr(e: &Expr, out: &mut std::collections::HashSet<String>) {
    match e {
        Expr::Call { func, args } => {
            out.insert(func.clone());
            for a in args { collect_called_names_expr(a, out); }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_called_names_expr(object, out);
            for a in args { collect_called_names_expr(a, out); }
        }
        Expr::BinOp { left, right, .. } => {
            collect_called_names_expr(left, out);
            collect_called_names_expr(right, out);
        }
        Expr::UnOp { operand, .. } => collect_called_names_expr(operand, out),
        Expr::FieldAccess { object, .. } => collect_called_names_expr(object, out),
        Expr::Array(items) => {
            for it in items { collect_called_names_expr(it, out); }
        }
        Expr::Range { start, end } => {
            collect_called_names_expr(start, out);
            collect_called_names_expr(end, out);
        }
        Expr::Index { target, index } => {
            collect_called_names_expr(target, out);
            collect_called_names_expr(index, out);
        }
        Expr::Slice { target, start, end } => {
            collect_called_names_expr(target, out);
            if let Some(s) = start { collect_called_names_expr(s, out); }
            if let Some(e) = end { collect_called_names_expr(e, out); }
        }
        Expr::Await(inner) => collect_called_names_expr(inner, out),
        _ => {}
    }
}
fn stmt_idents(out: &mut Vec<String>, s: &Stmt) {
    match s {
        Stmt::Let { value, .. } => expr_vars(value, out),
        Stmt::Assign { value, .. } => expr_vars(value, out),
        Stmt::Return { explicit_type: _, value: Some(e) } => expr_vars(e, out),
        Stmt::Return { explicit_type: _, value: None } => {}
        Stmt::Expr(e) => expr_vars(e, out),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
        } => {
            expr_vars(cond, out);
            for st in then_branch {
                stmt_idents(out, st);
            }
            if let Some(eb) = else_branch {
                for st in eb {
                    stmt_idents(out, st);
                }
            }
        }
        Stmt::While { cond, body } => {
            expr_vars(cond, out);
            for st in body {
                stmt_idents(out, st);
            }
        }
        Stmt::For { iterable, body, .. } => {
            expr_vars(iterable, out);
            for st in body {
                stmt_idents(out, st);
            }
        }
        Stmt::Match { expr, arms } => {
            expr_vars(expr, out);
            for (_, body) in arms {
                for st in body {
                    stmt_idents(out, st);
                }
            }
        }
        _ => {}
    }
}
fn ident_used_in_stmt(name: &str, s: &Stmt) -> bool {
    let mut idents = Vec::new();
    stmt_idents(&mut idents, s);
    idents.iter().any(|i| i == name)
}
fn is_terminal(s: &Stmt) -> bool {
    matches!(s, Stmt::Return { .. })
}
fn warn_unused_locals(stmts: &[Stmt], loc: &LocMap, diags: &mut Vec<Diagnostic>) {
    let n = stmts.len();
    for i in 0..n {
        if let Stmt::Let { name, .. } = &stmts[i] {
            let mut used = false;
            for j in (i + 1)..n {
                if ident_used_in_stmt(name, &stmts[j]) {
                    used = true;
                    break;
                }
            }
            if !used {
                if let Some((file, line, col)) = loc.locate(&stmts[i]) {
                    diags.push(Diagnostic::warning(
                        file,
                        line,
                        col,
                        format!("unused variable '{}'", name),
                    ));
                } else {
                    diags.push(Diagnostic::warning_no_pos(format!(
                        "unused variable '{}'",
                        name
                    )));
                }
            }
        }
    }
}
fn warn_unreachable(stmts: &[Stmt], loc: &LocMap, diags: &mut Vec<Diagnostic>) {
    let mut terminal = false;
    for s in stmts {
        if terminal {
            if let Some((file, line, col)) = loc.locate(s) {
                diags.push(Diagnostic::warning(
                    file,
                    line,
                    col,
                    "unreachable code".to_string(),
                ));
            }
        }
        match s {
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                warn_unreachable(then_branch, loc, diags);
                if let Some(eb) = else_branch {
                    warn_unreachable(eb, loc, diags);
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => {
                warn_unreachable(body, loc, diags);
            }
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    warn_unreachable(body, loc, diags);
                }
            }
            _ => {}
        }
        if is_terminal(s) {
            terminal = true;
        }
    }
}
fn collect_warnings(
    stmts: &[Stmt],
    cfg: &Option<CitrusToml>,
    loc: &LocMap,
    diags: &mut Vec<Diagnostic>,
) {
    let mut called: std::collections::HashSet<String> = std::collections::HashSet::new();
    walk_warnings(stmts, cfg, loc, diags, &mut called);
    if let Some(cfg) = cfg {
        for alias in cfg.imports.keys() {
            let used = called
                .iter()
                .any(|c| c == alias || c.starts_with(&format!("{}.", alias)));
            if !used {
                diags.push(Diagnostic::warning_no_pos(format!(
                    "unused import '{}'",
                    alias
                )));
            }
        }
    }
}
fn walk_warnings(
    stmts: &[Stmt],
    cfg: &Option<CitrusToml>,
    loc: &LocMap,
    diags: &mut Vec<Diagnostic>,
    called: &mut std::collections::HashSet<String>,
) {
    warn_unused_locals(stmts, loc, diags);
    warn_unreachable(stmts, loc, diags);
    for s in stmts {
        collect_called_names_stmt(s, called);
        match s {
            Stmt::Fn { body, .. } => walk_warnings(body, cfg, loc, diags, called),
            Stmt::Struct { methods, .. } => {
                for m in methods {
                    if let Stmt::Fn { body, .. } = m {
                        walk_warnings(body, cfg, loc, diags, called);
                    }
                }
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                walk_warnings(then_branch, cfg, loc, diags, called);
                if let Some(eb) = else_branch {
                    walk_warnings(eb, cfg, loc, diags, called);
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => {
                walk_warnings(body, cfg, loc, diags, called);
            }
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    walk_warnings(body, cfg, loc, diags, called);
                }
            }
            _ => {}
        }
    }
}
fn write_lock_file(
    base_dir: &str,
    cfg: &CitrusToml,
    edges: &[(String, String)],
) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Auto-generated by Lime (citrus). Do not edit by hand.\n");
    out.push_str("version = 1\n\n");
    out.push_str("[root]\n");
    out.push_str(&format!("name = \"{}\"\n", cfg.name));
    out.push_str(&format!("version = \"{}\"\n\n", cfg.version));
    let mut pkgs: Vec<(String, String)> = cfg.imports.iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    pkgs.sort();
    for (name, ver) in &pkgs {
        let resolved = read_pkg_version(name, REGISTRY_ROOT);
        out.push_str("[[package]]\n");
        out.push_str(&format!("name = \"{}\"\n", name));
        out.push_str(&format!("requested = \"{}\"\n", ver));
        out.push_str(&format!("resolved = \"{}\"\n", resolved));
        out.push_str(&format!("source = \"{}/{}/{}\"\n\n", REGISTRY_ROOT, name, resolved));
    }
    let mut sorted_edges: Vec<(String, String)> = edges.to_vec();
    sorted_edges.sort();
    sorted_edges.dedup();
    if !sorted_edges.is_empty() {
        out.push_str("[graph]\n");
        for (from, to) in &sorted_edges {
            out.push_str(&format!("edge = [\"{}\", \"{}\"]\n", from, to));
        }
    }
    let lock_path = format!("{}/citrus.lock", base_dir);
    fs::write(&lock_path, out).map_err(|e| format!("{}: {}", lock_path, e))
}
fn populate_package_cache(cfg: &CitrusToml, registry_root: &str) {
    let cache_root = ".citrus/cache";
    for (pkg, _requested) in &cfg.imports {
        let resolved = read_pkg_version(pkg, registry_root);
        let src_toml = format!("{}/{}/{}/citrus.toml", registry_root, pkg, resolved);
        if !std::path::Path::new(&src_toml).exists() {
            continue;
        }
        let dest_dir = format!("{}/{}/{}", cache_root, pkg, resolved);
        let _ = fs::create_dir_all(&dest_dir);
        if let Ok(contents) = fs::read_to_string(&src_toml) {
            let _ = fs::write(format!("{}/citrus.toml", dest_dir), contents);
        }
    }
}
pub fn format_lime_source(source: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut prev_blank = false;
    for raw in source.lines() {
        let trimmed_end = raw.trim_end();
        if trimmed_end.trim().is_empty() {
            if prev_blank {
                continue;
            }
            prev_blank = true;
            out.push(String::new());
            continue;
        }
        prev_blank = false;
        let mut indent_cols = 0usize;
        for ch in trimmed_end.chars() {
            match ch {
                ' ' => indent_cols += 1,
                '\t' => indent_cols += 4,
                _ => break,
            }
        }
        let level = (indent_cols + 2) / 4;
        let content = trimmed_end.trim_start();
        out.push(format!("{}{}", "    ".repeat(level), content));
    }
    while matches!(out.last(), Some(l) if l.is_empty()) {
        out.pop();
    }
    let mut result = out.join("\n");
    result.push('\n');
    result
}
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Array(Vec<Value>),
    Slice(Vec<Value>),
    StringBuilder(String),
    Option(Option<Box<Value>>),
    State {
        name: String,
        values: Vec<Value>,
    },
    Struct {
        name: String,
        fields: Vec<(String, Value)>,
    },
    Future {
        func: String,
        args: Vec<Value>,
    },
    Tuple(Vec<Value>),
}
#[derive(Clone)]
struct FunctionDef {
    type_params: Vec<String>,
    constraints: Vec<(String, String)>,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
    is_async: bool,
}
#[derive(Clone)]
struct StructDef {
    type_params: Vec<String>,
    constraints: Vec<(String, String)>,
    fields: Vec<(String, String)>,
    methods: HashMap<String, FunctionDef>,
}
#[derive(Clone)]
struct InterfaceDef {
    type_params: Vec<String>,
    constraints: Vec<(String, String)>,
    methods: Vec<InterfaceMethod>,
}
#[derive(Clone)]
struct Defs {
    structs: HashMap<String, StructDef>,
    state_variants: HashMap<String, String>,
    variant_fields: HashMap<String, Vec<(String, String)>>,
    states: HashMap<String, Vec<String>>,
    enum_type_params: HashMap<String, Vec<String>>,
    functions: HashMap<String, FunctionDef>,
    interfaces: HashMap<String, InterfaceDef>,
    fn_index: HashMap<Symbol, String>,   
    fn_bare: HashMap<String, Symbol>,    
    type_index: HashMap<Symbol, String>, 
    type_bare: HashMap<String, Symbol>,  
}
impl Defs {
    fn new() -> Self {
        let mut defs = Defs {
            structs: HashMap::new(),
            state_variants: HashMap::new(),
            variant_fields: HashMap::new(),
            states: HashMap::new(),
            enum_type_params: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
            fn_index: HashMap::new(),
            fn_bare: HashMap::new(),
            type_index: HashMap::new(),
            type_bare: HashMap::new(),
        };
        defs.state_variants.insert("Success".to_string(), "Result".to_string());
        defs.state_variants.insert("Error".to_string(), "Result".to_string());
        defs.states.insert(
            "Result".to_string(),
            vec!["Success".to_string(), "Error".to_string()],
        );
        defs.variant_fields.insert("Success".to_string(), vec![("_0".to_string(), "T".to_string())]);
        defs.variant_fields.insert("Error".to_string(), vec![("_0".to_string(), "E".to_string())]);
        defs.enum_type_params.insert("Result".to_string(), vec!["T".to_string(), "E".to_string()]);
        defs.state_variants.insert("Some".to_string(), "Option".to_string());
        defs.state_variants.insert("None".to_string(), "Option".to_string());
        defs.states.insert(
            "Option".to_string(),
            vec!["Some".to_string(), "None".to_string()],
        );
        defs.variant_fields.insert("Some".to_string(), vec![("_0".to_string(), "T".to_string())]);
        defs.variant_fields.insert("None".to_string(), vec![]);
        defs.enum_type_params.insert("Option".to_string(), vec!["T".to_string()]);
        defs
    }
    fn add_function(&mut self, name: String, fdef: FunctionDef) {
        self.add_function_index_only(name.clone());
        self.functions.insert(name, fdef);
    }
    fn add_type(&mut self, name: String) {
        self.add_type_index_only(name.clone());
    }
    fn resolve_function(&self, name: &str) -> Option<String> {
        if is_runtime_builtin(name) {
            return None;
        }
        if self.functions.contains_key(name) {
            return Some(name.to_string());
        }
        let bare = bare_name(name);
        match self.fn_bare.get(&bare) {
            Some(&Symbol::AMBIGUOUS) => None,
            Some(&sym) => self.fn_index.get(&sym).cloned(),
            None => None,
        }
    }
    fn resolve_type(&self, name: &str) -> Option<String> {
        if self.structs.contains_key(name) || self.states.contains_key(name) || self.interfaces.contains_key(name) {
            return Some(name.to_string());
        }
        let bare = bare_name(name);
        match self.type_bare.get(&bare) {
            Some(&Symbol::AMBIGUOUS) => None,
            Some(&sym) => self.type_index.get(&sym).cloned(),
            None => None,
        }
    }
    fn reindex(&mut self) {
        let func_names: Vec<String> = self.functions.keys().cloned().collect();
        let type_names: Vec<String> = self
            .structs
            .keys()
            .chain(self.interfaces.keys())
            .chain(self.states.keys())
            .cloned()
            .collect();
        self.fn_index.clear();
        self.fn_bare.clear();
        self.type_index.clear();
        self.type_bare.clear();
        for name in func_names {
            self.add_function_index_only(name);
        }
        for name in type_names {
            self.add_type_index_only(name);
        }
    }
    fn add_function_index_only(&mut self, name: String) {
        let sym = intern(&name);
        self.fn_index.insert(sym, name.clone());
        let bare = bare_name(&name);
        self.fn_bare
            .entry(bare)
            .and_modify(|e| *e = Symbol::AMBIGUOUS)
            .or_insert(sym);
    }
    fn add_type_index_only(&mut self, name: String) {
        let sym = intern(&name);
        self.type_index.insert(sym, name.clone());
        let bare = bare_name(&name);
        self.type_bare
            .entry(bare)
            .and_modify(|e| *e = Symbol::AMBIGUOUS)
            .or_insert(sym);
    }
}
fn bare_name(name: &str) -> String {
    match name.rfind('.') {
        Some(i) => name[i + 1..].to_string(),
        None => name.to_string(),
    }
}
fn collect_defs(stmts: &[Stmt], defs: &mut Defs) {
    for stmt in stmts {
        match stmt {
            Stmt::Struct {
                name,
                type_params,
                constraints,
                fields,
                methods,
            } => {
                let mut method_map = HashMap::new();
                for m in methods {
                    if let Stmt::Fn {
                        name: mname,
                        type_params: mtp,
                        constraints: mc,
                        params,
                        body,
                        is_async: _,
                    } = m
                    {
                        method_map.insert(
                            mname.clone(),
                            FunctionDef {
                                type_params: mtp.clone(),
                                constraints: mc.clone(),
                                params: params.clone(),
                                return_type: None,
                                body: body.clone(),
                                is_async: false,
                            },
                        );
                    }
                }
                defs.structs.insert(
                    name.clone(),
                    StructDef {
                        type_params: type_params.clone(),
                        constraints: constraints.clone(),
                        fields: fields.clone(),
                        methods: method_map,
                    },
                );
                defs.add_type(name.clone());
            }
            Stmt::Interface {
                name,
                type_params,
                constraints,
                methods,
            } => {
                defs.interfaces.insert(
                    name.clone(),
                    InterfaceDef {
                        type_params: type_params.clone(),
                        constraints: constraints.clone(),
                        methods: methods.clone(),
                    },
                );
                defs.add_type(name.clone());
            }
            Stmt::State {
                name,
                type_params,
                variants,
            } => {
                for v in variants {
                    defs.state_variants.insert(v.clone(), name.clone());
                }
                defs.states.insert(name.clone(), variants.clone());
                defs.add_type(name.clone());
                let _ = type_params;
            }
            Stmt::Enum {
                name,
                type_params,
                variants,
                methods,
            } => {
                if !type_params.is_empty() {
                    defs.enum_type_params.insert(name.clone(), type_params.clone());
                }
                let var_names: Vec<String> = variants.iter().map(|(n, _)| n.clone()).collect();
                for (vname, fields) in variants {
                    defs.state_variants.insert(vname.clone(), name.clone());
                    defs.variant_fields.insert(vname.clone(), fields.clone());
                }
                defs.states.insert(name.clone(), var_names);
                defs.add_type(name.clone());
            }
            Stmt::Fn {
                name,
                type_params,
                constraints,
                params,
                body,
                is_async,
            } => {
                defs.add_function(
                    name.clone(),
                    FunctionDef {
                        type_params: type_params.clone(),
                        constraints: constraints.clone(),
                        params: params.clone(),
                        return_type: None,
                        body: body.clone(),
                        is_async: *is_async,
                    },
                );
                collect_defs(body, defs);
            }
            Stmt::If { then_branch, else_branch, .. } => {
                collect_defs(then_branch, defs);
                if let Some(els) = else_branch {
                    collect_defs(els, defs);
                }
            }
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    collect_defs(body, defs);
                }
            }
            _ => {}
        }
    }
}
impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let strs: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Slice(arr) => {
                let strs: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("Slice[{}]", strs.join(", "))
            }
        Value::StringBuilder(s) => s.clone(),
        Value::Option(opt) => match opt {
            Some(v) => format!("Some({})", v.to_string()),
            None => "None".to_string(),
        },
        Value::State { name, values } => {
                if values.is_empty() {
                    name.clone()
                } else {
                    let strs: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                    format!("{}({})", name, strs.join(", "))
                }
            }
            Value::Struct { name, .. } => format!("{}(...)", name),
            Value::Future { func, .. } => format!("<future {}>", func),
            Value::Tuple(elems) => {
                let strs: Vec<String> = elems.iter().map(|v| v.to_string()).collect();
                format!("({})", strs.join(", "))
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Type {
    Int,
    Long,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Struct(String),
    State(String),
    Interface(String, Vec<Type>),
    List(Box<Type>),
    Slice(Box<Type>),
    Option(Box<Type>),
    Tuple(Vec<Type>),
    Unit,
    Unknown,
    Var(String),
}
#[derive(Debug, Clone)]
struct TypeEnv {
    vars: HashMap<String, Type>,
    constraints: HashMap<String, Vec<String>>,
}
impl TypeEnv {
    fn new() -> Self {
        TypeEnv {
            vars: HashMap::new(),
            constraints: HashMap::new(),
        }
    }
    fn get(&self, name: &str) -> Option<&Type> {
        self.vars.get(name)
    }
    fn insert(&mut self, name: String, ty: Type) {
        self.vars.insert(name, ty);
    }
    fn add_constraint(&mut self, tv: String, iface: String) {
        self.constraints.entry(tv).or_default().push(iface);
    }
}
fn type_from_str(s: &str, defs: &Defs) -> Type {
    if let Some(cached) = global_type_cache().lock().unwrap().get(s) {
        return cached.clone();
    }
    let t = type_from_str_impl(s, defs);
    global_type_cache().lock().unwrap().insert(s.to_string(), t.clone());
    t
}
fn type_from_str_impl(s: &str, defs: &Defs) -> Type {
    if let Some(inner) = s.strip_prefix("Option(") {
        if let Some(inner) = inner.strip_suffix(')') {
            return Type::Option(Box::new(type_from_str(inner, defs)));
        }
    }
    if let Some(inner) = s.strip_suffix('?') {
        return Type::Option(Box::new(type_from_str(inner, defs)));
    }
    if let Some(inner) = s.strip_prefix("List(") {
        if let Some(inner) = inner.strip_suffix(')') {
            return Type::List(Box::new(type_from_str(inner, defs)));
        }
    }
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len()-1];
        if !inner.is_empty() {
            let mut elems = Vec::new();
            let mut depth = 0i32;
            let mut start = 0usize;
            for (i, ch) in inner.char_indices() {
                match ch {
                    '(' | '<' => depth += 1,
                    ')' | '>' => depth -= 1,
                    ',' if depth == 0 => {
                        elems.push(type_from_str(inner[start..i].trim(), defs));
                        start = i + 1;
                    }
                    _ => {}
                }
            }
            elems.push(type_from_str(inner[start..].trim(), defs));
            return Type::Tuple(elems);
        }
    }
    match s {
        "int" | "i32" | "i" => Type::Int,
        "long" | "i64" | "l" => Type::Long,
        "float" | "double" | "f64" | "f" => Type::Float,
        "bool" | "i1" | "b" => Type::Bool,
        "str" | "i8*" | "s" => Type::String,
        "void" | "unit" | "u" => Type::Unit,
        _ => {
            let base = match s.find('(') {
                Some(i) => &s[..i],
                None => s,
            };
            if defs.structs.contains_key(base) {
                Type::Struct(s.to_string())
            } else if defs.states.contains_key(base) {
                Type::State(s.to_string())
            } else if defs.interfaces.contains_key(base) {
                let args = if let Some(i) = s.find('(') {
                    let inner = &s[i + 1..];
                    let inner = inner.strip_suffix(')').unwrap_or(inner);
                    if inner.trim().is_empty() {
                        Vec::new()
                    } else {
                        inner
                            .split(',')
                            .map(|a| type_from_str(a.trim(), defs))
                            .collect()
                    }
                } else {
                    Vec::new()
                };
                Type::Interface(base.to_string(), args)
            } else {
                if let Some(qualified) = defs.resolve_type(base) {
                    type_from_str(&s.replacen(base, &qualified, 1), defs)
                } else {
                    Type::Var(base.to_string())
                }
            }
        }
    }
}
fn substitute_vars_with_unknown(ty: &Type) -> Type {
    match ty {
        Type::Var(_) => Type::Unknown,
        Type::List(e) => Type::List(Box::new(substitute_vars_with_unknown(e))),
        Type::Option(e) => Type::Option(Box::new(substitute_vars_with_unknown(e))),
        Type::Array(e) => Type::Array(Box::new(substitute_vars_with_unknown(e))),
        Type::Struct(s) => Type::Struct(s.clone()),
        Type::Interface(s, a) => Type::Interface(s.clone(), a.clone()),
        Type::State(s) => Type::State(s.clone()),
        other => other.clone(),
    }
}
fn resolve_field_type(state_type: &str, field_type: &str, defs: &Defs) -> Type {
    let concrete_args = generic_args_of(state_type);
    if concrete_args.is_empty() {
        return type_from_str(field_type, defs);
    }
    let base = struct_base(state_type);
    let type_params = match defs.enum_type_params.get(&base) {
        Some(tp) => tp,
        None => return type_from_str(field_type, defs),
    };
    for (i, param) in type_params.iter().enumerate() {
        if field_type.trim() == param.as_str() {
            if let Some(arg) = concrete_args.get(i) {
                return type_from_str(arg, defs);
            }
        }
    }
    let mut result = field_type.to_string();
    for (i, param) in type_params.iter().enumerate() {
        if let Some(arg) = concrete_args.get(i) {
            result = result.replace(param.as_str(), arg);
        }
    }
    type_from_str(&result, defs)
}
fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "str".to_string(),
        Type::Struct(name) => name.clone(),
        Type::State(name) => name.clone(),
        Type::List(inner) => format!("List({})", type_to_string(inner)),
        Type::Option(inner) => format!("Option({})", type_to_string(inner)),
        Type::Interface(name, args) => {
            let args_str: Vec<String> = args.iter().map(type_to_string).collect();
            format!("{}({})", name, args_str.join(", "))
        }
        Type::Var(name) => name.clone(),
        Type::Long => "long".to_string(),
        Type::Unit => "void".to_string(),
        Type::Unknown => "unknown".to_string(),
        Type::Array(inner) => format!("Array({})", type_to_string(inner)),
        Type::Slice(inner) => format!("Slice({})", type_to_string(inner)),
        Type::Tuple(elems) => {
            let strs: Vec<String> = elems.iter().map(type_to_string).collect();
            format!("({})", strs.join(", "))
        }
    }
}
fn type_mismatch_msg(summary: &str, expected: &Type, received: &Type) -> String {
    format!(
        "Type error: {}\n\nexpected:\n    {}\n\nreceived:\n    {}",
        summary,
        type_to_string(expected),
        type_to_string(received)
    )
}
fn type_eq(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Option(inner_a), Type::Option(inner_b)) => type_eq(inner_a, inner_b),
        (Type::List(inner_a), Type::List(inner_b)) => type_eq(inner_a, inner_b),
        (Type::Array(inner_a), Type::Array(inner_b)) => type_eq(inner_a, inner_b),
        (Type::Var(_), _) | (_, Type::Var(_)) => true,
        (Type::Struct(a), Type::Struct(b)) => struct_base_eq(a, b),
        (Type::State(a), Type::State(b)) => struct_base_eq(a, b),
        (Type::Option(inner), Type::State(name)) | (Type::State(name), Type::Option(inner)) => {
            struct_base(name) == "Option" && format!("Option({})", type_to_string(inner)) == *name
        }
        _ => a == b,
    }
}
fn struct_base(name: &str) -> String {
    match name.find('(') {
        Some(i) => name[..i].to_string(),
        None => name.to_string(),
    }
}
fn struct_base_eq(a: &str, b: &str) -> bool {
    let ab = match a.find('(') {
        Some(i) => &a[..i],
        None => a,
    };
    let bb = match b.find('(') {
        Some(i) => &b[..i],
        None => b,
    };
    ab == bb
}
fn generic_args_of(name: &str) -> Vec<String> {
    let inner = match name.find('(') {
        Some(i) => &name[i + 1..],
        None => return Vec::new(),
    };
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner.split(',').map(|a| a.trim().to_string()).collect()
}
fn method_sig_matches(defs: &Defs, mdef: &FunctionDef, im: &InterfaceMethod) -> bool {
    if mdef.params.len() != im.params.len() {
        return false;
    }
    for ((_, mptype), (_, iptype)) in mdef.params.iter().zip(im.params.iter()) {
        let expected = type_from_str(iptype, defs);
        let actual = type_from_str(mptype, defs);
        if !type_eq(&actual, &expected) {
            return false;
        }
    }
    let want_ret = match &im.return_type {
        Some(rt) => type_from_str(rt, defs),
        None => Type::Unit,
    };
    let got_ret = match &mdef.return_type {
        Some(rt) => type_from_str(rt, defs),
        None => Type::Unit,
    };
    type_eq(&got_ret, &want_ret)
}
fn struct_satisfies_interface(defs: &Defs, sname: &str, iface_name: &str) -> bool {
    let sdef = match defs.structs.get(sname) {
        Some(s) => s,
        None => return false,
    };
    let iface = match defs.interfaces.get(iface_name) {
        Some(i) => i,
        None => return false,
    };
    for im in &iface.methods {
        match sdef.methods.get(&im.name) {
            Some(mdef) if method_sig_matches(defs, mdef, im) => {}
            _ => return false,
        }
    }
    true
}
fn subst_type(t: &str, type_params: &[String], arg: &str) -> String {
    let concrete_args = split_top_args(arg);
    if let Some(open) = t.find('(') {
        if let Some(close) = t.rfind(')') {
            if open < close {
                let name = &t[..open];
                let inner = &t[open + 1..close];
                let slots = split_top_args(inner);
                let mut out_slots: Vec<String> = Vec::new();
                for (i, slot) in slots.iter().enumerate() {
                    let slot_trim = slot.trim();
                    let mut replaced = slot_trim.to_string();
                    if let Some(tp) = type_params.get(i) {
                        if slot_trim == tp.as_str() {
                            if let Some(c) = concrete_args.get(i) {
                                replaced = c.clone();
                            }
                        }
                    }
                    out_slots.push(replaced);
                }
                return format!("{}({})", name, out_slots.join(", "));
            }
        }
    }
    if type_params.iter().any(|tp| tp.as_str() == t) {
        if let Some(idx) = type_params.iter().position(|tp| tp.as_str() == t) {
            if let Some(c) = concrete_args.get(idx) {
                return c.clone();
            }
        }
    }
    t.to_string()
}
fn split_top_args(s: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut depth: i32 = 0;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                args.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        args.push(cur.trim().to_string());
    }
    args
}
fn expr_from_path(path: &str) -> Expr {
    let mut parts: Vec<&str> = path.split('.').collect();
    let mut expr = Expr::Ident(parts.remove(0).to_string());
    for p in parts {
        expr = Expr::FieldAccess {
            object: Box::new(expr),
            field: p.to_string(),
        };
    }
    expr
}
fn dispatch_method(
    obj: Value,
    method: &str,
    arg_vals: Vec<Value>,
    env: &HashMap<String, Value>,
    defs: &Defs,
) -> Result<Value, String> {
    match obj {
        Value::StringBuilder(s) => match method {
            "build" => Ok(Value::String(s)),
            other => Err(format!("Unknown StringBuilder method: {}", other)),
        },
        Value::Array(arr) => eval_list_method_ext(&arr, method, &arg_vals),
        Value::String(s) => eval_string_method(&s, method, &arg_vals),
        Value::Float(f) => eval_float_method(f, method, &arg_vals),
        Value::Struct { name, fields } => {
            eval_struct_method_or_call(&name, &fields, method, arg_vals, defs)
        }
        other => Err(format!("Type {:?} has no method '{}'", other, method)),
    }
}
fn eval_struct_method_or_call(
    name: &str,
    fields: &Vec<(String, Value)>,
    method: &str,
    arg_vals: Vec<Value>,
    defs: &Defs,
) -> Result<Value, String> {
    match eval_struct_method(name, fields, method, arg_vals.clone(), defs) {
        Ok(v) => Ok(v),
        Err(e) if e.contains("Unknown method") => call_method(name, fields, method, arg_vals, defs),
        Err(e) => Err(e),
    }
}
fn struct_implements_interface_with(
    defs: &Defs,
    sname: &str,
    iface_name: &str,
    arg: &str,
) -> bool {
    let sdef = match defs.structs.get(sname) {
        Some(s) => s,
        None => return false,
    };
    let iface = match defs.interfaces.get(iface_name) {
        Some(i) => i,
        None => return false,
    };
    for im in &iface.methods {
        let exp_params: Vec<(String, String)> = im
            .params
            .iter()
            .map(|(n, t)| (n.clone(), subst_type(t, &iface.type_params, arg)))
            .collect();
        let exp_ret = im
            .return_type
            .as_ref()
            .map(|t| subst_type(t, &iface.type_params, arg));
        let exp_im = InterfaceMethod {
            name: im.name.clone(),
            params: exp_params,
            return_type: exp_ret,
        };
        match sdef.methods.get(&im.name) {
            Some(mdef) if method_sig_matches(defs, mdef, &exp_im) => {}
            _ => return false,
        }
    }
    true
}
fn resolve_operator_interface(
    defs: &Defs,
    lt: &Type,
    rt: &Type,
    op: &str,
    constraints: &HashMap<String, Vec<String>>,
) -> Option<(String, Type)> {
    if let (Type::Var(a), Type::Var(b)) = (lt, rt) {
        if a == b {
            let iface = match op {
                "+" => "Add",
                "==" | "!=" => "Equal",
                "<" | ">" | "<=" | ">=" => "Compare",
                _ => return None,
            };
            if let Some(ifaces) = constraints.get(a) {
                if ifaces.iter().any(|x| x == iface) {
                    let rty = match op {
                        "==" | "!=" | "<" | ">" | "<=" | ">=" => Type::Bool,
                        _ => Type::Var(a.clone()),
                    };
                    let method = match op {
                        "+" => "add".to_string(),
                        "==" | "!=" => "equal".to_string(),
                        "<" | ">" | "<=" | ">=" => "compare".to_string(),
                        _ => return None,
                    };
                    return Some((method, rty));
                }
            }
            if a == "_" {
                return None;
            }
        }
        return None;
    }
    if lt == &Type::Var("_".to_string()) || rt == &Type::Var("_".to_string()) {
        let concrete = if lt != &Type::Var("_".to_string()) { lt } else { rt };
        return match concrete {
            Type::Struct(n) => {
                let sname = n.clone();
                match op {
                    "+" => {
                        if struct_implements_interface_with(defs, &sname, "Add", &sname) {
                            Some(("add".to_string(), Type::Struct(sname)))
                        } else { None }
                    }
                    "==" | "!=" => {
                        if struct_implements_interface_with(defs, &sname, "Equal", &sname) {
                            Some(("equal".to_string(), Type::Bool))
                        } else { None }
                    }
                    "<" | ">" | "<=" | ">=" => {
                        if struct_implements_interface_with(defs, &sname, "Compare", &sname) {
                            Some(("compare".to_string(), Type::Bool))
                        } else { None }
                    }
                    _ => None,
                }
            }
            _ => None,
        };
    }
    let sname = match (lt, rt) {
        (Type::Struct(a), Type::Struct(b)) if a == b => a.clone(),
        _ => return None,
    };
    match op {
        "+" => {
            if struct_implements_interface_with(defs, &sname, "Add", &sname) {
                Some(("add".to_string(), Type::Struct(sname)))
            } else {
                None
            }
        }
        "==" | "!=" => {
            if struct_implements_interface_with(defs, &sname, "Equal", &sname) {
                Some(("equal".to_string(), Type::Bool))
            } else {
                None
            }
        }
        "<" | ">" | "<=" | ">=" => {
            if struct_implements_interface_with(defs, &sname, "Compare", &sname) {
                Some(("compare".to_string(), Type::Bool))
            } else {
                None
            }
        }
        _ => None,
    }
}
fn resolve_operators_stmts(
    stmts: &mut [Stmt],
    defs: &Defs,
    constraints: &HashMap<String, Vec<String>>,
    initial_env: &HashMap<String, Type>,
) {
    let mut env = initial_env.clone();
    for s in stmts.iter_mut() {
        resolve_operators_stmt(s, defs, &mut env, constraints);
    }
}
fn resolve_operators_defs(defs: &mut Defs) {
    let mut fworks: Vec<(
        String,
        HashMap<String, Vec<String>>,
        HashMap<String, Type>,
        Vec<Stmt>,
    )> = Vec::new();
    for (name, fdef) in defs.functions.iter() {
        let mut cons: HashMap<String, Vec<String>> = HashMap::new();
        for (tv, iface) in &fdef.constraints {
            cons.entry(tv.clone()).or_default().push(iface.clone());
        }
        let mut env: HashMap<String, Type> = HashMap::new();
        for (pname, ptype) in &fdef.params {
            env.insert(pname.clone(), type_from_str(ptype, defs));
        }
        fworks.push((name.clone(), cons, env, fdef.body.clone()));
    }
    for (name, cons, env, mut body) in fworks {
        resolve_operators_stmts(&mut body, defs, &cons, &env);
        if let Some(fdef) = defs.functions.get_mut(&name) {
            fdef.body = body;
        }
    }
    let mut mworks: Vec<(String, String, HashMap<String, Vec<String>>, HashMap<String, Type>, Vec<Stmt>)> =
        Vec::new();
    for (sname, sdef) in defs.structs.iter() {
        for (mname, mdef) in &sdef.methods {
            let mut cons: HashMap<String, Vec<String>> = HashMap::new();
            for (tv, iface) in &mdef.constraints {
                cons.entry(tv.clone()).or_default().push(iface.clone());
            }
            let mut env: HashMap<String, Type> = HashMap::new();
            for (fname, ftype) in &sdef.fields {
                env.insert(fname.clone(), type_from_str(ftype, defs));
            }
            for (pname, ptype) in &mdef.params {
                env.insert(pname.clone(), type_from_str(ptype, defs));
            }
            mworks.push((
                sname.clone(),
                mname.clone(),
                cons,
                env,
                mdef.body.clone(),
            ));
        }
    }
    for (sname, mname, cons, env, mut body) in mworks {
        resolve_operators_stmts(&mut body, defs, &cons, &env);
        if let Some(sdef) = defs.structs.get_mut(&sname) {
            if let Some(mdef) = sdef.methods.get_mut(&mname) {
                mdef.body = body;
            }
        }
    }
}
fn resolve_operators_stmt(
    s: &mut Stmt,
    defs: &Defs,
    env: &mut HashMap<String, Type>,
    constraints: &HashMap<String, Vec<String>>,
) {
    match s {
        Stmt::Expr(e) => resolve_operators_expr(e, defs, env, constraints),
        Stmt::Let { name, value, .. } => {
            if let Ok(t) = infer_type(value, env, defs, constraints) {
                env.insert(name.clone(), t);
            }
            resolve_operators_expr(value, defs, env, constraints);
        }
        Stmt::Assign { value, .. } => resolve_operators_expr(value, defs, env, constraints),
        Stmt::If { cond, then_branch, else_branch } => {
            resolve_operators_expr(cond, defs, env, constraints);
            resolve_operators_stmts(then_branch, defs, constraints, env);
            if let Some(b) = else_branch {
                resolve_operators_stmts(b, defs, constraints, env);
            }
        }
        Stmt::While { cond, body } => {
            resolve_operators_expr(cond, defs, env, constraints);
            resolve_operators_stmts(body, defs, constraints, env);
        }
        Stmt::For { var, iterable, body } => {
            if let Ok(it_ty) = infer_type(iterable, env, defs, constraints) {
                let elem = match &it_ty {
                    Type::List(e) | Type::Slice(e) => (**e).clone(),
                    _ => Type::Unknown,
                };
                env.insert(var.clone(), elem);
            }
            resolve_operators_expr(iterable, defs, env, constraints);
            resolve_operators_stmts(body, defs, constraints, env);
        }
        Stmt::Return { explicit_type: _, value: Some(e) } => resolve_operators_expr(e, defs, env, constraints),
        Stmt::Match { expr, arms, .. } => {
            resolve_operators_expr(expr, defs, env, constraints);
            for (_, body) in arms.iter_mut() {
                resolve_operators_stmts(body, defs, constraints, env);
            }
        }
        Stmt::Fn { params, body, constraints: fc, .. } => {
            let mut fenv = env.clone();
            let mut fcons = constraints.clone();
            for (tv, iface) in fc {
                fcons.entry(tv.clone()).or_default().push(iface.clone());
            }
            for (pname, ptype) in params {
                fenv.insert(pname.clone(), type_from_str(ptype, defs));
            }
            resolve_operators_stmts(body, defs, &fcons, &fenv);
        }
        Stmt::Struct { methods, .. } => {
            resolve_operators_stmts(methods, defs, constraints, env);
        }
        _ => {}
    }
}
fn infer_type(
    e: &Expr,
    env: &HashMap<String, Type>,
    defs: &Defs,
    constraints: &HashMap<String, Vec<String>>,
) -> Result<Type, String> {
    match e {
        Expr::IntLit(_) => Ok(Type::Int),
        Expr::LongLit(_) => Ok(Type::Long),
        Expr::FloatLit(_) => Ok(Type::Float),
        Expr::StringLit(_) => Ok(Type::String),
        Expr::BoolLit(_) => Ok(Type::Bool),
        Expr::Ident(n) => {
            if let Some(t) = env.get(n) {
                Ok(t.clone())
            } else if let Some(state_name) = defs.state_variants.get(n) {
                Ok(Type::State(state_name.clone()))
            } else {
                Err(format!("undefined variable '{}'", n))
            }
        }
        Expr::Call { func, args } => {
            match func.as_str() {
                "print" | "println" => Ok(Type::Unit),
                "len" => Ok(Type::Int),
                "StringBuilder" => Ok(Type::Unknown),
                "int" => Ok(Type::Int),
                "float" => Ok(Type::Float),
                "str" => Ok(Type::String),
                "input" => Ok(Type::String),
                "read_file" => Ok(Type::String),
                "write_file" | "append_file" | "remove_file" | "file_exists" | "fs_exists"
                    | "fs_create_dir" => Ok(Type::Bool),
                "fs_size" => Ok(Type::Int),
                "fs_metadata" => Ok(Type::Struct("fs.FileMetadata".to_string())),
                "fs_list_dir" => Ok(Type::List(Box::new(Type::String))),
                "time_now" => Ok(Type::Float),
                "time_sleep" => Ok(Type::Bool),
                "split" => Ok(Type::List(Box::new(Type::String))),
                "trim" | "slice" | "to_upper" | "to_lower" | "replace" | "repeat"
                    | "contains" | "starts_with" | "ends_with" | "byte_len" => {
                    match func.as_str() {
                        "contains" | "starts_with" | "ends_with" => Ok(Type::Bool),
                        _ => Ok(Type::String),
                    }
                }
                "push" | "reverse" | "remove_at" => {
                    if let Some(first) = args.first() {
                        infer_type(first, env, defs, constraints)
                    } else {
                        Ok(Type::Unknown)
                    }
                }
                "pop" | "first" | "last" => {
                    if let Some(first) = args.first() {
                        let ty = infer_type(first, env, defs, constraints)?;
                        match ty {
                            Type::List(elem) => Ok((*elem).clone()),
                            _ => Ok(Type::Unknown),
                        }
                    } else {
                        Ok(Type::Unknown)
                    }
                }
                "index_of" => Ok(Type::Int),
                "contains_item" => Ok(Type::Bool),
                "abs" | "sqrt" | "pow" => {
                    if args.len() != 1 {
                        return Err(format!("{}() takes exactly 1 argument", func));
                    }
                    let at = infer_type(&args[0], env, defs, constraints)?;
                    if at == Type::Float {
                        Ok(Type::Float)
                    } else {
                        Err(format!("{}() expects a float argument", func))
                    }
                }
                "min" | "max" => {
                    if args.len() != 2 {
                        return Err(format!("{}() takes exactly 2 arguments", func));
                    }
                    let at1 = infer_type(&args[0], env, defs, constraints)?;
                    let at2 = infer_type(&args[1], env, defs, constraints)?;
                    if at1 == Type::Float && at2 == Type::Float {
                        Ok(Type::Float)
                    } else {
                        Err(format!("{}() expects float arguments", func))
                    }
                }
                "clamp" => {
                    if args.len() != 3 {
                        return Err(format!("clamp() takes exactly 3 arguments"));
                    }
                    for a in args {
                        let at = infer_type(a, env, defs, constraints)?;
                        if at != Type::Float {
                            return Err("clamp() expects float arguments".to_string());
                        }
                    }
                    Ok(Type::Float)
                }
                _ => {
                    let resolved = resolve_pkg_name(defs, func)
                        .or_else(|| defs.resolve_type(func))
                        .unwrap_or_else(|| func.clone());
                    if let Some(state_name) = defs.state_variants.get(&resolved) {
                        let state_base = struct_base(state_name);
                        let enum_tp = defs.enum_type_params.get(state_base.as_str());
                        let concrete_state = if let Some(tp) = enum_tp {
                            if !tp.is_empty() {
                                let fields = defs.variant_fields.get(&resolved);
                                let mut concrete_args: Vec<String> = vec!["unknown".to_string(); tp.len()];
                                if let Some(flds) = fields {
                                    for (arg, (_, ftype)) in args.iter().zip(flds.iter()) {
                                        if let Some(pos) = tp.iter().position(|p| p == ftype) {
                                            concrete_args[pos] = type_to_string(&infer_type(arg, env, defs, constraints)?);
                                        }
                                    }
                                }
                                format!("{}({})", state_base, concrete_args.join(","))
                            } else {
                                state_name.clone()
                            }
                        } else {
                            state_name.clone()
                        };
                        if let Some(fields) = defs.variant_fields.get(&resolved) {
                            if args.len() != fields.len() {
                                return Err(format!(
                                    "Type error: variant {} requires {} field(s), got {}",
                                    resolved,
                                    fields.len(),
                                    args.len()
                                ));
                            }
                            for (arg, (_, ftype)) in args.iter().zip(fields.iter()) {
                                let at = infer_type(arg, env, defs, constraints)?;
                                let expected = resolve_field_type(&concrete_state, ftype, defs);
                                if !type_eq(&at, &expected) {
                                    return Err(type_mismatch_msg(
                                        &format!("variant {} argument type mismatch", resolved),
                                        &expected,
                                        &at,
                                    ));
                                }
                            }
                        } else {
                            for a in args {
                                infer_type(a, env, defs, constraints)?;
                            }
                        }
                        Ok(Type::State(concrete_state))
                    } else if defs.structs.contains_key(&resolved) {
                        Ok(Type::Struct(resolved))
                    } else if defs.states.contains_key(&resolved) {
                        Ok(Type::Struct(resolved))
                    } else if let Some(f) = defs.functions.get(&resolved) {
                        match &f.return_type {
                            Some(rt) => {
                                if f.type_params.is_empty() {
                                    Ok(type_from_str(rt, defs))
                                } else {
                                    match infer_generic_args(&resolved, args, env, defs, None) {
                                        Ok(targs) => {
                                            let sub = subst_type(rt, &f.type_params, &targs.join(","));
                                            Ok(type_from_str(&sub, defs))
                                        }
                                        Err(_) => {
                                            let t = type_from_str(rt, defs);
                                            Ok(substitute_vars_with_unknown(&t))
                                        }
                                    }
                                }
                            }
                            None => Ok(Type::Unit),
                        }
                    } else {
                        Ok(Type::Unknown)
                    }
                }
            }
        }
        Expr::MethodCall { object, method, args } => {
            let ot = infer_type(object, env, defs, constraints)?;
            match ot {
                Type::Struct(s) => {
                    let mut tmp_env = TypeEnv::new();
                    for (k, v) in env.iter() {
                        tmp_env.vars.insert(k.clone(), v.clone());
                    }
                    for (k, v) in constraints.iter() {
                        tmp_env.constraints.insert(k.clone(), v.clone());
                    }
                    if let Some(ty) = check_library_struct_method(&s, method, args, &tmp_env, defs) {
                        return ty;
                    }
                    if let Some(sd) = defs.structs.get(&s) {
                        if let Some(m) = sd.methods.get(method) {
                            if let Some(rt) = &m.return_type {
                                return Ok(type_from_str(rt, defs));
                            }
                        }
                    }
                    Ok(Type::Unknown)
                }
                Type::Interface(iface, _) => {
                    if let Some(idef) = defs.interfaces.get(&iface) {
                        if let Some(imsig) = idef.methods.iter().find(|m| m.name == *method) {
                            return Ok(match &imsig.return_type {
                                Some(rt) => type_from_str(rt, defs),
                                None => Type::Unit,
                            });
                        }
                    }
                    Ok(Type::Unknown)
                }
                Type::String => match method.as_str() {
                    "len" | "byte_len" | "length" => Ok(Type::Int),
                    "chars" | "bytes" => Ok(Type::Array(Box::new(Type::String))),
                    "slice" | "trim" | "to_upper" | "to_lower" | "replace"
                        | "repeat" | "read" => Ok(Type::String),
                    "contains" | "starts_with" | "ends_with" | "exists"
                        | "remove" | "write" | "append" => Ok(Type::Bool),
                    "metadata" => Ok(Type::Struct("fs.FileMetadata".to_string())),
                    _ => Ok(Type::Unknown),
                },
                Type::List(elem) => match method.as_str() {
                    "len" | "length" | "size" => Ok(Type::Int),
                    "get" => Ok((*elem).clone()),
                    "set" | "add" => Ok(Type::Unit),
                    "push" | "reverse" => Ok(Type::List(elem)),
                    "pop" | "first" | "last" => Ok((*elem).clone()),
                    "index_of" => Ok(Type::Int),
                    "contains" => Ok(Type::Bool),
                    _ => Ok(Type::Unknown),
                },
                Type::Float => match method.as_str() {
                    "abs" => Ok(Type::Float),
                    "sqrt" => Ok(Type::Float),
                    _ => Ok(Type::Unknown),
                },
                _ => Ok(Type::Unknown),
            }
        }
        Expr::UnOp { operand, .. } => infer_type(operand, env, defs, constraints),
        Expr::BinOp { left, op, right, .. } => {
            let lt = infer_type(left, env, defs, constraints)?;
            let rt = infer_type(right, env, defs, constraints)?;
            if let Some((_, t)) = resolve_operator_interface(defs, &lt, &rt, op, constraints) {
                Ok(t)
            } else {
                match op.as_str() {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "and" | "or" => Ok(Type::Bool),
                    "+" | "-" | "*" | "/" | "%" => {
                        if matches!(&lt, Type::Var(_)) || matches!(&rt, Type::Var(_)) {
                            Ok(Type::Int)
                        } else {
                            Ok(lt)
                        }
                    }
                    _ => Ok(lt),
                }
            }
        }
        Expr::FieldAccess { object, field } => {
            let ot = infer_type(object, env, defs, constraints)?;
            match ot {
                Type::Struct(s) => {
                    if let Some(sd) = defs.structs.get(&s) {
                        for (fn_, ft) in &sd.fields {
                            if fn_ == field {
                                return Ok(type_from_str(ft, defs));
                            }
                        }
                    }
                    Ok(Type::Unknown)
                }
                _ => Ok(Type::Unknown),
            }
        }
        Expr::Array(items) => {
            if let Some(first) = items.first() {
                let et = infer_type(first, env, defs, constraints)?;
                Ok(Type::List(Box::new(et)))
            } else {
                Ok(Type::List(Box::new(Type::Unknown)))
            }
        }
        Expr::Range { .. } => Ok(Type::List(Box::new(Type::Int))),
        Expr::Await(inner) => infer_type(inner, env, defs, constraints),
        Expr::Tuple(elems) => Ok(Type::Tuple(
            elems
                .iter()
                .map(|e| infer_type(e, env, defs, constraints))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Expr::TupleAccess { tuple, index } => {
            let tt = infer_type(tuple, env, defs, constraints)?;
            if let Type::Tuple(ets) = tt {
                if *index < ets.len() {
                    return Ok(ets[*index].clone());
                }
            }
            Ok(Type::Unknown)
        }
        _ => Ok(Type::Unknown),
    }
}
fn resolve_operators_expr(
    e: &mut Expr,
    defs: &Defs,
    env: &HashMap<String, Type>,
    constraints: &HashMap<String, Vec<String>>,
) {
    match e {
        Expr::BinOp { left, right, op, resolved_operator } => {
            resolve_operators_expr(left, defs, env, constraints);
            resolve_operators_expr(right, defs, env, constraints);
            let lt = infer_type(left, env, defs, constraints);
            let rt = infer_type(right, env, defs, constraints);
            let res = match (lt, rt) {
                (Ok(lt), Ok(rt)) => {
                    match resolve_operator_interface(defs, &lt, &rt, op, constraints) {
                        Some((method, _)) => ResolvedOperator::MethodCall {
                            method,
                            op: op.clone(),
                        },
                        None => ResolvedOperator::Builtin,
                    }
                }
                _ => ResolvedOperator::Builtin,
            };
            *resolved_operator = Some(res);
        }
        Expr::UnOp { operand, .. } => resolve_operators_expr(operand, defs, env, constraints),
        Expr::Call { args, .. } => {
            for a in args.iter_mut() {
                resolve_operators_expr(a, defs, env, constraints);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            resolve_operators_expr(object, defs, env, constraints);
            for a in args.iter_mut() {
                resolve_operators_expr(a, defs, env, constraints);
            }
        }
        Expr::FieldAccess { object, .. } => {
            resolve_operators_expr(object, defs, env, constraints)
        }
        Expr::Array(items) => {
            for it in items.iter_mut() {
                resolve_operators_expr(it, defs, env, constraints);
            }
        }
        Expr::Range { start, end } => {
            resolve_operators_expr(start, defs, env, constraints);
            resolve_operators_expr(end, defs, env, constraints);
        }
        Expr::Await(inner) => resolve_operators_expr(inner, defs, env, constraints),
        _ => {}
    }
}
fn check_interface_conformance(defs: &Defs) -> Result<(), String> {
    for (sname, sdef) in &defs.structs {
        for (iface_name, iface) in &defs.interfaces {
            let satisfies = iface
                .methods
                .iter()
                .all(|im| sdef.methods.contains_key(&im.name));
            if !satisfies {
                continue;
            }
            for im in &iface.methods {
                let mdef = sdef.methods.get(&im.name).unwrap();
                if !method_sig_matches(defs, mdef, im) {
                    return Err(format!(
                        "Struct '{}' has method '{}' with a signature that does not match interface '{}'",
                        sname, im.name, iface_name
                    ));
                }
            }
        }
    }
    Ok(())
}
fn struct_implements(defs: &Defs, struct_name: &str, iface_name: &str) -> bool {
    struct_satisfies_interface(defs, struct_name, iface_name)
}
fn type_matches(defs: &Defs, actual: &Type, expected: &Type) -> bool {
    if let (Type::Struct(sname), Type::Interface(iface, _)) = (actual, expected) {
        if struct_implements(defs, sname, iface) {
            return true;
        }
    }
    type_eq(actual, expected)
}
fn check_constraint(
    defs: &Defs,
    constraints: &[(String, String)],
    actual: &Type,
    expected: &Type,
) -> Result<(), String> {
    match (expected, actual) {
        (Type::List(e_exp), Type::List(e_act)) => {
            check_constraint(defs, constraints, e_act, e_exp)
        }
        (Type::Option(e_exp), Type::Option(e_act)) => {
            check_constraint(defs, constraints, e_act, e_exp)
        }
        (Type::Array(e_exp), Type::Array(e_act)) => {
            check_constraint(defs, constraints, e_act, e_exp)
        }
        (Type::Var(tv), concrete) => {
            for (ctv, iface) in constraints {
                if ctv == tv {
                    let ok = match concrete {
                        Type::Struct(sname) => {
                            struct_satisfies_interface(defs, sname, iface)
                        }
                        Type::Interface(iname, _) => iname == iface,
                        Type::Unknown => true,
                        _ => false,
                    };
                    if !ok {
                        return Err(format!(
                            "Type error: type {} does not satisfy constraint '{}: {}'",
                            type_to_string(concrete),
                            tv,
                            iface
                        ));
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn check_library_struct_method(
    struct_name: &str,
    method: &str,
    args: &[Expr],
    env: &TypeEnv,
    _defs: &Defs,
) -> Option<Result<Type, String>> {
    let check_args = |expect: &[(Type, &str)]| -> Option<Result<Type, String>> {
        if args.len() != expect.len() {
            return Some(Err(format!(
                "Type error: {}.{}() expects {} argument(s), got {}",
                struct_name,
                method,
                expect.len(),
                args.len()
            )));
        }
        for (i, (ety, _)) in expect.iter().enumerate() {
            match check_expr(&args[i], env, _defs) {
                Ok(at) if *ety == Type::Unknown || at == *ety => {}
                Ok(at) => {
                    return Some(Err(type_mismatch_msg(
                        &format!("argument {} of {}.{}", i, struct_name, method),
                        ety,
                        &at,
                    )));
                }
                Err(e) => return Some(Err(e)),
            }
        }
        None
    };
    let ret = |ty: Type| -> Option<Result<Type, String>> { Some(Ok(ty)) };
    match (struct_name, method) {
        ("time.Instant", "sleep") => {
            if let Some(e) = check_args(&[(Type::Float, "secs")]) {
                return Some(e);
            }
            ret(Type::Bool)
        }
        ("time.Instant", "elapsed") => {
            if let Some(e) = check_args(&[]) {
                return Some(e);
            }
            ret(Type::Struct("time.Duration".to_string()))
        }
        ("time.Duration", "secs") => {
            if let Some(e) = check_args(&[]) {
                return Some(e);
            }
            ret(Type::Float)
        }
        ("fs.FileMetadata", "size") | ("fs.FileMetadata", "is_dir")
        | ("fs.FileMetadata", "is_file") => {
            if let Some(e) = check_args(&[]) {
                return Some(e);
            }
            ret(Type::Int)
        }
        ("collections.HashMap", "insert") => {
            if let Some(e) = check_args(&[(Type::Unknown, "key"), (Type::Unknown, "value")]) {
                return Some(e);
            }
            ret(Type::Struct(struct_name.to_string()))
        }
        ("collections.HashMap", "get") => {
            if let Some(e) = check_args(&[(Type::Unknown, "key")]) {
                return Some(e);
            }
            ret(Type::Option(Box::new(Type::Unknown)))
        }
        ("collections.HashMap", "contains") => {
            if let Some(e) = check_args(&[(Type::Unknown, "key")]) {
                return Some(e);
            }
            ret(Type::Bool)
        }
        ("collections.HashMap", "length") | ("collections.HashMap", "size") => {
            if let Some(e) = check_args(&[]) {
                return Some(e);
            }
            ret(Type::Int)
        }
        ("collections.HashSet", "add") => {
            if let Some(e) = check_args(&[(Type::Unknown, "item")]) {
                return Some(e);
            }
            ret(Type::Struct(struct_name.to_string()))
        }
        ("collections.HashSet", "contains") => {
            if let Some(e) = check_args(&[(Type::Unknown, "item")]) {
                return Some(e);
            }
            ret(Type::Bool)
        }
        ("collections.HashSet", "length") | ("collections.HashSet", "size") => {
            if let Some(e) = check_args(&[]) {
                return Some(e);
            }
            ret(Type::Int)
        }
        _ => None,
    }
}
fn check_expr(expr: &Expr, env: &TypeEnv, defs: &Defs) -> Result<Type, String> {
    match expr {
        Expr::IntLit(_) => Ok(Type::Int),
        Expr::LongLit(_) => Ok(Type::Long),
        Expr::FloatLit(_) => Ok(Type::Float),
        Expr::StringLit(_) => Ok(Type::String),
        Expr::BoolLit(_) => Ok(Type::Bool),
        Expr::Ident(name) => {
            if let Some(t) = env.get(name) {
                Ok(t.clone())
            } else if let Some(state_name) = defs.state_variants.get(name) {
                Ok(Type::State(state_name.clone()))
            } else {
                Err(format!("Type error: undefined variable '{}'", name))
            }
        }
        Expr::Range { start, end } => {
            let st = check_expr(start, env, defs)?;
            let et = check_expr(end, env, defs)?;
            if st != Type::Int && st != Type::Unknown {
                return Err(format!("Type error: range start must be int (got {:?})", st));
            }
            if et != Type::Int && et != Type::Unknown {
                return Err(format!("Type error: range end must be int (got {:?})", et));
            }
            Ok(Type::List(Box::new(Type::Int)))
        }
        Expr::Array(elements) => {
            let mut elem_ty = Type::Unknown;
            for e in elements {
                let t = check_expr(e, env, defs)?;
                if elem_ty == Type::Unknown {
                    elem_ty = t.clone();
                } else if !type_eq(&elem_ty, &t) {
                    return Err(type_mismatch_msg(
                        "list element type mismatch",
                        &elem_ty,
                        &t,
                    ));
                }
            }
            Ok(Type::List(Box::new(elem_ty)))
        }
        Expr::UnOp { op, operand } => {
            let t = check_expr(operand, env, defs)?;
            match op.as_str() {
                "-" => match t {
                    Type::Int | Type::Float | Type::Unknown => Ok(t),
                    _ => Err(format!("Type error: cannot negate type {:?}", t)),
                },
                "not" => match t {
                    Type::Bool | Type::Unknown => Ok(Type::Bool),
                    _ => Err(format!("Type error: cannot logically negate type {:?}", t)),
                },
                other => Err(format!("Type error: unknown unary operator '{}'", other)),
            }
        }
        Expr::BinOp { left, op, right, .. } => {
            let lt = check_expr(left, env, defs)?;
            let rt = check_expr(right, env, defs)?;
            match op.as_str() {
                "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                    if let Some((_, result_ty)) =
                        resolve_operator_interface(defs, &lt, &rt, op, &env.constraints)
                    {
                        Ok(result_ty)
                    } else if (lt == Type::Int || lt == Type::Long)
                        && (rt == Type::Int || rt == Type::Long)
                    {
                        Ok(Type::Bool)
                    } else if !type_eq(&lt, &rt) {
                        return Err(type_mismatch_msg(
                            "cannot compare values",
                            &lt,
                            &rt,
                        ));
                    } else {
                        Ok(Type::Bool)
                    }
                }
                "and" | "or" => {
                    if (lt != Type::Bool && lt != Type::Unknown)
                        || (rt != Type::Bool && rt != Type::Unknown)
                    {
                        return Err(format!(
                            "Type error: logical operator requires bool (got {:?}, {:?})",
                            lt, rt
                        ));
                    }
                    Ok(Type::Bool)
                }
                "+" | "-" | "*" | "/" | "%" => {
                    if let Some((_, result_ty)) =
                        resolve_operator_interface(defs, &lt, &rt, op, &env.constraints)
                    {
                        Ok(result_ty)
                    } else if lt == Type::Int && rt == Type::Long {
                        Ok(Type::Long)
                    } else if lt == Type::Long && rt == Type::Int {
                        Ok(Type::Long)
                    } else if !type_eq(&lt, &rt) {
                        return Err(type_mismatch_msg(
                            &format!("binary '{}' type mismatch", op),
                            &lt,
                            &rt,
                        ));
                    } else if matches!(&lt, Type::Var(_)) || matches!(&rt, Type::Var(_)) {
                        Ok(Type::Int)
                    } else {
                        Ok(lt)
                    }
                }
                other => Err(format!("Type error: unknown binary operator '{}'", other)),
            }
        }
        Expr::Call { func, args } => {
            match func.as_str() {
                "print" | "println" => {
                    for a in args {
                        check_expr(a, env, defs)?;
                    }
                    Ok(Type::Unit)
                }
                "len" => {
                    if args.len() != 1 {
                        return Err("Type error: len() takes exactly 1 argument".to_string());
                    }
                    check_expr(&args[0], env, defs)?;
                    Ok(Type::Int)
                }
                "StringBuilder" => {
                    if !args.is_empty() {
                        return Err(
                            "Type error: StringBuilder() takes no arguments".to_string()
                        );
                    }
                    Ok(Type::Unknown)
                }
        "int" | "float" | "str" => {
            if args.len() != 1 {
                return Err(format!(
                    "Type error: {}() takes exactly 1 argument",
                    func
                ));
            }
            check_expr(&args[0], env, defs)?;
            match func.as_str() {
                "int" => Ok(Type::Int),
                "float" => Ok(Type::Float),
                "str" => Ok(Type::String),
                _ => Ok(Type::Unknown),
            }
        }
        "input" => {
            for a in args { check_expr(a, env, defs)?; }
            Ok(Type::String)
        }
        "read_file" => {
            for a in args { check_expr(a, env, defs)?; }
            Ok(Type::String)
        }
        "write_file" | "append_file" | "remove_file" | "file_exists" => {
            for a in args { check_expr(a, env, defs)?; }
            Ok(Type::Bool)
        }
        "sqrt" | "abs" => {
            if args.len() != 1 {
                return Err(format!("Type error: {}() takes exactly 1 argument", func));
            }
            let at = check_expr(&args[0], env, defs)?;
            if at == Type::Float || at == Type::Unknown {
                Ok(Type::Float)
            } else {
                Err(format!("Type error: {}() expects a float argument", func))
            }
        }
        "min" | "max" => {
            if args.len() != 2 {
                return Err(format!("Type error: {}() takes exactly 2 arguments", func));
            }
            let at1 = check_expr(&args[0], env, defs)?;
            let at2 = check_expr(&args[1], env, defs)?;
            if (at1 == Type::Float || at1 == Type::Unknown) && (at2 == Type::Float || at2 == Type::Unknown) {
                Ok(Type::Float)
            } else {
                Err(format!("Type error: {}() expects float arguments", func))
            }
        }
        "clamp" => {
            if args.len() != 3 {
                return Err("Type error: clamp() takes exactly 3 arguments".to_string());
            }
            for a in args {
                let at = check_expr(a, env, defs)?;
                if at != Type::Float && at != Type::Unknown {
                    return Err("Type error: clamp() expects float arguments".to_string());
                }
            }
            Ok(Type::Float)
        }
        "pow" => {
            if args.len() != 2 {
                return Err("Type error: pow() takes exactly 2 arguments".to_string());
            }
            let at1 = check_expr(&args[0], env, defs)?;
            let at2 = check_expr(&args[1], env, defs)?;
            if (at1 == Type::Float || at1 == Type::Unknown) && (at2 == Type::Float || at2 == Type::Unknown) {
                Ok(Type::Float)
            } else {
                Err(format!("Type error: pow() expects float arguments"))
            }
        }
        other => {
                    let base = match other.find('(') {
                        Some(i) => &other[..i],
                        None => other,
                    };
                    let resolved = resolve_pkg_name(defs, base).unwrap_or_else(|| base.to_string());
                    if args.is_empty() {
                        if let Some(fdef) = defs.functions.get(&resolved) {
                            if fdef.params.is_empty() {
                                return Ok(match &fdef.return_type {
                                    Some(rt) => type_from_str(rt, defs),
                                    None => Type::Unit,
                                });
                            }
                        }
                    }
                    if let Some(struct_def) = defs.structs.get(&resolved) {
                        if args.len() != struct_def.fields.len() {
                            return Err(format!(
                                "Type error: {} expects {} field(s), got {}",
                                base,
                                struct_def.fields.len(),
                                args.len()
                            ));
                        }
                        for (i, (arg, (fname, ftype))) in
                            args.iter().zip(struct_def.fields.iter()).enumerate()
                        {
                            let at = check_expr(arg, env, defs)?;
                            let expected = type_from_str(ftype, defs);
                            if !type_eq(&at, &expected) {
                                return Err(type_mismatch_msg(
                                    &format!("field '{}' of {} (arg {})", fname, base, i),
                                    &expected,
                                    &at,
                                ));
                            }
                        }
                        return Ok(Type::Struct(resolved.clone()));
                    }
                    if let Some(state_name) = defs.state_variants.get(&resolved) {
                        let state_base = struct_base(state_name);
                        let enum_tp = defs.enum_type_params.get(state_base.as_str());
                        let concrete_state = if let Some(tp) = enum_tp {
                            if !tp.is_empty() {
                                let fields = defs.variant_fields.get(&resolved);
                                let mut concrete_args: Vec<String> = vec!["unknown".to_string(); tp.len()];
                                if let Some(flds) = fields {
                                    for (arg, (_, ftype)) in args.iter().zip(flds.iter()) {
                                        if let Some(pos) = tp.iter().position(|p| p == ftype) {
                                            concrete_args[pos] = type_to_string(&check_expr(arg, env, defs)?);
                                        }
                                    }
                                }
                                format!("{}({})", state_base, concrete_args.join(","))
                            } else {
                                state_name.clone()
                            }
                        } else {
                            state_name.clone()
                        };
                        if let Some(fields) = defs.variant_fields.get(&resolved) {
                            if args.len() != fields.len() {
                                return Err(format!(
                                    "Type error: variant {} requires {} field(s), got {}",
                                    resolved,
                                    fields.len(),
                                    args.len()
                                ));
                            }
                            for (i, (arg, (fname, ftype))) in args.iter().zip(fields.iter()).enumerate() {
                                let at = check_expr(arg, env, defs)?;
                                let expected = resolve_field_type(&concrete_state, ftype, defs);
                                if !type_eq(&at, &expected) {
                                    return Err(type_mismatch_msg(
                                        &format!("variant {} field {} (arg {})", resolved, fname, i),
                                        &expected,
                                        &at,
                                    ));
                                }
                            }
                        } else {
                            for a in args {
                                check_expr(a, env, defs)?;
                            }
                        }
                        return Ok(Type::State(concrete_state));
                    }
                    if let Some(fdef) = defs.functions.get(&resolved) {
                        if args.len() != fdef.params.len() {
                            return Err(format!(
                                "Type error: function {} expects {} argument(s), got {}",
                                base,
                                fdef.params.len(),
                                args.len()
                            ));
                        }
                        for ((pname, ptype), arg) in fdef.params.iter().zip(args.iter()) {
                            let at = check_expr(arg, env, defs)?;
                            let expected = type_from_str(ptype, defs);
                            if !type_matches(defs, &at, &expected) {
                                return Err(type_mismatch_msg(
                                    &format!("argument '{}' of {}", pname, base),
                                    &expected,
                                    &at,
                                ));
                            }
                            if let Err(e) = check_constraint(defs, &fdef.constraints, &at, &expected)
                            {
                                return Err(format!(
                                    "Type error: argument '{}' of {}: {}",
                                    pname, base, e
                                ));
                            }
                        }
                        return Ok(match &fdef.return_type {
                            Some(rt) => type_from_str(rt, defs),
                            None => Type::Unit,
                        });
                    }
                    Err(format!("Type error: unknown function '{}'", func))
                }
            }
        }
        Expr::Tuple(elems) => {
            let mut types = Vec::new();
            for e in elems {
                types.push(check_expr(e, env, defs)?);
            }
            Ok(Type::Tuple(types))
        }
        Expr::TupleAccess { tuple, index } => {
            let t = check_expr(tuple, env, defs)?;
            match t {
                Type::Tuple(elems) => {
                    if *index < elems.len() {
                        Ok(elems[*index].clone())
                    } else {
                        Err(format!(
                            "Type error: tuple index {} out of bounds (len {})",
                            index,
                            elems.len()
                        ))
                    }
                }
                _ => Err(format!("Type error: cannot index non-tuple type {:?}", t)),
            }
        }
        Expr::FieldAccess { object, field } => {
            let obj_ty = check_expr(object, env, defs)?;
            match obj_ty {
                Type::Struct(name) => {
                    if let Some(sdef) = defs.structs.get(&name) {
                        for (fname, ftype) in &sdef.fields {
                            if fname == field {
                                return Ok(type_from_str(ftype, defs));
                            }
                        }
                        return Err(format!(
                            "Type error: unknown field '{}' on struct {}",
                            field, name
                        ));
                    }
                    Ok(Type::Unknown)
                }
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: field access on non-struct type {:?}",
                    other
                )),
            }
        }
        Expr::MethodCall { object, method, args } => {
            let obj_ty = check_expr(object, env, defs)?;
            match obj_ty {
                Type::Struct(name) => {
                    if let Some(ty) = check_library_struct_method(&name, method, args, env, defs) {
                        return ty;
                    }
                    if let Some(sdef) = defs.structs.get(&name) {
                        let mdef = sdef
                            .methods
                            .get(method)
                            .cloned()
                            .ok_or_else(|| {
                                format!(
                                    "Type error: unknown method '{}' on struct {}",
                                    method, name
                                )
                            })?;
                        if args.len() != mdef.params.len() {
                            return Err(format!(
                                "Type error: method {}.{} expects {} argument(s), got {}",
                                name,
                                method,
                                mdef.params.len(),
                                args.len()
                            ));
                        }
                        for ((pname, ptype), arg) in mdef.params.iter().zip(args.iter()) {
                            let at = check_expr(arg, env, defs)?;
                            let expected = type_from_str(ptype, defs);
                            if !type_eq(&at, &expected) {
                                return Err(type_mismatch_msg(
                                    &format!("argument '{}' of {}.{}", pname, name, method),
                                    &expected,
                                    &at,
                                ));
                            }
                        }
                        return Ok(match &mdef.return_type {
                            Some(rt) => type_from_str(rt, defs),
                            None => Type::Unit,
                        });
                    }
                    Ok(Type::Unknown)
                }
                Type::Interface(iface, _) => {
                    let idef = defs.interfaces.get(&iface).ok_or_else(|| {
                        format!("Type error: unknown interface '{}'", iface)
                    })?;
                    let imsig = idef
                        .methods
                        .iter()
                        .find(|m| m.name == *method)
                        .ok_or_else(|| {
                            format!(
                                "Type error: interface '{}' has no method '{}'",
                                iface, method
                            )
                        })?;
                    if args.len() != imsig.params.len() {
                        return Err(format!(
                            "Type error: interface method {}.{} expects {} argument(s), got {}",
                            iface,
                            method,
                            imsig.params.len(),
                            args.len()
                        ));
                    }
                    for ((_pname, ptype), arg) in imsig.params.iter().zip(args.iter()) {
                        let at = check_expr(arg, env, defs)?;
                        let expected = type_from_str(ptype, defs);
                        if !type_eq(&at, &expected) {
                            return Err(type_mismatch_msg(
                                &format!("argument of interface {}.{}", iface, method),
                                &expected,
                                &at,
                            ));
                        }
                    }
                    return Ok(match &imsig.return_type {
                        Some(rt) => type_from_str(rt, defs),
                        None => Type::Unit,
                    });
                }
                Type::String => match method.as_str() {
                    "len" | "byte_len" => {
                        if !args.is_empty() {
                            return Err(format!(
                                "Type error: String.{}() takes no arguments",
                                method
                            ));
                        }
                        Ok(Type::Int)
                    }
                    "chars" | "bytes" => {
                        if !args.is_empty() {
                            return Err(format!(
                                "Type error: String.{}() takes no arguments",
                                method
                            ));
                        }
                        Ok(Type::Array(Box::new(Type::String)))
                    }
                    "slice" => {
                        if args.len() != 2 {
                            return Err(
                                "Type error: String.slice() takes exactly 2 arguments"
                                    .to_string(),
                            );
                        }
                        for a in args {
                            let at = check_expr(a, env, defs)?;
                            if at != Type::Int && at != Type::Unknown {
                                return Err(format!(
                                    "Type error: String.slice() arguments must be int (got {:?})",
                                    at
                                ));
                            }
                        }
                        Ok(Type::String)
                    }
                    "to_upper" | "to_lower" => {
                        if !args.is_empty() {
                            return Err(format!(
                                "Type error: String.{}() takes no arguments",
                                method
                            ));
                        }
                        Ok(Type::String)
                    }
                    "repeat" => {
                        if args.len() != 1 {
                            return Err(
                                "Type error: String.repeat() takes exactly 1 argument".to_string()
                            );
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if at != Type::Int && at != Type::Unknown {
                            return Err(format!(
                                "Type error: String.repeat() expects int, got {:?}",
                                at
                            ));
                        }
                        Ok(Type::String)
                    }
                    "length" => {
                        if !args.is_empty() {
                            return Err(
                                "Type error: String.length() takes no arguments".to_string()
                            );
                        }
                        Ok(Type::Int)
                    }
                    "write" => {
                        if args.len() != 1 {
                            return Err(
                                "Type error: String.write() takes exactly 1 argument".to_string()
                            );
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if at != Type::String && at != Type::Unknown {
                            return Err(format!(
                                "Type error: String.write() expects str, got {:?}",
                                at
                            ));
                        }
                        Ok(Type::Bool)
                    }
                    "exists" | "remove" => {
                        if !args.is_empty() {
                            return Err(format!(
                                "Type error: String.{}() takes no arguments",
                                method
                            ));
                        }
                        Ok(Type::Bool)
                    }
                    "read" => {
                        if !args.is_empty() {
                            return Err(
                                "Type error: String.read() takes no arguments".to_string()
                            );
                        }
                        Ok(Type::String)
                    }
                    "append" => {
                        if args.len() != 1 {
                            return Err(
                                "Type error: String.append() takes exactly 1 argument".to_string()
                            );
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if at != Type::String && at != Type::Unknown {
                            return Err(format!(
                                "Type error: String.append() expects str, got {:?}",
                                at
                            ));
                        }
                        Ok(Type::Bool)
                    }
                    "metadata" => {
                        if !args.is_empty() {
                            return Err(
                                "Type error: String.metadata() takes no arguments".to_string()
                            );
                        }
                        Ok(Type::Struct("fs.FileMetadata".to_string()))
                    }
                    other => Err(format!(
                        "Type error: unknown method '{}' on str",
                        other
                    )),
                },
                Type::List(elem) => match method.as_str() {
                    "len" => {
                        if !args.is_empty() {
                            return Err("Type error: List.len() takes no arguments".to_string());
                        }
                        Ok(Type::Int)
                    }
                    "add" => {
                        if args.len() != 1 {
                            return Err("Type error: List.add() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if !type_eq(&at, &*elem) {
                            return Err(format!(
                                "Type error: List.add() expects {:?}, got {:?}",
                                elem, at
                            ));
                        }
                        Ok(Type::Unit)
                    }
                    "get" => {
                        if args.len() != 1 {
                            return Err("Type error: List.get() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if at != Type::Int && at != Type::Unknown {
                            return Err(format!(
                                "Type error: List.get() index must be int (got {:?})",
                                at
                            ));
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "set" => {
                        if args.len() != 2 {
                            return Err("Type error: List.set() takes exactly 2 arguments".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if at != Type::Int && at != Type::Unknown {
                            return Err(format!(
                                "Type error: List.set() index must be int (got {:?})",
                                at
                            ));
                        }
                        let vt = check_expr(&args[1], env, defs)?;
                        if !type_eq(&vt, &*elem) {
                            return Err(format!(
                                "Type error: List.set() expects {:?}, got {:?}",
                                elem, vt
                            ));
                        }
                        Ok(Type::Unit)
                    }
                    "push" => {
                        if args.len() != 1 {
                            return Err("Type error: List.push() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if !type_eq(&at, &*elem) {
                            return Err(format!(
                                "Type error: List.push() expects {:?}, got {:?}",
                                elem, at
                            ));
                        }
                        Ok(Type::List(elem.clone()))
                    }
                    "pop" => {
                        if !args.is_empty() {
                            return Err("Type error: List.pop() takes no arguments".to_string());
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "first" => {
                        if !args.is_empty() {
                            return Err("Type error: List.first() takes no arguments".to_string());
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "last" => {
                        if !args.is_empty() {
                            return Err("Type error: List.last() takes no arguments".to_string());
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "length" | "size" => {
                        if !args.is_empty() {
                            return Err(format!(
                                "Type error: List.{}() takes no arguments",
                                method
                            ));
                        }
                        Ok(Type::Int)
                    }
                    "index_of" => {
                        if args.len() != 1 {
                            return Err("Type error: List.index_of() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if !type_eq(&at, &*elem) {
                            return Err(format!(
                                "Type error: List.index_of() expects {:?}, got {:?}",
                                elem, at
                            ));
                        }
                        Ok(Type::Int)
                    }
                    "contains" => {
                        if args.len() != 1 {
                            return Err("Type error: List.contains() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if !type_eq(&at, &*elem) {
                            return Err(format!(
                                "Type error: List.contains() expects {:?}, got {:?}",
                                elem, at
                            ));
                        }
                        Ok(Type::Bool)
                    }
                    "reverse" => {
                        if !args.is_empty() {
                            return Err("Type error: List.reverse() takes no arguments".to_string());
                        }
                        Ok(Type::List(elem.clone()))
                    }
                    other => Err(format!(
                        "Type error: unknown method '{}' on List",
                        other
                    )),
                },
                Type::Slice(elem) => match method.as_str() {
                    "len" => {
                        if !args.is_empty() {
                            return Err("Type error: Slice.len() takes no arguments".to_string());
                        }
                        Ok(Type::Int)
                    }
                    "first" => {
                        if !args.is_empty() {
                            return Err("Type error: Slice.first() takes no arguments".to_string());
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "last" => {
                        if !args.is_empty() {
                            return Err("Type error: Slice.last() takes no arguments".to_string());
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "get" => {
                        if args.len() != 1 {
                            return Err("Type error: Slice.get() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if at != Type::Int && at != Type::Unknown {
                            return Err(format!(
                                "Type error: Slice.get() index must be int (got {:?})",
                                at
                            ));
                        }
                        Ok(elem.as_ref().clone())
                    }
                    "contains" => {
                        if args.len() != 1 {
                            return Err("Type error: Slice.contains() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if !type_eq(&at, &*elem) {
                            return Err(format!(
                                "Type error: Slice.contains() expects {:?}, got {:?}",
                                elem, at
                            ));
                        }
                        Ok(Type::Bool)
                    }
                    "index_of" => {
                        if args.len() != 1 {
                            return Err("Type error: Slice.index_of() takes exactly 1 argument".to_string());
                        }
                        let at = check_expr(&args[0], env, defs)?;
                        if !type_eq(&at, &*elem) {
                            return Err(format!(
                                "Type error: Slice.index_of() expects {:?}, got {:?}",
                                elem, at
                            ));
                        }
                        Ok(Type::Int)
                    }
                    "reverse" => {
                        if !args.is_empty() {
                            return Err("Type error: Slice.reverse() takes no arguments".to_string());
                        }
                        Ok(Type::Slice(elem.clone()))
                    }
                    other => Err(format!(
                        "Type error: unknown method '{}' on Slice",
                        other
                    )),
                },
                Type::Float => match method.as_str() {
                    "abs" => {
                        if !args.is_empty() {
                            return Err("Type error: Float.abs() takes no arguments".to_string());
                        }
                        Ok(Type::Float)
                    }
                    "sqrt" => {
                        if !args.is_empty() {
                            return Err("Type error: Float.sqrt() takes no arguments".to_string());
                        }
                        Ok(Type::Float)
                    }
                    other => Err(format!(
                        "Type error: unknown method '{}' on Float",
                        other
                    )),
                },
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: no method '{}' on type {}",
                    method,
                    type_to_string(&other)
                )),
            }
        }
        Expr::Index { target, index } => {
            let tt = check_expr(target, env, defs)?;
            match tt {
                Type::List(elem) | Type::Slice(elem) => {
                    let it = check_expr(index, env, defs)?;
                    if it != Type::Int && it != Type::Unknown {
                        return Err(format!(
                            "Type error: list index must be int (got {})",
                            type_to_string(&it)
                        ));
                    }
                    Ok(*elem)
                }
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: cannot index non-list type {}",
                    type_to_string(&other)
                )),
            }
        }
        Expr::Slice { target, start, end } => {
            let tt = check_expr(target, env, defs)?;
            match tt {
                Type::List(elem) | Type::Slice(elem) => {
                    if let Some(s) = start {
                        let st = check_expr(s, env, defs)?;
                        if st != Type::Int && st != Type::Unknown {
                            return Err(format!(
                                "Type error: slice start must be int (got {})",
                                type_to_string(&st)
                            ));
                        }
                    }
                    if let Some(e) = end {
                        let et = check_expr(e, env, defs)?;
                        if et != Type::Int && et != Type::Unknown {
                            return Err(format!(
                                "Type error: slice end must be int (got {})",
                                type_to_string(&et)
                            ));
                        }
                    }
                    Ok(Type::Slice(elem))
                }
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: cannot slice non-list type {}",
                    type_to_string(&other)
                )),
            }
        }
        Expr::Await(inner) => {
            if let Expr::Call { func, .. } = inner.as_ref() {
                match defs.functions.get(func) {
                    Some(fdef) if fdef.is_async => {
                        if let Some(rt) = &fdef.return_type {
                            Ok(type_from_str(rt, defs))
                        } else {
                            Ok(Type::Unit)
                        }
                    }
                    Some(_) => {
                        return Err(format!(
                            "Type error: await can only be applied to a lime (async) function, but '{}' is a synchronous fn",
                            func
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Type error: await target '{}' is not a known function",
                            func
                        ));
                    }
                }
            } else {
                return Err(
                    "Type error: await can only be applied to a lime function call".to_string(),
                );
            }
        }
    }
}
fn check_stmt(
    stmt: &Stmt,
    env: &mut TypeEnv,
    defs: &Defs,
    expected_return: Option<&Type>,
    is_async: bool,
    loc: &LocMap,
    diags: &mut Vec<Diagnostic>,
) -> Result<(), String> {
    match check_stmt_inner(stmt, env, defs, expected_return, is_async, loc, diags) {
        Ok(()) => Ok(()),
        Err(e) => {
            if !e.is_empty() {
                let hint = suggest_for(&e, env, defs);
                diags.push(loc.diagnostic(stmt, e).with_hint(hint));
            }
            Err(String::new())
        }
    }
}
fn suggest_for(msg: &str, env: &TypeEnv, defs: &Defs) -> Option<String> {
    if let Some(rest) = msg.strip_prefix("Type error: undefined variable '") {
        let name = rest.trim_end_matches('\'');
        let mut cands: Vec<String> = env.vars.keys().cloned().collect();
        cands.extend(defs.functions.keys().cloned());
        cands.extend(defs.structs.keys().cloned());
        cands.extend(defs.states.keys().cloned());
        return nearest(name, cands.iter().map(|s| s.as_str()));
    }
    if let Some(rest) = msg.strip_prefix("undefined variable '") {
        let name = rest.trim_end_matches('\'');
        let mut cands: Vec<String> = env.vars.keys().cloned().collect();
        cands.extend(defs.functions.keys().cloned());
        cands.extend(defs.structs.keys().cloned());
        cands.extend(defs.states.keys().cloned());
        return nearest(name, cands.iter().map(|s| s.as_str()));
    }
    if let Some(rest) = msg.strip_prefix("function '") {
        let name = rest.split('\'').next().unwrap_or("");
        let cands: Vec<&String> = defs.functions.keys().collect();
        let mut all: Vec<String> = cands.iter().map(|s| (*s).clone()).collect();
        all.extend(defs.structs.keys().cloned());
        all.extend(defs.states.keys().cloned());
        return nearest(name, all.iter().map(|s| s.as_str()));
    }
    if let Some(rest) = msg.strip_prefix("Type error: unknown function '") {
        let name = rest.trim_end_matches('\'');
        let mut all: Vec<String> = defs.functions.keys().cloned().collect();
        all.extend(defs.structs.keys().cloned());
        all.extend(defs.states.keys().cloned());
        return nearest(name, all.iter().map(|s| s.as_str()));
    }
    if let Some(rest) = msg.strip_prefix("Type error: unknown field '") {
        let inner = rest.trim_end_matches('\'');
        if let Some((fname, sname)) = inner.split_once("' on struct ") {
            if let Some(st) = defs.structs.get(sname) {
                let cands: Vec<&String> = st.fields.iter().map(|(n, _)| n).collect();
                return nearest(fname, cands.iter().map(|s| s.as_str()));
            }
        }
    }
    if let Some(rest) = msg.strip_prefix("Unknown field: ") {
        let inner = rest.trim_end_matches('\'');
        if let Some((fname, sname)) = inner.split_once(" on struct ") {
            if let Some(st) = defs.structs.get(sname) {
                let cands: Vec<&String> = st.fields.iter().map(|(n, _)| n).collect();
                return nearest(fname, cands.iter().map(|s| s.as_str()));
            }
        }
    }
    None
}
fn check_stmt_inner(
    stmt: &Stmt,
    env: &mut TypeEnv,
    defs: &Defs,
    expected_return: Option<&Type>,
    is_async: bool,
    loc: &LocMap,
    diags: &mut Vec<Diagnostic>,
) -> Result<(), String> {
    match stmt {
        Stmt::Let { name, type_hint, value, .. } => {
            let v_ty = check_expr(value, env, defs)?;
            let declared = match type_hint {
                Some(h) => type_from_str(h, defs),
                None => Type::Unknown,
            };
            if declared != Type::Unknown && !type_eq(&v_ty, &declared) {
                if let (Type::Interface(iface, _), Type::Struct(sname)) = (&declared, &v_ty) {
                    if !struct_implements(defs, sname, iface) {
                        return Err(format!(
                            "Type error: let '{}' expects interface '{}', but struct '{}' does not implement it",
                            name, iface, sname
                        ));
                    }
                } else {
                    return Err(type_mismatch_msg(
                        &format!("let '{}' has an incompatible value type", name),
                        &declared,
                        &v_ty,
                    ));
                }
            }
            let bind_ty = if declared != Type::Unknown {
                declared
            } else {
                v_ty
            };
            env.insert(name.clone(), bind_ty);
            Ok(())
        }
        Stmt::Return { explicit_type, value } => {
            if let Some(et) = explicit_type {
                if let Some(rt) = expected_return {
                    if !type_eq(rt, et) {
                        return Err(format!(
                            "Type error: explicit return type {:?} does not match expected {:?}",
                            et, rt
                        ));
                    }
                }
            }
            match value {
                Some(e) => {
                    let v_ty = check_expr(e, env, defs)?;
                    if let Some(et) = explicit_type {
                        if !type_eq(et, &v_ty) {
                            return Err(format!(
                                "Type error: explicit return type {:?} does not match expression type {:?}",
                                et, v_ty
                            ));
                        }
                    }
                    match expected_return {
                        Some(rt) if *rt != Type::Unknown && !type_eq(rt, &v_ty) => Err(
                            type_mismatch_msg("return type mismatch", rt, &v_ty),
                        ),
                        _ => Ok(()),
                    }
                }
                None => {
                    if explicit_type.is_some() {
                        return Err("Type error: explicit return type requires a value expression".to_string());
                    }
                    Ok(())
                }
            }
        },
        Stmt::If { cond, then_branch, else_branch } => {
            let c_ty = check_expr(cond, env, defs)?;
            if c_ty != Type::Bool && c_ty != Type::Unknown {
                return Err(format!(
                    "Type error: if condition must be bool (got {:?})",
                    c_ty
                ));
            }
            check_stmts(then_branch, env, defs, expected_return, is_async, loc, diags);
            if !diags.is_empty() { return Err(String::new()); }
            if let Some(els) = else_branch {
                check_stmts(els, env, defs, expected_return, is_async, loc, diags);
            if !diags.is_empty() { return Err(String::new()); }
            }
            Ok(())
        }
        Stmt::For { var, iterable, body } => {
            let iter_ty = check_expr(iterable, env, defs)?;
            let elem_ty = match &iter_ty {
                Type::List(elem) => (&**elem).clone(),
                Type::Slice(elem) => (&**elem).clone(),
                Type::Array(elem) => (&**elem).clone(),
                _ => Type::Unknown,
            };
            let mut loop_env = env.clone();
            loop_env.insert(var.clone(), elem_ty);
            check_stmts(body, &mut loop_env, defs, expected_return, is_async, loc, diags);
            if !diags.is_empty() { return Err(String::new()); }
            Ok(())
        }
        Stmt::While { cond, body } => {
            let c_ty = check_expr(cond, env, defs)?;
            if c_ty != Type::Bool && c_ty != Type::Unknown {
                return Err(format!(
                    "Type error: while condition must be bool (got {:?})",
                    c_ty
                ));
            }
            check_stmts(body, env, defs, expected_return, is_async, loc, diags);
            if !diags.is_empty() { return Err(String::new()); }
            Ok(())
        }
        Stmt::Match { expr, arms } => {
            let m_ty = check_expr(expr, env, defs)?;
            if let Type::Tuple(elem_types) = &m_ty {
                let mut has_wildcard = false;
                for (pattern, body) in arms {
                    match pattern {
                        Pattern::Catch => {
                            has_wildcard = true;
                            check_stmts(body, env, defs, expected_return, is_async, loc, diags);
                            if !diags.is_empty() { return Err(String::new()); }
                        }
                        Pattern::Tuple(elems) | Pattern::Try { elems } => {
                            let mut arm_env = env.clone();
                            bind_tuple_pattern(elems, elem_types, defs, &mut arm_env)?;
                            check_stmts(body, &mut arm_env, defs, expected_return, is_async, loc, diags);
                            if !diags.is_empty() { return Err(String::new()); }
                        }
                        other => {
                            return Err(format!(
                                "Type error: expected tuple pattern in match on tuple type, got {:?}",
                                other
                            ));
                        }
                    }
                }
                if !has_wildcard
                    && !arms.iter().any(|(p, _)| matches!(p, Pattern::Tuple(_) | Pattern::Try { .. }))
                {
                    return Err(
                        "Type error: match on tuple is not exhaustive (add a `catch:` arm or a `try (...)` pattern)"
                            .to_string(),
                    );
                }
                return Ok(());
            }
            let (state_name, variants) = if let Type::State(sn) = &m_ty {
                let base = struct_base(sn);
                (sn.clone(), defs.states.get(&base).cloned().unwrap_or_default())
            } else if let Type::Option(inner) = &m_ty {
                let sn = format!("Option({})", type_to_string(inner));
                (sn, vec!["Some".to_string(), "None".to_string()])
            } else {
                (String::new(), Vec::new())
            };
            if !variants.is_empty() {
                let mut covered: Vec<String> = Vec::new();
                let mut has_wildcard = false;
                for (pattern, body) in arms {
                    if matches!(pattern, Pattern::Catch) {
                        has_wildcard = true;
                        check_stmts(body, env, defs, expected_return, is_async, loc, diags);
                        if !diags.is_empty() { return Err(String::new()); }
                        continue;
                    }
                    if let Pattern::Try { elems } = pattern {
                        let pname = "Success";
                        if !variants.contains(&pname.to_string()) {
                            let known: Vec<String> = variants.iter().filter(|v| *v != "Success" && *v != "Error").cloned().collect();
                            return Err(format!(
                                "Type error: unknown variant '{}' for state {}. available variants: {}",
                                pname, state_name,
                                known.join(", ")
                            ));
                        }
                        covered.push(pname.to_string());
                        let mut arm_env = env.clone();
                        let fields = defs.variant_fields.get(pname);
                        let field_types: Vec<Type> = fields
                            .map(|f| f.iter().map(|(_, ft)| resolve_field_type(&state_name, ft, defs)).collect())
                            .unwrap_or_default();
                        bind_tuple_pattern(elems, &field_types, defs, &mut arm_env)?;
                        check_stmts(body, &mut arm_env, defs, expected_return, is_async, loc, diags);
                        if !diags.is_empty() { return Err(String::new()); }
                        continue;
                    }
                    if let Pattern::Error = pattern {
                        let pname = "Error";
                        if !variants.contains(&pname.to_string()) {
                            let known: Vec<String> = variants.iter().filter(|v| *v != "Success" && *v != "Error").cloned().collect();
                            return Err(format!(
                                "Type error: unknown variant '{}' for state {}. available variants: {}",
                                pname, state_name,
                                known.join(", ")
                            ));
                        }
                        covered.push(pname.to_string());
                        let mut arm_env = env.clone();
                        let fields = defs.variant_fields.get(pname);
                        let err_ty = fields
                            .and_then(|f| f.first())
                            .map(|(_, ft)| resolve_field_type(&state_name, ft, defs))
                            .unwrap_or(Type::Unknown);
                        arm_env.insert("error".to_string(), err_ty);
                        check_stmts(body, &mut arm_env, defs, expected_return, is_async, loc, diags);
                        if !diags.is_empty() { return Err(String::new()); }
                        continue;
                    }
                    let (pname, bindings) = match pattern {
                        Pattern::Variant { name, bindings } => (name.clone(), bindings.clone()),
                        Pattern::Tuple(_) => continue,
                        Pattern::Catch | Pattern::Try { .. } | Pattern::Error => unreachable!(),
                    };
                    if !variants.contains(&pname) {
                        let known: Vec<String> = variants.iter().filter(|v| *v != "Success" && *v != "Error").cloned().collect();
                        return Err(format!(
                            "Type error: unknown variant '{}' for state {}. available variants: {}",
                            pname, state_name,
                            known.join(", ")
                        ));
                    }
                    covered.push(pname.clone());
                    let mut arm_env = env.clone();
                    let fields = defs.variant_fields.get(&pname);
                    for (i, b) in bindings.iter().enumerate() {
                        if b != "Ignore" {
                            let ty = fields
                                .and_then(|f| f.get(i))
                                .map(|(_, ft)| resolve_field_type(&state_name, ft, defs))
                                .unwrap_or(Type::Unknown);
                            arm_env.insert(b.clone(), ty);
                        }
                    }
                    check_stmts(body, &mut arm_env, defs, expected_return, is_async, loc, diags);
                    if !diags.is_empty() { return Err(String::new()); }
                }
                if !has_wildcard {
                    for v in &variants {
                        if !covered.contains(v) {
                            return Err(format!(
                                "Type error: match on state {} is not exhaustive (missing variant '{}')",
                                state_name, v
                            ));
                        }
                    }
                }
            } else {
                for (pattern, body) in arms {
                    let mut arm_env = env.clone();
                    let (pname, bindings) = match pattern {
                        Pattern::Variant { name, bindings } => (name.clone(), bindings.clone()),
                        Pattern::Try { elems } => ("Success".to_string(), pattern_binding_names(elems)),
                        Pattern::Error => ("Error".to_string(), vec!["error".to_string()]),
                        Pattern::Catch => (String::new(), vec![]),
                        Pattern::Tuple(elems) => (String::new(), pattern_binding_names(elems)),
                    };
                    let fields = defs.variant_fields.get(&pname);
                    for (i, b) in bindings.iter().enumerate() {
                        if b != "Ignore" {
                            let ty = fields
                                .and_then(|f| f.get(i))
                                .map(|(_, ft)| type_from_str(ft, defs))
                                .unwrap_or(Type::Unknown);
                            arm_env.insert(b.clone(), ty);
                        }
                    }
                    check_stmts(body, &mut arm_env, defs, expected_return, is_async, loc, diags);
                    if !diags.is_empty() { return Err(String::new()); }
                }
            }
            Ok(())
        }
        Stmt::Expr(e) => {
            check_expr(e, env, defs)?;
            Ok(())
        }
        Stmt::Assign { name, value } => {
            let v_ty = check_expr(value, env, defs)?;
            match env.get(name) {
                Some(existing) => {
                    if !type_eq(existing, &v_ty) {
                        return Err(type_mismatch_msg(
                            &format!("cannot assign value to '{}'", name),
                            existing,
                            &v_ty,
                        ));
                    }
                }
                None => {
                    return Err(format!("Type error: assignment to undeclared variable '{}'", name));
                }
            }
            env.insert(name.clone(), v_ty);
            Ok(())
        }
        Stmt::Fn { .. } => Ok(()),
        Stmt::Struct { .. } => Ok(()),
        Stmt::State { .. } => Ok(()),
        Stmt::Defer { body } => {
            check_stmts(body, env, defs, expected_return, is_async, loc, diags);
            if !diags.is_empty() { return Err(String::new()); }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn check_stmts(
    stmts: &[Stmt],
    env: &mut TypeEnv,
    defs: &Defs,
    expected_return: Option<&Type>,
    is_async: bool,
    loc: &LocMap,
    diags: &mut Vec<Diagnostic>,
) {
    for s in stmts {
        let _ = check_stmt(s, env, defs, expected_return, is_async, loc, diags);
    }
}
fn pattern_binding_names(elems: &[Pattern]) -> Vec<String> {
    elems
        .iter()
        .flat_map(|p| match p {
            Pattern::Variant { name, .. } => vec![name.clone()],
            Pattern::Catch => vec![],
            Pattern::Tuple(inner) => pattern_binding_names(inner),
            Pattern::Try { elems } => pattern_binding_names(elems),
            Pattern::Error => vec!["error".to_string()],
        })
        .collect()
}
fn bind_tuple_pattern(
    elems: &[Pattern],
    elem_types: &[Type],
    defs: &Defs,
    env: &mut TypeEnv,
) -> Result<(), String> {
    if elems.len() != elem_types.len() {
        return Err(format!(
            "Type error: tuple pattern size mismatch (expected {}, received {})",
            elem_types.len(),
            elems.len()
        ));
    }
    for (pat, ty) in elems.iter().zip(elem_types.iter()) {
        bind_tuple_element(pat, ty, defs, env)?;
    }
    Ok(())
}
fn bind_tuple_element(
    pat: &Pattern,
    ty: &Type,
    defs: &Defs,
    env: &mut TypeEnv,
) -> Result<(), String> {
    match pat {
        Pattern::Catch => Ok(()),
        Pattern::Variant { name, bindings } if bindings.is_empty() => {
            env.insert(name.clone(), ty.clone());
            Ok(())
        }
        Pattern::Tuple(inner) => match ty {
            Type::Tuple(inner_types) => bind_tuple_pattern(inner, inner_types, defs, env),
            other => Err(format!(
                "Type error: nested tuple pattern does not match element type {}",
                type_to_string(other)
            )),
        },
        other => Err(format!(
            "Type error: unsupported pattern {:?} in tuple match",
            other
        )),
    }
}
fn check_function(
    params: &[(String, String)],
    constraints: &[(String, String)],
    return_type: &Option<String>,
    body: &[Stmt],
    defs: &Defs,
    is_async: bool,
    loc: &LocMap,
    out_inferred: &mut Option<Type>,
) -> Result<(), String> {
    let mut env = TypeEnv::new();
    for (tv, iface) in constraints {
        env.add_constraint(tv.clone(), iface.clone());
    }
    for (pname, ptype) in params {
        env.insert(pname.clone(), type_from_str(ptype, defs));
    }
    let rt = return_type.as_ref().map(|r| type_from_str(r, defs));
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_stmts(body, &mut env, defs, rt.as_ref(), is_async, loc, &mut diags);
    if diags.is_empty() {
        match infer_return_type_from_body(body, &env, defs) {
            Ok(t) => *out_inferred = Some(t),
            Err(e) => *out_inferred = None,
        }
        Ok(())
    } else {
        Err(diags
            .iter()
            .map(render_diagnostic)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
fn infer_return_type_from_body(
    body: &[Stmt],
    env: &TypeEnv,
    defs: &Defs,
) -> Result<Type, String> {
    let mut ret_type: Option<Type> = None;
    scan_return_types(body, env, defs, &mut ret_type)?;
    Ok(ret_type.unwrap_or(Type::Unit))
}
fn scan_return_types(
    stmts: &[Stmt],
    env: &TypeEnv,
    defs: &Defs,
    ret_type: &mut Option<Type>,
) -> Result<(), String> {
    for stmt in stmts {
        match stmt {
            Stmt::Return { explicit_type, value } => {
                let t = match (explicit_type, value) {
                    (Some(et), _) => {
                        if value.is_none() {
                            return Err("Type error: explicit return type requires a value expression".to_string());
                        }
                        et.clone()
                    }
                    (_, Some(e)) => {
                        match infer_type(e, &env.vars, defs, &env.constraints) {
                            Ok(t) => t,
                            Err(_) => continue, 
                        }
                    }
                    (None, None) => Type::Unit,
                };
                match ret_type {
                    Some(existing) if !type_eq(existing, &t) => {
                        let both_unit = *existing == Type::Unit && t == Type::Unit;
                        if both_unit {
                        } else if *existing == Type::Unit || t == Type::Unit {
                            return Err(format!(
                                "Type error: cannot mix void and {} return",
                                type_to_string(if *existing == Type::Unit { &t } else { existing })
                            ));
                        } else {
                            return Err(type_mismatch_msg(
                                "return type mismatch",
                                existing,
                                &t,
                            ));
                        }
                    }
                    None => *ret_type = Some(t),
                    _ => {}
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                scan_return_types(then_branch, env, defs, ret_type)?;
                if let Some(eb) = else_branch {
                    scan_return_types(eb, env, defs, ret_type)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum DiagnosticLevel {
    Error,
    Warning,
}
struct Diagnostic {
    level: DiagnosticLevel,
    file: String,
    line: usize,
    col: usize,
    message: String,
    has_position: bool,
    hint: Option<String>,
}
impl Diagnostic {
    fn error(file: String, line: usize, col: usize, message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Error,
            file,
            line,
            col,
            message,
            has_position: true,
            hint: None,
        }
    }
    fn error_no_pos(message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Error,
            file: String::new(),
            line: 0,
            col: 0,
            message,
            has_position: false,
            hint: None,
        }
    }
    fn warning(file: String, line: usize, col: usize, message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Warning,
            file,
            line,
            col,
            message,
            has_position: true,
            hint: None,
        }
    }
    fn warning_no_pos(message: String) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Warning,
            file: String::new(),
            line: 0,
            col: 0,
            message,
            has_position: false,
            hint: None,
        }
    }
    fn with_hint(mut self, hint: Option<String>) -> Diagnostic {
        self.hint = hint;
        self
    }
}
fn render_diagnostic(d: &Diagnostic) -> String {
    let tag = match d.level {
        DiagnosticLevel::Error => "error[type]",
        DiagnosticLevel::Warning => "warning[type]",
    };
    let base = if d.has_position {
        format!("{} {}:{}:{}\n{}", tag, d.file, d.line, d.col, d.message)
    } else if d.level == DiagnosticLevel::Warning {
        format!("warning: {}", d.message)
    } else {
        d.message.clone()
    };
    match &d.hint {
        Some(s) => format!("{}\n  did you mean '{}'?", base, s),
        None => base,
    }
}
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr: Vec<usize> = vec![0; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}
fn nearest<I, T>(name: &str, candidates: I) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let threshold = (name.chars().count() / 3).max(1);
    let mut best: Option<(usize, String)> = None;
    for c in candidates {
        let cs = c.as_ref();
        if cs == name {
            continue;
        }
        let d = levenshtein(name, cs);
        if d <= threshold {
            match &best {
                Some((bd, _)) if *bd <= d => {}
                _ => best = Some((d, cs.to_string())),
            }
        }
    }
    best.map(|(_, s)| s)
}
#[derive(Default)]
struct LocMap {
    by_addr: HashMap<usize, (usize, usize)>,
    file: String,
}
impl LocMap {
    fn loc_of(&self, stmt: &Stmt) -> Option<(usize, usize)> {
        self.by_addr.get(&(stmt as *const Stmt as usize)).copied()
    }
    fn locate(&self, stmt: &Stmt) -> Option<(String, usize, usize)> {
        self.loc_of(stmt)
            .map(|(line, col)| (self.file.clone(), line, col))
    }
    fn diagnostic(&self, stmt: &Stmt, msg: String) -> Diagnostic {
        match self.loc_of(stmt) {
            Some((line, col)) => Diagnostic::error(self.file.clone(), line, col, msg),
            None => Diagnostic::error_no_pos(msg),
        }
    }
}
fn build_loc_map(
    stmts: &[Stmt],
    locs: &[(usize, usize)],
    idx: &mut usize,
    map: &mut HashMap<usize, (usize, usize)>,
) {
    for s in stmts {
        if let Some(pos) = locs.get(*idx) {
            map.insert(s as *const Stmt as usize, *pos);
        }
        *idx += 1;
        match s {
            Stmt::Fn { body, .. } => build_loc_map(body, locs, idx, map),
            Stmt::Struct { methods, .. } => {
                for m in methods {
                    if let Stmt::Fn { body, .. } = m {
                        build_loc_map(body, locs, idx, map);
                    }
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                build_loc_map(then_branch, locs, idx, map);
                if let Some(eb) = else_branch {
                    build_loc_map(eb, locs, idx, map);
                }
            }
            Stmt::For { body, .. } => build_loc_map(body, locs, idx, map),
            Stmt::While { body, .. } => build_loc_map(body, locs, idx, map),
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    build_loc_map(body, locs, idx, map);
                }
            }
            Stmt::Defer { body } => build_loc_map(body, locs, idx, map),
            _ => {}
        }
    }
}
fn make_loc_map(stmts: &[Stmt], locs: &[(usize, usize)], file: &str) -> LocMap {
    let mut by_addr = HashMap::new();
    let mut idx = 0usize;
    build_loc_map(stmts, locs, &mut idx, &mut by_addr);
    LocMap { by_addr, file: file.to_string() }
}
fn type_check(stmts: &[Stmt], defs: &mut Defs) -> Result<(), String> {
    type_check_located(stmts, defs, &LocMap::default())
}
fn type_check_located(stmts: &[Stmt], defs: &mut Defs, loc: &LocMap) -> Result<(), String> {
    let mut diags: Vec<Diagnostic> = Vec::new();
    let mut top_env = TypeEnv::new();
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                name,
                type_params,
                constraints,
                params,
                body,
                is_async,
            } => {
                let _ = type_params;
                let mut inferred = None;
                if let Err(e) = check_function(params, constraints, &None, body, defs, *is_async, loc, &mut inferred) {
                    diags.push(Diagnostic::error_no_pos(format!(
                        "In function '{}': {}",
                        name, e
                    )));
                }
                if let Some(t) = inferred {
                    if let Some(fdef) = defs.functions.get_mut(name) {
                        fdef.return_type = Some(type_to_string(&t));
                    }
                }
            }
            Stmt::Struct {
                name,
                type_params,
                constraints,
                fields,
                methods,
            } => {
                let mut env = TypeEnv::new();
                for (tv, iface) in constraints {
                    env.add_constraint(tv.clone(), iface.clone());
                }
                for (fname, ftype) in fields {
                    env.insert(fname.clone(), type_from_str(ftype, defs));
                }
                for m in methods {
                    if let Stmt::Fn {
                        name: mname,
                        type_params: _,
                        constraints: mc,
                        params,
                        body,
                        is_async: _,
                    } = m
                    {
                        let mut menv = env.clone();
                        for (tv, iface) in mc {
                            menv.add_constraint(tv.clone(), iface.clone());
                        }
                        for (pname, ptype) in params {
                            menv.insert(pname.clone(), type_from_str(ptype, defs));
                        }
                        let mut mdiags: Vec<Diagnostic> = Vec::new();
                        check_stmts(body, &mut menv, defs, None, false, loc, &mut mdiags);
                        for d in mdiags {
                            diags.push(Diagnostic::error_no_pos(format!(
                                "In method '{}.{}': {}",
                                name, mname, render_diagnostic(&d)
                            )));
                        }
                    }
                }
            }
            Stmt::Let { name, value, .. } => {
                let _ = check_expr(value, &top_env, defs);
                let v_ty = infer_type(value, &top_env.vars, defs, &top_env.constraints);
                let mut env = top_env.clone();
                if let Ok(t) = &v_ty {
                    env.insert(name.clone(), t.clone());
                }
                let _ = check_stmt(stmt, &mut env, defs, None, false, loc, &mut diags);
                if let Ok(t) = &v_ty {
                    top_env.insert(name.clone(), t.clone());
                }
            }
            Stmt::If { .. }
            | Stmt::Match { .. }
            | Stmt::Return { .. }
            | Stmt::Expr(_)
            | Stmt::Assign { .. } => {
                let mut env = top_env.clone();
                let _ = check_stmt(stmt, &mut env, defs, None, false, loc, &mut diags);
            }
            _ => {}
        }
    }
    if diags.is_empty() {
        Ok(())
    } else {
        Err(diags
            .iter()
            .map(render_diagnostic)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
fn expr_vars(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Ident(n) => out.push(n.clone()),
        Expr::BinOp { left, right, .. } => {
            expr_vars(left, out);
            expr_vars(right, out);
        }
        Expr::UnOp { operand, .. } => expr_vars(operand, out),
        Expr::Call { args, .. } => {
            for a in args {
                expr_vars(a, out);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            expr_vars(object, out);
            for a in args {
                expr_vars(a, out);
            }
        }
        Expr::FieldAccess { object, .. } => expr_vars(object, out),
        Expr::Array(items) => {
            for it in items {
                expr_vars(it, out);
            }
        }
        Expr::Range { start, end } => {
            expr_vars(start, out);
            expr_vars(end, out);
        }
        Expr::Index { target, index } => {
            expr_vars(target, out);
            expr_vars(index, out);
        }
        Expr::Slice { target, start, end } => {
            expr_vars(target, out);
            if let Some(s) = start { expr_vars(s, out); }
            if let Some(e) = end { expr_vars(e, out); }
        }
        Expr::Await(inner) => expr_vars(inner, out),
        _ => {}
    }
}
fn collect_escape_seeds(stmts: &[Stmt], seeds: &mut Vec<String>) {
    for s in stmts {
        match s {
            Stmt::Return { explicit_type: _, value: Some(e) } => expr_vars(e, seeds),
            Stmt::Expr(Expr::Await(inner)) => {
                if let Expr::Call { args, .. } = inner.as_ref() {
                    for a in args {
                        expr_vars(a, seeds);
                    }
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                collect_escape_seeds(then_branch, seeds);
                if let Some(els) = else_branch {
                    collect_escape_seeds(els, seeds);
                }
            }
            Stmt::While { body, .. } => collect_escape_seeds(body, seeds),
            Stmt::For { body, .. } => collect_escape_seeds(body, seeds),
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    collect_escape_seeds(body, seeds);
                }
            }
            Stmt::Defer { body } => collect_escape_seeds(body, seeds),
            _ => {}
        }
    }
}
fn collect_sources(stmts: &[Stmt], sources: &mut HashMap<String, Vec<String>>) {
    for s in stmts {
        match s {
            Stmt::Let { name, value, .. } => {
                let mut vs = Vec::new();
                expr_vars(value, &mut vs);
                sources.insert(name.clone(), vs);
            }
            Stmt::Assign { name, value } => {
                let mut vs = Vec::new();
                expr_vars(value, &mut vs);
                sources.insert(name.clone(), vs);
            }
            Stmt::If { then_branch, else_branch, .. } => {
                collect_sources(then_branch, sources);
                if let Some(els) = else_branch {
                    collect_sources(els, sources);
                }
            }
            Stmt::While { body, .. } => collect_sources(body, sources),
            Stmt::For { body, .. } => collect_sources(body, sources),
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    collect_sources(body, sources);
                }
            }
            Stmt::Defer { body } => collect_sources(body, sources),
            _ => {}
        }
    }
}
fn escapes(v: &str, escaping: &mut Vec<String>, sources: &HashMap<String, Vec<String>>) {
    if escaping.contains(&v.to_string()) {
        return;
    }
    escaping.push(v.to_string());
    if let Some(srcs) = sources.get(v) {
        for s in srcs {
            escapes(s, escaping, sources);
        }
    }
}
fn async_escapes(stmts: &[Stmt], is_async: bool) -> Vec<String> {
    if !is_async {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut after_await = false;
    for s in stmts {
        if stmt_has_await(s) {
            after_await = true;
            continue;
        }
        if after_await {
            let mut vs = Vec::new();
            stmt_vars(s, &mut vs);
            for v in vs {
                if !result.contains(&v) {
                    result.push(v);
                }
            }
        }
    }
    result
}
fn stmt_has_await(s: &Stmt) -> bool {
    match s {
        Stmt::Expr(Expr::Await(_)) => true,
        Stmt::Let { value, .. } => expr_has_await(value),
        Stmt::Return { explicit_type: _, value: Some(e) } => expr_has_await(e),
        Stmt::If { then_branch, else_branch, .. } => {
            then_branch.iter().any(stmt_has_await)
                || else_branch.as_ref().map(|b| b.iter().any(stmt_has_await)).unwrap_or(false)
        }
        Stmt::While { body, .. } => body.iter().any(stmt_has_await),
        Stmt::For { body, .. } => body.iter().any(stmt_has_await),
        Stmt::Match { arms, .. } => arms.iter().any(|(_, b)| b.iter().any(stmt_has_await)),
        _ => false,
    }
}
fn expr_has_await(e: &Expr) -> bool {
    match e {
        Expr::Await(_) => true,
        Expr::BinOp { left, right, .. } => expr_has_await(left) || expr_has_await(right),
        Expr::UnOp { operand, .. } => expr_has_await(operand),
        Expr::Call { args, .. } => args.iter().any(expr_has_await),
        Expr::MethodCall { object, args, .. } => {
            expr_has_await(object) || args.iter().any(expr_has_await)
        }
        Expr::Await(inner) => expr_has_await(inner),
        _ => false,
    }
}
fn stmt_vars(s: &Stmt, out: &mut Vec<String>) {
    match s {
        Stmt::Let { name, value, .. } => {
            out.push(name.clone());
            expr_vars(value, out);
        }
        Stmt::Assign { name, value } => {
            out.push(name.clone());
            expr_vars(value, out);
        }
        Stmt::Return { explicit_type: _, value: Some(e) } => expr_vars(e, out),
        Stmt::Expr(e) => expr_vars(e, out),
        Stmt::If { cond, then_branch, else_branch } => {
            expr_vars(cond, out);
            for b in then_branch {
                stmt_vars(b, out);
            }
            if let Some(els) = else_branch {
                for b in els {
                    stmt_vars(b, out);
                }
            }
        }
        Stmt::While { cond, body } => {
            expr_vars(cond, out);
            for b in body {
                stmt_vars(b, out);
            }
        }
        Stmt::For { var, iterable, body } => {
            out.push(var.clone());
            expr_vars(iterable, out);
            for b in body {
                stmt_vars(b, out);
            }
        }
        Stmt::Match { expr, arms } => {
            expr_vars(expr, out);
            for (_, body) in arms {
                for b in body {
                    stmt_vars(b, out);
                }
            }
        }
        _ => {}
    }
}
fn analyze_block(
    stmts: &[Stmt],
    is_async: bool,
    defs: &Defs,
    report: &mut Vec<(String, MemoryPlace)>,
) -> Result<(), String> {
    let mut seeds = Vec::new();
    collect_escape_seeds(stmts, &mut seeds);
    let mut sources: HashMap<String, Vec<String>> = HashMap::new();
    collect_sources(stmts, &mut sources);
    let mut escaping: Vec<String> = Vec::new();
    for s in &seeds {
        escapes(s, &mut escaping, &sources);
    }
    for v in async_escapes(stmts, is_async) {
        if !escaping.contains(&v) {
            escaping.push(v);
        }
    }
    for s in stmts {
        match s {
            Stmt::Let {
                name,
                place,
                ..
            } => {
                let decision = match place {
                    Some(MemoryPlace::Heap) => MemoryPlace::Heap,
                    Some(MemoryPlace::Stack) => {
                        if escaping.contains(name) {
                            return Err(format!(
                                "Memory error: '{}' is explicitly placed on the stack but escapes (returned, passed to a call, or held across await)",
                                name
                            ));
                        }
                        MemoryPlace::Stack
                    }
                    None => {
                        if escaping.contains(name) {
                            MemoryPlace::Heap
                        } else {
                            MemoryPlace::Stack
                        }
                    }
                };
                report.push((name.clone(), decision));
            }
            Stmt::Fn {
                name: fname,
                body,
                is_async: fasync,
                ..
            } => {
                analyze_block(body, *fasync, defs, report)?;
                let _ = fname;
            }
            Stmt::If { then_branch, else_branch, .. } => {
                analyze_block(then_branch, is_async, defs, report)?;
                if let Some(els) = else_branch {
                    analyze_block(els, is_async, defs, report)?;
                }
            }
            Stmt::While { body, .. } => analyze_block(body, is_async, defs, report)?,
            Stmt::For { body, .. } => analyze_block(body, is_async, defs, report)?,
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    analyze_block(body, is_async, defs, report)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
fn mangled_name(base: &str, type_args: &[String]) -> String {
    format!("{}.{}", base, type_args.join("."))
}
fn parse_generic_call_name(func: &str) -> Option<(&str, Vec<&str>)> {
    if let Some(paren_idx) = func.find('(') {
        let base = &func[..paren_idx];
        let inner = &func[paren_idx + 1..func.len() - 1];
        let args: Vec<&str> = if inner.is_empty() {
            Vec::new()
        } else {
            inner.split(", ").collect()
        };
        Some((base, args))
    } else {
        None
    }
}
fn monomorphize_type_str(t: &str, type_params: &[String], type_args: &[&str]) -> String {
    let mut result = t.to_string();
    for (i, tp) in type_params.iter().enumerate() {
        if i < type_args.len() {
            result = result.replace(tp.as_str(), type_args[i]);
        }
    }
    result
}
fn monomorphize_function(fdef: &FunctionDef, type_params: &[String], type_args: &[&str]) -> FunctionDef {
    let params: Vec<(String, String)> = fdef.params.iter()
        .map(|(n, t)| (n.clone(), monomorphize_type_str(t, type_params, type_args)))
        .collect();
    let return_type = fdef.return_type.as_ref()
        .map(|rt| monomorphize_type_str(rt, type_params, type_args));
    FunctionDef {
        type_params: Vec::new(),
        constraints: Vec::new(),
        params,
        return_type,
        body: fdef.body.clone(),
        is_async: fdef.is_async,
    }
}
fn infer_generic_args(
    func_name: &str,
    call_args: &[Expr],
    env: &HashMap<String, Type>,
    defs: &Defs,
    expected: Option<&Type>,
) -> Result<Vec<String>, String> {
    let fdef = defs.functions.get(func_name).ok_or_else(|| {
        format!("function '{}' not found", func_name)
    })?;
    if fdef.type_params.is_empty() {
        return Err(format!("'{}' is not a generic function", func_name));
    }
    let mut type_map: HashMap<String, String> = HashMap::new();
    for (arg, (_, ptype_str)) in call_args.iter().zip(fdef.params.iter()) {
        let arg_type = infer_type(arg, env, defs, &HashMap::new())?;
        let arg_str = type_to_string(&arg_type);
        let ptype = type_from_str(ptype_str, defs);
        match &ptype {
            Type::Var(tv) => {
                if let Some(existing) = type_map.get(tv) {
                    if existing != &arg_str && existing != "unknown" && arg_str != "unknown" {
                        return Err(format!(
                            "Type mismatch for type parameter '{}': inferred '{}' and '{}' (func={})",
                            tv, existing, arg_str, func_name
                        ));
                    }
                }
                type_map.insert(tv.clone(), arg_str);
            }
            _ => {
                collect_var_bindings(&arg_type, &ptype, &mut type_map, defs)?;
            }
        }
    }
    let mut result = Vec::new();
    'outer: for tp in &fdef.type_params {
        match type_map.get(tp) {
            Some(s) => result.push(s.clone()),
            None => {
                if let Some(exp) = expected {
                    let mut bindings: HashMap<String, String> = HashMap::new();
                    let ret_ty = type_from_str(&fdef.return_type.clone().unwrap_or_default(), defs);
                    if collect_var_bindings(exp, &ret_ty, &mut bindings, defs).is_ok() {
                        if let Some(s) = bindings.get(tp) {
                            result.push(s.clone());
                            continue;
                        }
                    }
                    if let Some(rt_str) = &fdef.return_type {
                        let base = rt_str.split('(').next().unwrap_or(rt_str);
                        if let Some(sd) = defs.structs.get(base) {
                            if !sd.type_params.is_empty() {
                                let exp_str = type_to_string(exp);
                                let exp_args = generic_args_of(&exp_str);
                                if exp_args.len() == sd.type_params.len() {
                                    for (i, stp) in sd.type_params.iter().enumerate() {
                                        if stp == tp {
                                            if let Some(earg) = exp_args.get(i) {
                                                result.push(earg.clone());
                                                continue 'outer;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                return Err(format!(
                    "Cannot infer type parameter '{}' from call arguments in '{}'",
                    tp, func_name
                ));
            }
        }
    }
    Ok(result)
}
fn collect_var_bindings(
    concrete: &Type,
    pattern: &Type,
    type_map: &mut HashMap<String, String>,
    defs: &Defs,
) -> Result<(), String> {
    if matches!(concrete, Type::Unknown) {
        return Ok(());
    }
    match (concrete, pattern) {
        (Type::Var(tv), _) => {
            let s = type_to_string(concrete);
            if s == tv.as_str() || s == "unknown" {
                return Ok(());
            }
            if let Some(existing) = type_map.get(tv) {
                if existing != &s && existing != "unknown" && s != "unknown" {
                    return Err(format!(
                        "Type mismatch for type parameter '{}': '{}' vs '{}'",
                        tv, existing, s
                    ));
                }
            }
            type_map.insert(tv.clone(), s);
            Ok(())
        }
        (_, Type::Var(tv)) => {
            let s = type_to_string(concrete);
            if s == tv.as_str() || s == "unknown" {
                return Ok(());
            }
            if let Some(existing) = type_map.get(tv) {
                if existing != &s && existing != "unknown" && s != "unknown" {
                    return Err(format!(
                        "Type mismatch for type parameter '{}': '{}' vs '{}'",
                        tv, existing, s
                    ));
                }
            }
            type_map.insert(tv.clone(), s);
            Ok(())
        }
        (Type::List(p_inner), Type::List(c_inner)) => {
            collect_var_bindings(p_inner, c_inner, type_map, defs)
        }
        (Type::Option(p_inner), Type::Option(c_inner)) => {
            collect_var_bindings(p_inner, c_inner, type_map, defs)
        }
        (Type::Array(p_inner), Type::Array(c_inner)) => {
            collect_var_bindings(p_inner, c_inner, type_map, defs)
        }
        (Type::Struct(p), Type::Struct(c)) | (Type::State(p), Type::State(c)) => {
            let concrete_args = generic_args_of(p);
            let pattern_args = generic_args_of(c);
            if concrete_args.len() != pattern_args.len() {
                return Ok(());
            }
            for (pat, con) in pattern_args.iter().zip(concrete_args.iter()) {
                match type_from_str(pat, defs) {
                    Type::Var(tv) => {
                        let s = type_to_string(&type_from_str(con, defs));
                        if let Some(existing) = type_map.get(&tv) {
                            if existing != &s && existing != "unknown" && s != "unknown" {
                                return Err(format!(
                                    "Type mismatch for type parameter '{}': '{}' vs '{}'",
                                    tv, existing, s
                                ));
                            }
                        }
                        type_map.insert(tv, s);
                    }
                    _ => {
                        collect_var_bindings(
                            &type_from_str(con, defs),
                            &type_from_str(pat, defs),
                            type_map,
                            defs,
                        )?;
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn check_generic_constraints(
    fdef: &FunctionDef,
    type_args: &[String],
    defs: &Defs,
) -> Result<(), String> {
    for (tv, iface) in &fdef.constraints {
        for (i, tp) in fdef.type_params.iter().enumerate() {
            if tp == tv && i < type_args.len() {
                let concrete_type = type_from_str(&type_args[i], defs);
                let ok = match &concrete_type {
                    Type::Struct(sname) => struct_satisfies_interface(defs, sname, iface),
                    Type::Interface(iname, _) => iname == iface,
                    Type::Unknown => true,
                    Type::Int | Type::Float | Type::Bool | Type::String => true,
                    _ => false,
                };
                if !ok {
                    return Err(format!(
                        "Type error: {} does not satisfy constraint '{}: {}'",
                        type_to_string(&concrete_type),
                        tv,
                        iface
                    ));
                }
            }
        }
    }
    Ok(())
}
fn collect_mono_from_expr(
    e: &Expr,
    env: &mut HashMap<String, Type>,
    defs: &Defs,
    mono_fdefs: &mut HashMap<MonoKey, FunctionDef>,
    call_updates: &mut HashMap<String, String>,
    worklist: &mut Vec<String>,
    expected: Option<&Type>,
) -> Result<(), String> {
    match e {
        Expr::Call { func, args } => {
            let base_name;
            let explicit_type_args: Option<Vec<String>>;
            if let Some((base, type_strs)) = parse_generic_call_name(func) {
                base_name = base.to_string();
                explicit_type_args = Some(type_strs.iter().map(|s| s.to_string()).collect());
            } else {
                base_name = func.clone();
                explicit_type_args = None;
            }
            let base_name = if defs.functions.contains_key(&base_name) {
                base_name
            } else {
                resolve_pkg_name(defs, &base_name).unwrap_or(base_name)
            };
            if let Some(fdef) = defs.functions.get(&base_name) {
                if !fdef.type_params.is_empty() {
                    let type_args: Vec<String> = if let Some(ref explicit) = explicit_type_args {
                        explicit.clone()
                    } else {
                        infer_generic_args(&base_name, args, env, defs, expected)?
                    };
                    check_generic_constraints(fdef, &type_args, defs)?;
                    let mangled = mangled_name(&base_name, &type_args);
                    let key = MonoKey {
                        function: intern(&base_name),
                        types: type_args.clone(),
                        mangled: mangled.clone(),
                    };
                    if !mono_fdefs.contains_key(&key) {
                        let type_param_strs: Vec<&str> = type_args.iter().map(|s| s.as_str()).collect();
                        let mono = monomorphize_function(fdef, &fdef.type_params, &type_param_strs);
                        mono_fdefs.insert(key, mono);
                        worklist.push(mangled.clone());
                    }
                    if func != &mangled {
                        call_updates.insert(func.clone(), mangled);
                    }
                }
            }
            for a in args {
                collect_mono_from_expr(a, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_mono_from_expr(left, env, defs, mono_fdefs, call_updates, worklist, None)?;
            collect_mono_from_expr(right, env, defs, mono_fdefs, call_updates, worklist, None)?;
        }
        Expr::UnOp { operand, .. } => {
            collect_mono_from_expr(operand, env, defs, mono_fdefs, call_updates, worklist, None)?;
        }
        Expr::MethodCall { object, args, .. } => {
            collect_mono_from_expr(object, env, defs, mono_fdefs, call_updates, worklist, None)?;
            for a in args {
                collect_mono_from_expr(a, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
        }
        Expr::FieldAccess { object, .. } => {
            collect_mono_from_expr(object, env, defs, mono_fdefs, call_updates, worklist, None)?;
        }
        Expr::Array(items) => {
            for it in items {
                collect_mono_from_expr(it, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
        }
        Expr::Range { start, end } => {
            collect_mono_from_expr(start, env, defs, mono_fdefs, call_updates, worklist, None)?;
            collect_mono_from_expr(end, env, defs, mono_fdefs, call_updates, worklist, None)?;
        }
        Expr::Index { target, index } => {
            collect_mono_from_expr(target, env, defs, mono_fdefs, call_updates, worklist, None)?;
            collect_mono_from_expr(index, env, defs, mono_fdefs, call_updates, worklist, None)?;
        }
        Expr::Slice { target, start, end } => {
            collect_mono_from_expr(target, env, defs, mono_fdefs, call_updates, worklist, None)?;
            if let Some(s) = start {
                collect_mono_from_expr(s, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
            if let Some(e) = end {
                collect_mono_from_expr(e, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
        }
        Expr::Await(inner) => {
            collect_mono_from_expr(inner, env, defs, mono_fdefs, call_updates, worklist, None)?;
        }
        _ => {}
    }
    Ok(())
}
fn collect_mono_from_stmts(
    stmts: &[Stmt],
    env: &mut HashMap<String, Type>,
    defs: &Defs,
    mono_fdefs: &mut HashMap<MonoKey, FunctionDef>,
    call_updates: &mut HashMap<String, String>,
    worklist: &mut Vec<String>,
) -> Result<(), String> {
    for s in stmts {
        match s {
            Stmt::Let { name, value, type_hint, .. } => {
                if let Ok(t) = infer_type(value, env, defs, &HashMap::new()) {
                    env.insert(name.clone(), t);
                }
                let expected = type_hint.as_ref().map(|h| type_from_str(h, defs));
                if let Some(exp) = &expected {
                    env.insert(name.clone(), exp.clone());
                }
                let exp_ref = expected.as_ref();
                collect_mono_from_expr(value, env, defs, mono_fdefs, call_updates, worklist, exp_ref)?;
            }
            Stmt::Return { explicit_type: _, value } => {
                if let Some(e) = value {
                    collect_mono_from_expr(e, env, defs, mono_fdefs, call_updates, worklist, None)?;
                }
            }
            Stmt::Expr(e) => {
                collect_mono_from_expr(e, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
            Stmt::Assign { name, value } => {
                if let Ok(t) = infer_type(value, env, defs, &HashMap::new()) {
                    env.insert(name.clone(), t);
                }
                collect_mono_from_expr(value, env, defs, mono_fdefs, call_updates, worklist, None)?;
            }
            Stmt::If { cond, then_branch, else_branch } => {
                collect_mono_from_expr(cond, env, defs, mono_fdefs, call_updates, worklist, None)?;
                let mut then_env = env.clone();
                collect_mono_from_stmts(then_branch, &mut then_env, defs, mono_fdefs, call_updates, worklist)?;
                if let Some(eb) = else_branch {
                    let mut else_env = env.clone();
                    collect_mono_from_stmts(eb, &mut else_env, defs, mono_fdefs, call_updates, worklist)?;
                }
            }
            Stmt::While { cond, body } => {
                collect_mono_from_expr(cond, env, defs, mono_fdefs, call_updates, worklist, None)?;
                collect_mono_from_stmts(body, env, defs, mono_fdefs, call_updates, worklist)?;
            }
            Stmt::For { var, iterable, body } => {
                if let Ok(it_ty) = infer_type(iterable, env, defs, &HashMap::new()) {
                    let elem = match &it_ty {
                        Type::List(e) => (**e).clone(),
                        _ => Type::Unknown,
                    };
                    env.insert(var.clone(), elem);
                }
                collect_mono_from_expr(iterable, env, defs, mono_fdefs, call_updates, worklist, None)?;
                collect_mono_from_stmts(body, env, defs, mono_fdefs, call_updates, worklist)?;
            }
            Stmt::Match { expr, arms } => {
                collect_mono_from_expr(expr, env, defs, mono_fdefs, call_updates, worklist, None)?;
                for (_, body) in arms {
                    let mut arm_env = env.clone();
                    collect_mono_from_stmts(body, &mut arm_env, defs, mono_fdefs, call_updates, worklist)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
fn update_call_in_expr(e: &mut Expr, call_updates: &HashMap<String, String>) {
    match e {
        Expr::Call { func, .. } => {
            if let Some(new_name) = call_updates.get(func.as_str()) {
                *func = new_name.clone();
            }
        }
        Expr::BinOp { left, right, .. } => {
            update_call_in_expr(left, call_updates);
            update_call_in_expr(right, call_updates);
        }
        Expr::UnOp { operand, .. } => {
            update_call_in_expr(operand, call_updates);
        }
        Expr::MethodCall { object, args, .. } => {
            update_call_in_expr(object, call_updates);
            for a in args {
                update_call_in_expr(a, call_updates);
            }
        }
        Expr::FieldAccess { object, .. } => {
            update_call_in_expr(object, call_updates);
        }
        Expr::Array(items) => {
            for it in items {
                update_call_in_expr(it, call_updates);
            }
        }
        Expr::Range { start, end } => {
            update_call_in_expr(start, call_updates);
            update_call_in_expr(end, call_updates);
        }
        Expr::Await(inner) => {
            update_call_in_expr(inner, call_updates);
        }
        _ => {}
    }
}
fn update_call_in_stmts(stmts: &mut [Stmt], call_updates: &HashMap<String, String>) {
    for s in stmts.iter_mut() {
        match s {
            Stmt::Let { value, .. } => update_call_in_expr(value, call_updates),
            Stmt::Return { explicit_type: _, value } => {
                if let Some(e) = value {
                    update_call_in_expr(e, call_updates);
                }
            }
            Stmt::Expr(e) => update_call_in_expr(e, call_updates),
            Stmt::Assign { value, .. } => update_call_in_expr(value, call_updates),
            Stmt::If { cond, then_branch, else_branch } => {
                update_call_in_expr(cond, call_updates);
                update_call_in_stmts(then_branch, call_updates);
                if let Some(eb) = else_branch {
                    update_call_in_stmts(eb, call_updates);
                }
            }
            Stmt::While { cond, body } => {
                update_call_in_expr(cond, call_updates);
                update_call_in_stmts(body, call_updates);
            }
            Stmt::For { iterable, body, .. } => {
                update_call_in_expr(iterable, call_updates);
                update_call_in_stmts(body, call_updates);
            }
            Stmt::Match { expr, arms } => {
                update_call_in_expr(expr, call_updates);
                for (_, body) in arms {
                    update_call_in_stmts(body, call_updates);
                }
            }
            Stmt::Fn { body, .. } => {
                update_call_in_stmts(body, call_updates);
            }
            Stmt::Struct { methods, .. } => {
                update_call_in_stmts(methods, call_updates);
            }
            _ => {}
        }
    }
}
fn monomorphize_all(defs: &mut Defs, stmts: &mut [Stmt]) -> Result<(), String> {
    let mut mono_fdefs: HashMap<MonoKey, FunctionDef> = HashMap::new();
    let mut call_updates: HashMap<String, String> = HashMap::new();
    let mut worklist: Vec<String> = defs.functions.keys().cloned().collect();
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut env: HashMap<String, Type> = HashMap::new();
    collect_mono_from_stmts(
        stmts,
        &mut env,
        defs,
        &mut mono_fdefs,
        &mut call_updates,
        &mut worklist,
    )?;
    while let Some(func_name) = worklist.pop() {
        if processed.contains(&func_name) {
            continue;
        }
        processed.insert(func_name.clone());
        let fdef = match defs.functions.get(&func_name) {
            Some(f) => f.clone(),
            None => continue,
        };
        let mut env: HashMap<String, Type> = HashMap::new();
        for (pname, ptype) in &fdef.params {
            env.insert(pname.clone(), type_from_str(ptype, defs));
        }
        collect_mono_from_stmts(
            &fdef.body,
            &mut env,
            defs,
            &mut mono_fdefs,
            &mut call_updates,
            &mut worklist,
        )?;
    }
    for (key, fdef) in &mono_fdefs {
        let mangled = &key.mangled;
        if !defs.functions.contains_key(mangled) {
            defs.functions.insert(mangled.clone(), fdef.clone());
        }
    }
    for (_name, fdef) in defs.functions.iter_mut() {
        update_call_in_stmts(&mut fdef.body, &call_updates);
    }
    update_call_in_stmts(stmts, &call_updates);
    Ok(())
}
fn memory_analyze(stmts: &[Stmt], defs: &Defs) -> Result<HashMap<String, MemoryPlace>, String> {
    let mut report: Vec<(String, MemoryPlace)> = Vec::new();
    analyze_block(stmts, false, defs, &mut report)?;
    let mut map: HashMap<String, MemoryPlace> = HashMap::new();
    for (name, place) in &report {
        map.insert(name.clone(), *place);
    }
    Ok(map)
}
fn report_memory(memory: &HashMap<String, MemoryPlace>) {
    eprintln!("=== Memory ===");
    let mut entries: Vec<(&String, &MemoryPlace)> = memory.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (name, place) in entries {
        let p = match place {
            MemoryPlace::Stack => "stack",
            MemoryPlace::Heap => "heap",
        };
        eprintln!("  {} -> {}", name, p);
    }
    eprintln!();
}
fn eval_string_method(s: &str, method: &str, args: &[Value]) -> Result<Value, String> {
    match method {
        "len" => Ok(Value::Int(s.chars().count() as i64)),
        "byte_len" => Ok(Value::Int(s.len() as i64)),
        "chars" => {
            let chars: Vec<Value> = s.chars().map(|c| Value::String(c.to_string())).collect();
            Ok(Value::Array(chars))
        }
        "bytes" => {
            let bytes: Vec<Value> = s.bytes().map(|b| Value::Int(b as i64)).collect();
            Ok(Value::Array(bytes))
        }
        "slice" => {
            if args.len() != 2 {
                return Err("slice() takes exactly 2 arguments (start, end)".to_string());
            }
            let start = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err("slice() start must be int".to_string()),
            };
            let end = match &args[1] {
                Value::Int(n) => *n,
                _ => return Err("slice() end must be int".to_string()),
            };
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let s_idx = if start < 0 { len + start } else { start }.max(0) as usize;
            let e_idx = if end < 0 { len + end } else { end }.max(0) as usize;
            let s_idx = s_idx.min(chars.len());
            let e_idx = e_idx.min(chars.len());
            if s_idx > e_idx {
                return Ok(Value::String(String::new()));
            }
            Ok(Value::String(chars[s_idx..e_idx].iter().collect()))
        }
        "to_upper" => Ok(Value::String(s.to_uppercase())),
        "to_lower" => Ok(Value::String(s.to_lowercase())),
        "repeat" => {
            if args.len() != 1 {
                return Err("repeat() takes exactly 1 argument".to_string());
            }
            let n = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err("repeat() expects an int".to_string()),
            };
            let n = if n < 0 { 0 } else { n as usize };
            Ok(Value::String(s.repeat(n)))
        }
        "length" => Ok(Value::Int(s.chars().count() as i64)),
        "write" => {
            if args.len() != 1 {
                return Err("write() takes exactly 1 argument (content)".to_string());
            }
            match &args[0] {
                Value::String(content) => fs::write(s, content)
                    .map(|_| Value::Bool(true))
                    .map_err(|e| format!("write('{}') failed: {}", s, e)),
                _ => Err("write() expects a string content".to_string()),
            }
        }
        "exists" => {
            if !args.is_empty() {
                return Err("exists() takes no arguments".to_string());
            }
            Ok(Value::Bool(std::path::Path::new(s).exists()))
        }
        "read" => {
            if !args.is_empty() {
                return Err("read() takes no arguments".to_string());
            }
            fs::read_to_string(s).map(Value::String).map_err(|e| format!("read('{}') failed: {}", s, e))
        }
        "append" => {
            if args.len() != 1 {
                return Err("append() takes exactly 1 argument (content)".to_string());
            }
            match &args[0] {
                Value::String(content) => {
                    use std::io::Write as _;
                    let mut f = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(s)
                        .map_err(|e| format!("append('{}') failed: {}", s, e))?;
                    f.write_all(content.as_bytes())
                        .map(|_| Value::Bool(true))
                        .map_err(|e| format!("append('{}') failed: {}", s, e))
                }
                _ => Err("append() expects a string content".to_string()),
            }
        }
        "remove" => {
            if !args.is_empty() {
                return Err("remove() takes no arguments".to_string());
            }
            fs::remove_file(s)
                .map(|_| Value::Bool(true))
                .map_err(|e| format!("remove('{}') failed: {}", s, e))
        }
        "metadata" => {
            if !args.is_empty() {
                return Err("metadata() takes no arguments".to_string());
            }
            let meta = std::fs::metadata(s)
                .map_err(|e| format!("metadata('{}') failed: {}", s, e))?;
            let size = meta.len() as i64;
            let is_dir = meta.is_dir();
            let is_file = meta.is_file();
            Ok(Value::Struct {
                name: "fs.FileMetadata".to_string(),
                fields: vec![
                    ("size".to_string(), Value::Int(size)),
                    ("is_dir".to_string(), Value::Bool(is_dir)),
                    ("is_file".to_string(), Value::Bool(is_file)),
                ],
            })
        }
        other => Err(format!("Unknown String method: {}", other)),
    }
}
fn eval_float_method(v: f64, method: &str, args: &[Value]) -> Result<Value, String> {
    match method {
        "abs" => {
            if !args.is_empty() {
                return Err("abs() takes no arguments".to_string());
            }
            Ok(Value::Float(v.abs()))
        }
        "sqrt" => {
            if !args.is_empty() {
                return Err("sqrt() takes no arguments".to_string());
            }
            Ok(Value::Float(v.sqrt()))
        }
        other => Err(format!("Unknown Float method: {}", other)),
    }
}
fn eval_list_method(arr: &[Value], method: &str, args: &[Value]) -> Result<Value, String> {
    match method {
        "add" => {
            if args.len() != 1 {
                return Err("add() takes exactly 1 argument".to_string());
            }
            let mut new_arr = arr.to_vec();
            new_arr.push(args[0].clone());
            Ok(Value::Array(new_arr))
        }
        "len" => Ok(Value::Int(arr.len() as i64)),
        "get" => {
            if args.len() != 1 {
                return Err("get() takes exactly 1 argument".to_string());
            }
            let idx = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err("get() index must be int".to_string()),
            };
            if idx < 0 || idx as usize >= arr.len() {
                return Err(format!("List index out of bounds: {}", idx));
            }
            Ok(arr[idx as usize].clone())
        }
        "set" => {
            if args.len() != 2 {
                return Err("set() takes exactly 2 arguments".to_string());
            }
            let idx = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err("set() index must be int".to_string()),
            };
            if idx < 0 || idx as usize >= arr.len() {
                return Err(format!("List index out of bounds: {}", idx));
            }
            let mut new_arr = arr.to_vec();
            new_arr[idx as usize] = args[1].clone();
            Ok(Value::Array(new_arr))
        }
        other => Err(format!("Unknown List method: {}", other)),
    }
}
fn eval_list_method_ext(arr: &[Value], method: &str, args: &[Value]) -> Result<Value, String> {
    match method {
        "push" => {
            if args.len() != 1 {
                return Err("push() takes exactly 1 argument".to_string());
            }
            let mut new_arr = arr.to_vec();
            new_arr.push(args[0].clone());
            Ok(Value::Array(new_arr))
        }
        "pop" => {
            if !args.is_empty() {
                return Err("pop() takes no arguments".to_string());
            }
            if arr.is_empty() {
                return Err("pop() on empty list".to_string());
            }
            Ok(arr[arr.len() - 1].clone())
        }
        "first" => {
            if !args.is_empty() {
                return Err("first() takes no arguments".to_string());
            }
            if arr.is_empty() {
                return Err("first() on empty list".to_string());
            }
            Ok(arr[0].clone())
        }
        "last" => {
            if !args.is_empty() {
                return Err("last() takes no arguments".to_string());
            }
            if arr.is_empty() {
                return Err("last() on empty list".to_string());
            }
            Ok(arr[arr.len() - 1].clone())
        }
        "length" | "size" => {
            if !args.is_empty() {
                return Err(format!("{}(takes no arguments", method));
            }
            Ok(Value::Int(arr.len() as i64))
        }
        "index_of" => {
            if args.len() != 1 {
                return Err("index_of() takes exactly 1 argument".to_string());
            }
            let target = &args[0];
            for (i, v) in arr.iter().enumerate() {
                if v == target {
                    return Ok(Value::Int(i as i64));
                }
            }
            Ok(Value::Int(-1))
        }
        "contains" => {
            if args.len() != 1 {
                return Err("contains() takes exactly 1 argument".to_string());
            }
            Ok(Value::Bool(arr.contains(&args[0])))
        }
        "reverse" => {
            if !args.is_empty() {
                return Err("reverse() takes no arguments".to_string());
            }
            let mut new_arr = arr.to_vec();
            new_arr.reverse();
            Ok(Value::Array(new_arr))
        }
        _ => eval_list_method(arr, method, args),
    }
}
fn resolve_pkg_name(defs: &Defs, name: &str) -> Option<String> {
    defs.resolve_function(name)
}
fn is_runtime_builtin(name: &str) -> bool {
    matches!(
        name,
        "and" | "or" | "not" | "print" | "println" | "len" | "StringBuilder" | "int" | "float"
            | "str" | "Some" | "None" | "push" | "pop" | "first" | "last" | "index_of"
            | "contains_item" | "contains" | "starts_with" | "ends_with" | "trim" | "replace"
            | "split" | "slice" | "byte_len" | "reverse" | "remove_at" | "to_upper" | "to_lower"
            | "repeat" | "time_now" | "time_sleep" | "read_file" | "write_file" | "append_file"
            | "remove_file" | "file_exists" | "fs_exists" | "fs_size" | "fs_metadata"
            | "fs_list_dir" | "fs_create_dir" | "input"
            | "abs" | "sqrt" | "min" | "max" | "clamp" | "pow"
    )
}
fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>, defs: &Defs) -> Result<Value, String> {
    match expr {
        Expr::IntLit(n) => Ok(Value::Int(*n)),
        Expr::LongLit(n) => Ok(Value::Int(*n)),
        Expr::FloatLit(f) => Ok(Value::Float(*f)),
        Expr::StringLit(s) => Ok(Value::String(s.clone())),
        Expr::BoolLit(b) => Ok(Value::Bool(*b)),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Undefined variable: {}", name)),
        Expr::BinOp { left, op, right, resolved_operator } => {
            let l = eval_expr(left, env, defs)?;
            let r = eval_expr(right, env, defs)?;
            match resolved_operator {
                Some(ResolvedOperator::MethodCall { method, op: mop }) => {
                    let (sname, fields) = match &l {
                        Value::Struct { name, fields } => (name.clone(), fields.clone()),
                        _ => return Err("Operator interface target is not a struct".to_string()),
                    };
                    let res = call_method(&sname, &fields, &method, vec![r], defs)?;
                    match mop.as_str() {
                        "+" => Ok(res),
                        "==" => Ok(res),
                        "!=" => match res {
                            Value::Bool(b) => Ok(Value::Bool(!b)),
                            _ => Err("equal() must return bool".to_string()),
                        },
                        "<" | ">" | "<=" | ">=" => {
                            let i = match res {
                                Value::Int(n) => n,
                                _ => return Err("compare() must return int".to_string()),
                            };
                            let b = match mop.as_str() {
                                "<" => i < 0,
                                ">" => i > 0,
                                "<=" => i <= 0,
                                ">=" => i >= 0,
                                _ => unreachable!(),
                            };
                            Ok(Value::Bool(b))
                        }
                        _ => Err(format!("Unknown operator interface op: {}", mop)),
                    }
                }
                Some(ResolvedOperator::Builtin) | None => {
                    match (&l, &r) {
                        (Value::Int(a), Value::Int(b)) => {
                            let result = match op.as_str() {
                                "+" => Value::Int(a + b),
                                "-" => Value::Int(a - b),
                                "*" => Value::Int(a * b),
                                "/" => {
                                    if *b == 0 {
                                        return Err("Division by zero".to_string());
                                    }
                                    Value::Int(a / b)
                                }
                                "%" => Value::Int(a % b),
                                "==" => Value::Bool(a == b),
                                "!=" => Value::Bool(a != b),
                                "<" => Value::Bool(a < b),
                                ">" => Value::Bool(a > b),
                                "<=" => Value::Bool(a <= b),
                                ">=" => Value::Bool(a >= b),
                                _ => return Err(format!("Unknown operator: {}", op)),
                            };
                            Ok(result)
                        }
                        (Value::Float(a), Value::Float(b)) => {
                            let result = match op.as_str() {
                                "+" => Value::Float(a + b),
                                "-" => Value::Float(a - b),
                                "*" => Value::Float(a * b),
                                "/" => {
                                    if *b == 0.0 {
                                        return Err("Division by zero".to_string());
                                    }
                                    Value::Float(a / b)
                                }
                                "%" => Value::Float(a % b),
                                "==" => Value::Bool(a == b),
                                "!=" => Value::Bool(a != b),
                                "<" => Value::Bool(a < b),
                                ">" => Value::Bool(a > b),
                                "<=" => Value::Bool(a <= b),
                                ">=" => Value::Bool(a >= b),
                                _ => return Err(format!("Unknown operator: {}", op)),
                            };
                            Ok(result)
                        }
                        (Value::String(a), Value::String(b)) => {
                            match op.as_str() {
                                "+" => Ok(Value::String(format!("{}{}", a, b))),
                                "==" => Ok(Value::Bool(a == b)),
                                "!=" => Ok(Value::Bool(a != b)),
                                _ => Err(format!("Invalid operation on strings: {}", op)),
                            }
                        }
                        (Value::Bool(a), Value::Bool(b)) => {
                            match op.as_str() {
                                "and" => Ok(Value::Bool(*a && *b)),
                                "or" => Ok(Value::Bool(*a || *b)),
                                "==" => Ok(Value::Bool(a == b)),
                                "!=" => Ok(Value::Bool(a != b)),
                                _ => Err(format!("Invalid operation on bools: {}", op)),
                            }
                        }
                        _ => Err("Type mismatch in binary operation".to_string()),
                    }
                }
            }
        }
        Expr::UnOp { op, operand } => {
            let val = eval_expr(operand, env, defs)?;
            match op.as_str() {
                "-" => match val {
                    Value::Int(n) => Ok(Value::Int(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    _ => Err("Cannot negate non-numeric value".to_string()),
                },
                "not" => match val {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    _ => Err("Cannot negate non-boolean value".to_string()),
                },
                _ => Err(format!("Unknown unary operator: {}", op)),
            }
        }
        Expr::Tuple(elems) => {
            let mut vals = Vec::new();
            for e in elems {
                vals.push(eval_expr(e, env, defs)?);
            }
            Ok(Value::Tuple(vals))
        }
        Expr::TupleAccess { tuple, index } => {
            let val = eval_expr(tuple, env, defs)?;
            match val {
                Value::Tuple(elems) => {
                    if *index < elems.len() {
                        Ok(elems[*index].clone())
                    } else {
                        Err(format!(
                            "Runtime error: tuple index {} out of bounds (len {})",
                            index,
                            elems.len()
                        ))
                    }
                }
                _ => Err(format!("Runtime error: cannot index non-tuple value")),
            }
        }
        Expr::Call { func, args } => {
            match func.as_str() {
                "print" | "println" => {
                    for arg in args {
                        let val = eval_expr(arg, env, defs)?;
                        println!("{}", val.to_string());
                    }
                    Ok(Value::Int(0))
                }
                "len" => {
                    if args.len() != 1 {
                        return Err("len() takes exactly 1 argument".to_string());
                    }
                    let val = eval_expr(&args[0], env, defs)?;
                    match val {
                        Value::String(s) => Ok(Value::Int(s.len() as i64)),
                        Value::Array(arr) => Ok(Value::Int(arr.len() as i64)),
                        _ => Err("len() only works on strings and arrays".to_string()),
                    }
                }
                "StringBuilder" => {
                    if !args.is_empty() {
                        return Err("StringBuilder() takes no arguments".to_string());
                    }
                    Ok(Value::StringBuilder(String::new()))
                }
                "int" => {
                    if args.len() != 1 {
                        return Err("int() takes exactly 1 argument".to_string());
                    }
                    let val = eval_expr(&args[0], env, defs)?;
                    match val {
                        Value::Int(n) => Ok(Value::Int(n)),
                        Value::Float(f) => Ok(Value::Int(f as i64)),
                        Value::String(s) => s
                            .trim()
                            .parse::<i64>()
                            .map(Value::Int)
                            .map_err(|_| format!("Cannot convert '{}' to int", s)),
                        _ => Err("int() cannot convert this value".to_string()),
                    }
                }
                "float" => {
                    if args.len() != 1 {
                        return Err("float() takes exactly 1 argument".to_string());
                    }
                    let val = eval_expr(&args[0], env, defs)?;
                    match val {
                        Value::Int(n) => Ok(Value::Float(n as f64)),
                        Value::Float(f) => Ok(Value::Float(f)),
                        Value::String(s) => s
                            .trim()
                            .parse::<f64>()
                            .map(Value::Float)
                            .map_err(|_| format!("Cannot convert '{}' to float", s)),
                        _ => Err("float() cannot convert this value".to_string()),
                    }
                }
        "str" => {
            if args.len() != 1 {
                return Err("str() takes exactly 1 argument".to_string());
            }
            let val = eval_expr(&args[0], env, defs)?;
            Ok(Value::String(val.to_string()))
        }
        "Some" => {
            if args.len() != 1 {
                return Err("Some() takes exactly 1 argument".to_string());
            }
            let val = eval_expr(&args[0], env, defs)?;
            Ok(Value::Option(Some(Box::new(val))))
        }
        "None" => {
            if args.len() != 0 {
                return Err("None() takes no arguments".to_string());
            }
            Ok(Value::Option(None))
        }
        "push" => {
            let list = eval_expr(&args[0], env, defs)?;
            let item = eval_expr(&args[1], env, defs)?;
            if let Value::Array(mut arr) = list {
                arr.push(item);
                Ok(Value::Array(arr))
            } else {
                Err("push() expects a list as first argument".to_string())
            }
        }
        "pop" => {
            let list = eval_expr(&args[0], env, defs)?;
            if let Value::Array(mut arr) = list {
                if let Some(v) = arr.pop() {
                    Ok(v)
                } else {
                    Err("pop() on empty list".to_string())
                }
            } else {
                Err("pop() expects a list".to_string())
            }
        }
        "first" => {
            let list = eval_expr(&args[0], env, defs)?;
            if let Value::Array(arr) = list {
                arr.first().cloned().ok_or_else(|| "first() on empty list".to_string())
            } else {
                Err("first() expects a list".to_string())
            }
        }
        "last" => {
            let list = eval_expr(&args[0], env, defs)?;
            if let Value::Array(arr) = list {
                arr.last().cloned().ok_or_else(|| "last() on empty list".to_string())
            } else {
                Err("last() expects a list".to_string())
            }
        }
        "index_of" => {
            let list = eval_expr(&args[0], env, defs)?;
            let item = eval_expr(&args[1], env, defs)?;
            if let Value::Array(arr) = list {
                let idx = arr.iter().position(|v| v == &item);
                Ok(Value::Int(idx.map(|i| i as i64).unwrap_or(-1)))
            } else {
                Err("index_of() expects a list".to_string())
            }
        }
        "contains_item" => {
            let list = eval_expr(&args[0], env, defs)?;
            let item = eval_expr(&args[1], env, defs)?;
            if let Value::Array(arr) = list {
                Ok(Value::Bool(arr.iter().any(|v| v == &item)))
            } else {
                Err("contains_item() expects a list".to_string())
            }
        }
        "contains" => {
            let s = eval_expr(&args[0], env, defs)?;
            let sub = eval_expr(&args[1], env, defs)?;
            if let (Value::String(a), Value::String(b)) = (s, sub) {
                Ok(Value::Bool(a.contains(&b)))
            } else {
                Err("contains() expects two strings".to_string())
            }
        }
        "starts_with" => {
            let s = eval_expr(&args[0], env, defs)?;
            let p = eval_expr(&args[1], env, defs)?;
            if let (Value::String(a), Value::String(b)) = (s, p) {
                Ok(Value::Bool(a.starts_with(&b)))
            } else {
                Err("starts_with() expects two strings".to_string())
            }
        }
        "ends_with" => {
            let s = eval_expr(&args[0], env, defs)?;
            let p = eval_expr(&args[1], env, defs)?;
            if let (Value::String(a), Value::String(b)) = (s, p) {
                Ok(Value::Bool(a.ends_with(&b)))
            } else {
                Err("ends_with() expects two strings".to_string())
            }
        }
        "trim" => {
            let s = eval_expr(&args[0], env, defs)?;
            if let Value::String(a) = s {
                Ok(Value::String(a.trim().to_string()))
            } else {
                Err("trim() expects a string".to_string())
            }
        }
        "replace" => {
            let s = eval_expr(&args[0], env, defs)?;
            let from = eval_expr(&args[1], env, defs)?;
            let to = eval_expr(&args[2], env, defs)?;
            if let (Value::String(a), Value::String(b), Value::String(c)) = (s, from, to) {
                Ok(Value::String(a.replace(&b, &c)))
            } else {
                Err("replace() expects three strings".to_string())
            }
        }
        "split" => {
            let s = eval_expr(&args[0], env, defs)?;
            let sep = eval_expr(&args[1], env, defs)?;
            if let (Value::String(a), Value::String(b)) = (s, sep) {
                let parts: Vec<Value> = a
                    .split(&b)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Value::Array(parts))
            } else {
                Err("split() expects two strings".to_string())
            }
        }
        "slice" => {
            let s = eval_expr(&args[0], env, defs)?;
            let start = eval_expr(&args[1], env, defs)?;
            let end = eval_expr(&args[2], env, defs)?;
            if let (Value::String(a), Value::Int(s0), Value::Int(e0)) = (s, start, end) {
                let chars: Vec<char> = a.chars().collect();
                let s0 = s0.clamp(0, chars.len() as i64) as usize;
                let e0 = e0.clamp(0, chars.len() as i64) as usize;
                let e0 = e0.max(s0);
                Ok(Value::String(chars[s0..e0].iter().collect()))
            } else {
                Err("slice() expects (string, int, int)".to_string())
            }
        }
        "byte_len" => {
            let s = eval_expr(&args[0], env, defs)?;
            if let Value::String(a) = s {
                Ok(Value::Int(a.len() as i64))
            } else {
                Err("byte_len() expects a string".to_string())
            }
        }
        "reverse" => {
            let list = eval_expr(&args[0], env, defs)?;
            if let Value::Array(mut arr) = list {
                arr.reverse();
                Ok(Value::Array(arr))
            } else {
                Err("reverse() expects a list".to_string())
            }
        }
        "remove_at" => {
            let list = eval_expr(&args[0], env, defs)?;
            let idx = eval_expr(&args[1], env, defs)?;
            if let (Value::Array(mut arr), Value::Int(i)) = (list, idx) {
                if i < 0 || (i as usize) >= arr.len() {
                    return Err("remove_at() index out of bounds".to_string());
                }
                arr.remove(i as usize);
                Ok(Value::Array(arr))
            } else {
                Err("remove_at() expects (list, int)".to_string())
            }
        }
        "to_upper" => {
            let s = eval_expr(&args[0], env, defs)?;
            if let Value::String(a) = s {
                Ok(Value::String(a.to_uppercase()))
            } else {
                Err("to_upper() expects a string".to_string())
            }
        }
        "to_lower" => {
            let s = eval_expr(&args[0], env, defs)?;
            if let Value::String(a) = s {
                Ok(Value::String(a.to_lowercase()))
            } else {
                Err("to_lower() expects a string".to_string())
            }
        }
        "repeat" => {
            let s = eval_expr(&args[0], env, defs)?;
            let n = eval_expr(&args[1], env, defs)?;
            if let (Value::String(a), Value::Int(times)) = (s, n) {
                let times = if times < 0 { 0 } else { times as usize };
                Ok(Value::String(a.repeat(times)))
            } else {
                Err("repeat() expects (string, int)".to_string())
            }
        }
        "time_now" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            Ok(Value::Float(secs))
        }
        "time_sleep" => {
            let s = eval_expr(&args[0], env, defs)?;
            if let Value::Float(secs) = s {
                if secs > 0.0 {
                    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
                }
                Ok(Value::Bool(true))
            } else {
                Err("time_sleep() expects a float (seconds)".to_string())
            }
        }
        "fs_exists" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                Ok(Value::Bool(std::path::Path::new(&path).exists()))
            } else {
                Err("fs_exists() expects a string path".to_string())
            }
        }
        "fs_size" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                match std::fs::metadata(&path) {
                    Ok(m) => Ok(Value::Int(m.len() as i64)),
                    Err(e) => Err(format!("fs_size('{}') failed: {}", path, e)),
                }
            } else {
                Err("fs_size() expects a string path".to_string())
            }
        }
        "fs_metadata" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                match std::fs::metadata(&path) {
                    Ok(m) => Ok(Value::Struct {
                        name: "FileMetadata".to_string(),
                        fields: vec![
                            ("size".to_string(), Value::Int(m.len() as i64)),
                            ("is_dir".to_string(), Value::Bool(m.is_dir())),
                            ("is_file".to_string(), Value::Bool(m.is_file())),
                        ],
                    }),
                    Err(e) => Err(format!("fs_metadata('{}') failed: {}", path, e)),
                }
            } else {
                Err("fs_metadata() expects a string path".to_string())
            }
        }
        "fs_list_dir" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                match std::fs::read_dir(&path) {
                    Ok(rd) => {
                        let mut out: Vec<Value> = Vec::new();
                        for e in rd.flatten() {
                            out.push(Value::String(
                                e.path().to_string_lossy().to_string(),
                            ));
                        }
                        Ok(Value::Array(out))
                    }
                    Err(e) => Err(format!("fs_list_dir('{}') failed: {}", path, e)),
                }
            } else {
                Err("fs_list_dir() expects a string path".to_string())
            }
        }
        "fs_create_dir" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                std::fs::create_dir_all(&path)
                    .map(|_| Value::Bool(true))
                    .map_err(|e| format!("fs_create_dir('{}') failed: {}", path, e))
            } else {
                Err("fs_create_dir() expects a string path".to_string())
            }
        }
        "abs" => {
            let x = eval_expr(&args[0], env, defs)?;
            if let Value::Float(f) = x {
                Ok(Value::Float(f.abs()))
            } else {
                Err("abs() expects a float".to_string())
            }
        }
        "min" => {
            let a = eval_expr(&args[0], env, defs)?;
            let b = eval_expr(&args[1], env, defs)?;
            if let (Value::Float(x), Value::Float(y)) = (a, b) {
                Ok(Value::Float(x.min(y)))
            } else {
                Err("min() expects two floats".to_string())
            }
        }
        "max" => {
            let a = eval_expr(&args[0], env, defs)?;
            let b = eval_expr(&args[1], env, defs)?;
            if let (Value::Float(x), Value::Float(y)) = (a, b) {
                Ok(Value::Float(x.max(y)))
            } else {
                Err("max() expects two floats".to_string())
            }
        }
        "clamp" => {
            let x = eval_expr(&args[0], env, defs)?;
            let lo = eval_expr(&args[1], env, defs)?;
            let hi = eval_expr(&args[2], env, defs)?;
            if let (Value::Float(a), Value::Float(b), Value::Float(c)) = (x, lo, hi) {
                Ok(Value::Float(a.max(b).min(c)))
            } else {
                Err("clamp() expects (float, float, float)".to_string())
            }
        }
        "sqrt" => {
            let x = eval_expr(&args[0], env, defs)?;
            if let Value::Float(f) = x {
                Ok(Value::Float(f.sqrt()))
            } else {
                Err("sqrt() expects a float".to_string())
            }
        }
        "pow" => {
            let a = eval_expr(&args[0], env, defs)?;
            let b = eval_expr(&args[1], env, defs)?;
            if let (Value::Float(x), Value::Float(y)) = (a, b) {
                Ok(Value::Float(x.powf(y)))
            } else {
                Err("pow() expects two floats".to_string())
            }
        }
        "input" => {
            use std::io::Write as _;
            if let Some(p) = args.first() {
                let prompt = eval_expr(p, env, defs)?;
                print!("{}", prompt.to_string());
                let _ = std::io::stdout().flush();
            }
            let mut line = String::new();
            std::io::stdin()
                .read_line(&mut line)
                .map_err(|e| format!("input() failed: {}", e))?;
            Ok(Value::String(line.trim_end_matches(['\n', '\r']).to_string()))
        }
        "read_file" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                fs::read_to_string(&path)
                    .map(Value::String)
                    .map_err(|e| format!("read_file('{}') failed: {}", path, e))
            } else {
                Err("read_file() expects a string path".to_string())
            }
        }
        "write_file" => {
            let p = eval_expr(&args[0], env, defs)?;
            let c = eval_expr(&args[1], env, defs)?;
            if let (Value::String(path), Value::String(content)) = (p, c) {
                fs::write(&path, content)
                    .map(|_| Value::Bool(true))
                    .map_err(|e| format!("write_file('{}') failed: {}", path, e))
            } else {
                Err("write_file() expects (string path, string content)".to_string())
            }
        }
        "append_file" => {
            use std::io::Write as _;
            let p = eval_expr(&args[0], env, defs)?;
            let c = eval_expr(&args[1], env, defs)?;
            if let (Value::String(path), Value::String(content)) = (p, c) {
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| format!("append_file('{}') failed: {}", path, e))?;
                f.write_all(content.as_bytes())
                    .map(|_| Value::Bool(true))
                    .map_err(|e| format!("append_file('{}') failed: {}", path, e))
            } else {
                Err("append_file() expects (string path, string content)".to_string())
            }
        }
        "file_exists" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                Ok(Value::Bool(std::path::Path::new(&path).exists()))
            } else {
                Err("file_exists() expects a string path".to_string())
            }
        }
        "remove_file" => {
            let p = eval_expr(&args[0], env, defs)?;
            if let Value::String(path) = p {
                fs::remove_file(&path)
                    .map(|_| Value::Bool(true))
                    .map_err(|e| format!("remove_file('{}') failed: {}", path, e))
            } else {
                Err("remove_file() expects a string path".to_string())
            }
        }
        other => {
                    if other.contains('.') && !defs.functions.contains_key(other) {
                        if let Some(dot) = other.rfind('.') {
                            let (obj_path, method) = other.split_at(dot);
                            let method = &method[1..];
                            let obj_expr = expr_from_path(obj_path);
                            let obj_val = eval_expr(&obj_expr, env, defs)?;
                            let mut arg_vals = Vec::new();
                            for a in args {
                                arg_vals.push(eval_expr(a, env, defs)?);
                            }
                            return dispatch_method(obj_val, method, arg_vals, env, defs);
                        }
                    }
                    let base = match other.find('(') {
                        Some(i) => &other[..i],
                        None => other,
                    };
                    let base = resolve_pkg_name(defs, base).unwrap_or_else(|| base.to_string());
                    if args.is_empty() {
                        if let Some(fdef) = defs.functions.get(&base) {
                            if fdef.params.is_empty() {
                                let mut arg_vals = Vec::new();
                                return call_function(&base, arg_vals, defs);
                            }
                        }
                    }
                    let struct_key = defs
                        .structs
                        .get(&base)
                        .map(|_| base.to_string())
                        .or_else(|| {
                            let suffix = format!(".{}", base);
                            let mut found: Option<String> = None;
                            for k in defs.structs.keys() {
                                if k.ends_with(&suffix) {
                                    if found.is_some() {
                                        return None;
                                    }
                                    found = Some(k.clone());
                                }
                            }
                            found
                        });
                    if let Some(skey) = struct_key {
                        let struct_def = &defs.structs[&skey];
                        let field_defs = &struct_def.fields;
                        let mut values = Vec::new();
                        for a in args {
                            values.push(eval_expr(a, env, defs)?);
                        }
                        if values.len() != field_defs.len() {
                            return Err(format!(
                                "{} expects {} field(s), got {}",
                                base,
                                field_defs.len(),
                                values.len()
                            ));
                        }
                        let fields = field_defs
                            .iter()
                            .map(|(fname, _)| fname.clone())
                            .zip(values.into_iter())
                            .collect();
                        Ok(Value::Struct {
                            name: skey.clone(),
                            fields,
                        })
                    } else {
                        let state_key = defs
                            .state_variants
                            .get(&base)
                            .map(|_| base.to_string())
                            .or_else(|| {
                                let suffix = format!(".{}", base);
                                let mut found: Option<String> = None;
                                for k in defs.state_variants.keys() {
                                    if k.ends_with(&suffix) {
                                        if found.is_some() {
                                            return None;
                                        }
                                        found = Some(k.clone());
                                    }
                                }
                                found
                            });
                        if let Some(stkey) = state_key {
                            let mut values = Vec::new();
                            for a in args {
                                values.push(eval_expr(a, env, defs)?);
                            }
                            Ok(Value::State {
                                name: stkey.clone(),
                                values,
                            })
                        } else if defs.functions.contains_key(&base) {
                            let mut arg_vals = Vec::new();
                            for a in args {
                                arg_vals.push(eval_expr(a, env, defs)?);
                            }
                            call_function(&base, arg_vals, defs)
                        } else {
                            Err(format!("Unknown function: {}", func))
                        }
                    }
                }
            }
        }
        Expr::MethodCall { object, method, args } => {
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(eval_expr(a, env, defs)?);
            }
            if let Expr::Ident(var) = object.as_ref() {
                let current = env
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", var))?;
                match current {
                    Value::StringBuilder(mut s) => match method.as_str() {
                        "add" => {
                            if arg_vals.len() != 1 {
                                return Err("add() takes exactly 1 argument".to_string());
                            }
                            s.push_str(&arg_vals[0].to_string());
                            env.insert(var.clone(), Value::StringBuilder(s));
                            Ok(Value::Int(0))
                        }
                        "build" => Ok(Value::String(s)),
                        other => Err(format!("Unknown StringBuilder method: {}", other)),
                    },
                    Value::Array(arr) => {
                        let result = eval_list_method_ext(&arr, method, &arg_vals)?;
                        if method == "add" || method == "set" {
                            env.insert(var.clone(), result.clone());
                        }
                        Ok(result)
                    }
                    Value::Slice(arr) => {
                        let result = eval_list_method_ext(&arr, method, &arg_vals)?;
                        Ok(result)
                    }
                    Value::String(s) => eval_string_method(&s, method, &arg_vals),
                    Value::Float(f) => eval_float_method(f, method, &arg_vals),
                    Value::Struct { name, fields } => {
                        eval_struct_method_or_call(&name, &fields, method, arg_vals, defs)
                    }
                    other => Err(format!(
                        "Type {:?} has no method '{}'",
                        other, method
                    )),
                }
            } else {
                let obj = eval_expr(object, env, defs)?;
                match obj {
                    Value::StringBuilder(s) => match method.as_str() {
                        "build" => Ok(Value::String(s)),
                        other => Err(format!("Unknown StringBuilder method: {}", other)),
                    },
                    Value::Array(arr) => eval_list_method_ext(&arr, method, &arg_vals),
                    Value::String(s) => eval_string_method(&s, method, &arg_vals),
                    Value::Float(f) => eval_float_method(f, method, &arg_vals),
                    Value::Struct { name, fields } => {
                        eval_struct_method_or_call(&name, &fields, method, arg_vals, defs)
                    }
                    other => Err(format!(
                        "Type {:?} has no method '{}'",
                        other, method
                    )),
                }
            }
        }
        Expr::FieldAccess { object, field } => {
            let obj = eval_expr(object, env, defs)?;
            match obj {
                Value::Struct { name, fields } => {
                    for (fname, fval) in &fields {
                        if fname == field {
                            return Ok(fval.clone());
                        }
                    }
                    Err(format!("Unknown field: {} on struct {}", field, name))
                }
                other => Err(format!(
                    "Field access on non-struct value: {:?}",
                    other
                )),
            }
        }
        Expr::Array(elements) => {
            let mut values = Vec::new();
            for e in elements {
                values.push(eval_expr(e, env, defs)?);
            }
            Ok(Value::Array(values))
        }
        Expr::Range { start, end } => {
            let s = eval_expr(start, env, defs)?;
            let e = eval_expr(end, env, defs)?;
            match (&s, &e) {
                (Value::Int(a), Value::Int(b)) => {
                    let mut values = Vec::new();
                    let mut i = *a;
                    while i < *b {
                        values.push(Value::Int(i));
                        i += 1;
                    }
                    Ok(Value::Array(values))
                }
                _ => Err("Range requires integer bounds".to_string()),
            }
        }
        Expr::Index { target, index } => {
            let target_val = eval_expr(target, env, defs)?;
            let index_val = eval_expr(index, env, defs)?;
            let idx = match index_val {
                Value::Int(i) => i as usize,
                _ => return Err("Runtime error: list index must be int".to_string()),
            };
            match target_val {
                Value::Array(values) | Value::Slice(values) => {
                    if idx >= values.len() {
                        return Err(format!(
                            "Runtime error: index out of bounds\nindex:\n{}\nlength:\n{}",
                            idx,
                            values.len()
                        ));
                    }
                    Ok(values[idx].clone())
                }
                _ => Err("Runtime error: cannot index non-list type".to_string()),
            }
        }
        Expr::Slice { target, start, end } => {
            let target_val = eval_expr(target, env, defs)?;
            let values = match target_val {
                Value::Array(values) | Value::Slice(values) => values,
                _ => return Err("Runtime error: cannot slice non-list type".to_string()),
            };
            let len = values.len() as i64;
            let start_idx = match start {
                Some(s) => {
                    let sv = eval_expr(s, env, defs)?;
                    match sv {
                        Value::Int(i) => i,
                        _ => return Err("Runtime error: slice start must be int".to_string()),
                    }
                }
                None => 0,
            };
            let end_idx = match end {
                Some(e) => {
                    let ev = eval_expr(e, env, defs)?;
                    match ev {
                        Value::Int(i) => i,
                        _ => return Err("Runtime error: slice end must be int".to_string()),
                    }
                }
                None => len,
            };
            let start_idx = if start_idx < 0 { 0 } else { start_idx as usize };
            let end_idx = if end_idx > len { len } else { end_idx };
            let end_idx = if end_idx < start_idx as i64 { start_idx as i64 } else { end_idx };
            Ok(Value::Slice(values[start_idx..end_idx as usize].to_vec()))
        }
        Expr::Await(inner) => {
            let fut = eval_expr(inner, env, defs)?;
            match fut {
                Value::Future { func, args } => {
                    call_function_impl(&func, args, defs, true)
                }
                other => Err(format!(
                    "await can only be applied to a lime (async) function call, got {:?}",
                    other
                )),
            }
        }
    }
}
fn call_function(
    name: &str,
    args: Vec<Value>,
    defs: &Defs,
) -> Result<Value, String> {
    call_function_impl(name, args, defs, false)
}
fn call_function_impl(
    name: &str,
    args: Vec<Value>,
    defs: &Defs,
    force_run: bool,
) -> Result<Value, String> {
    let func = defs
        .functions
        .get(name)
        .cloned()
        .ok_or_else(|| format!("Unknown function: {}", name))?;
    if args.len() != func.params.len() {
        return Err(format!(
            "Function {} expects {} argument(s), got {}",
            name,
            func.params.len(),
            args.len()
        ));
    }
    let mut local: HashMap<String, Value> = HashMap::new();
    for ((param_name, _param_type), val) in func.params.iter().zip(args.iter().cloned()) {
        local.insert(param_name.clone(), val);
    }
    if func.is_async && !force_run {
        return Ok(Value::Future {
            func: name.to_string(),
            args,
        });
    }
    Ok(exec_value(execute_stmts(&func.body, &mut local, defs)?))
}
fn call_method(
    struct_name: &str,
    fields: &Vec<(String, Value)>,
    method: &str,
    args: Vec<Value>,
    defs: &Defs,
) -> Result<Value, String> {
    let struct_def = defs
        .structs
        .get(struct_name)
        .ok_or_else(|| format!("Unknown struct: {}", struct_name))?;
    let func = struct_def
        .methods
        .get(method)
        .cloned()
        .ok_or_else(|| format!("Unknown method: {} on {}", method, struct_name))?;
    if args.len() != func.params.len() {
        return Err(format!(
            "{}.{} expects {} argument(s), got {}",
            struct_name,
            method,
            func.params.len(),
            args.len()
        ));
    }
    let mut local: HashMap<String, Value> = HashMap::new();
    for (fname, fval) in fields {
        local.insert(fname.clone(), fval.clone());
    }
    for ((param_name, _param_type), val) in func.params.iter().zip(args.into_iter()) {
        local.insert(param_name.clone(), val);
    }
    Ok(exec_value(execute_stmts(&func.body, &mut local, defs)?))
}
fn eval_struct_method(
    struct_name: &str,
    fields: &Vec<(String, Value)>,
    method: &str,
    args: Vec<Value>,
    _defs: &Defs,
) -> Result<Value, String> {
    let field = |name: &str| -> Result<Value, String> {
        fields
            .iter()
            .find(|(f, _)| f == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| format!("struct {} missing field '{}'", struct_name, name))
    };
    match (struct_name, method) {
        ("time.Instant", "sleep") => {
            if args.len() != 1 {
                return Err("Instant.sleep() takes exactly 1 argument".to_string());
            }
            match &args[0] {
                Value::Float(secs) => {
                    if *secs > 0.0 {
                        std::thread::sleep(std::time::Duration::from_secs_f64(*secs));
                    }
                    Ok(Value::Bool(true))
                }
                _ => Err("Instant.sleep() expects a Float number of seconds".to_string()),
            }
        }
        ("time.Instant", "elapsed") => {
            if !args.is_empty() {
                return Err("Instant.elapsed() takes no arguments".to_string());
            }
            let start = match field("secs")? {
                Value::Float(s) => s,
                _ => return Err("Instant.secs is not a Float".to_string()),
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            Ok(Value::Struct {
                name: "time.Duration".to_string(),
                fields: vec![("secs".to_string(), Value::Float(now - start))],
            })
        }
        ("time.Duration", "secs") => field("secs"),
        ("fs.FileMetadata", "size") => field("size"),
        ("fs.FileMetadata", "is_dir") => field("is_dir"),
        ("fs.FileMetadata", "is_file") => field("is_file"),
        ("collections.HashMap", "insert") => {
            if args.len() != 2 {
                return Err("HashMap.insert() takes exactly 2 arguments".to_string());
            }
            let entries_val = field("entries")?;
            let mut entries = match entries_val {
                Value::Array(a) => a,
                _ => return Err("HashMap.entries is not a list".to_string()),
            };
            let entry = Value::Struct {
                name: "collections.Entry".to_string(),
                fields: vec![
                    ("key".to_string(), args[0].clone()),
                    ("value".to_string(), args[1].clone()),
                ],
            };
            entries.push(entry);
            Ok(Value::Struct {
                name: "collections.HashMap".to_string(),
                fields: vec![("entries".to_string(), Value::Array(entries))],
            })
        }
        ("collections.HashMap", "get") => {
            if args.len() != 1 {
                return Err("HashMap.get() takes exactly 1 argument".to_string());
            }
            let entries = match field("entries")? {
                Value::Array(a) => a,
                _ => return Err("HashMap.entries is not a list".to_string()),
            };
            for e in entries {
                if let Value::Struct { name, fields } = e {
                    if name == "collections.Entry" {
                        let ek = fields.iter().find(|(f, _)| f == "key").map(|(_, v)| v.clone());
                        let ev = fields.iter().find(|(f, _)| f == "value").map(|(_, v)| v.clone());
                        if let (Some(k), Some(v)) = (ek, ev) {
                            if k == args[0] {
                                return Ok(Value::Struct {
                                    name: "option.Some".to_string(),
                                    fields: vec![("0".to_string(), v)],
                                });
                            }
                        }
                    }
                }
            }
            Ok(Value::Struct {
                name: "option.None".to_string(),
                fields: vec![],
            })
        }
        ("collections.HashMap", "contains") => {
            if args.len() != 1 {
                return Err("HashMap.contains() takes exactly 1 argument".to_string());
            }
            let entries = match field("entries")? {
                Value::Array(a) => a,
                _ => return Err("HashMap.entries is not a list".to_string()),
            };
            for e in entries {
                if let Value::Struct { name, fields } = e {
                    if name == "collections.Entry" {
                        if let Some((_, k)) = fields.iter().find(|(f, _)| f == "key") {
                            if k == &args[0] {
                                return Ok(Value::Bool(true));
                            }
                        }
                    }
                }
            }
            Ok(Value::Bool(false))
        }
        ("collections.HashMap", "length") | ("collections.HashMap", "size") => {
            if !args.is_empty() {
                return Err(format!("HashMap.{}() takes no arguments", method));
            }
            match field("entries")? {
                Value::Array(a) => Ok(Value::Int(a.len() as i64)),
                _ => Err("HashMap.entries is not a list".to_string()),
            }
        }
        ("collections.HashSet", "add") => {
            if args.len() != 1 {
                return Err("HashSet.add() takes exactly 1 argument".to_string());
            }
            let items_val = field("items")?;
            let mut items = match items_val {
                Value::Array(a) => a,
                _ => return Err("HashSet.items is not a list".to_string()),
            };
            if !items.contains(&args[0]) {
                items.push(args[0].clone());
            }
            Ok(Value::Struct {
                name: "collections.HashSet".to_string(),
                fields: vec![("items".to_string(), Value::Array(items))],
            })
        }
        ("collections.HashSet", "contains") => {
            if args.len() != 1 {
                return Err("HashSet.contains() takes exactly 1 argument".to_string());
            }
            match field("items")? {
                Value::Array(a) => Ok(Value::Bool(a.contains(&args[0]))),
                _ => Err("HashSet.items is not a list".to_string()),
            }
        }
        ("collections.HashSet", "length") | ("collections.HashSet", "size") => {
            if !args.is_empty() {
                return Err(format!("HashSet.{}() takes no arguments", method));
            }
            match field("items")? {
                Value::Array(a) => Ok(Value::Int(a.len() as i64)),
                _ => Err("HashSet.items is not a list".to_string()),
            }
        }
        ("string.String", "to_upper") => match field("0")? {
            Value::String(s) => Ok(Value::String(s.to_uppercase())),
            _ => Err("string.String value is not a String".to_string()),
        },
        ("string.String", "to_lower") => match field("0")? {
            Value::String(s) => Ok(Value::String(s.to_lowercase())),
            _ => Err("string.String value is not a String".to_string()),
        },
        ("string.String", "repeat") => {
            if args.len() != 1 {
                return Err("string.String.repeat() takes exactly 1 argument".to_string());
            }
            let s = match field("0")? {
                Value::String(s) => s,
                _ => return Err("string.String value is not a String".to_string()),
            };
            let n = match &args[0] {
                Value::Int(n) => *n,
                _ => return Err("string.String.repeat() expects an Int".to_string()),
            };
            let mut out = String::new();
            for _ in 0..n.max(0) {
                out.push_str(&s);
            }
            Ok(Value::String(out))
        }
        ("string.String", "length") => match field("0")? {
            Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
            _ => Err("string.String value is not a String".to_string()),
        },
        _ => Err(format!(
            "Unknown method '{}' on struct {}",
            method, struct_name
        )),
    }
}
#[derive(Debug)]
enum ExecResult {
    Continue,
    Value(Value),
    Return(Value),
}
fn exec_value(r: ExecResult) -> Value {
    match r {
        ExecResult::Return(v) | ExecResult::Value(v) => v,
        ExecResult::Continue => Value::Int(0),
    }
}
fn execute_stmts(
    stmts: &[Stmt],
    env: &mut HashMap<String, Value>,
    defs: &Defs,
) -> Result<ExecResult, String> {
    let mut last: ExecResult = ExecResult::Continue;
    let len = stmts.len();
    let mut defers: Vec<&[Stmt]> = Vec::new();
    for (idx, stmt) in stmts.iter().enumerate() {
        let r = match stmt {
            Stmt::Defer { body } => {
                defers.push(body);
                ExecResult::Continue
            }
            _ => execute_stmt(stmt, env, defs)?,
        };
        match r {
            ExecResult::Return(v) => {
                for d in defers.iter().rev() {
                    execute_stmts(d, env, defs)?;
                }
                return Ok(ExecResult::Return(v));
            }
            other => {
                if idx == len - 1 {
                    last = other;
                }
            }
        }
    }
    for d in defers.iter().rev() {
        execute_stmts(d, env, defs)?;
    }
    Ok(last)
}
fn bind_tuple_value(
    pat_elems: &[Pattern],
    vals: &[Value],
    out: &mut Vec<(String, Value)>,
) -> Result<(), String> {
    if pat_elems.len() != vals.len() {
        return Err(format!(
            "tuple pattern size mismatch (expected {}, received {})",
            pat_elems.len(),
            vals.len()
        ));
    }
    for (pat, v) in pat_elems.iter().zip(vals.iter()) {
        match pat {
            Pattern::Catch => {}
            Pattern::Variant { name, bindings } if bindings.is_empty() => {
                out.push((name.clone(), v.clone()));
            }
            Pattern::Tuple(inner) => match v {
                Value::Tuple(inner_vals) => bind_tuple_value(inner, inner_vals, out)?,
                other => {
                    return Err(format!(
                        "nested tuple pattern does not match value {:?}",
                        other
                    ))
                }
            },
            other => {
                return Err(format!(
                    "unsupported pattern {:?} in tuple match",
                    other
                ))
            }
        }
    }
    Ok(())
}
fn execute_stmt(
    stmt: &Stmt,
    env: &mut HashMap<String, Value>,
    defs: &Defs,
) -> Result<ExecResult, String> {
    match stmt {
        Stmt::Let { name, value, .. } => {
            let v = eval_expr(value, env, defs)?;
            env.insert(name.clone(), v);
            Ok(ExecResult::Continue)
        }
        Stmt::Destructure { vars, value } => {
            let v = eval_expr(value, env, defs)?;
            match v {
                Value::Tuple(elems) => {
                    if vars.len() != elems.len() {
                        return Err(format!(
                            "Runtime error: tuple pattern size mismatch (expected {}, received {})",
                            vars.len(),
                            elems.len()
                        ));
                    }
                    for (name, item) in vars.iter().zip(elems.into_iter()) {
                        env.insert(name.clone(), item);
                    }
                    Ok(ExecResult::Continue)
                }
                other => Err(format!(
                    "Runtime error: destructure expects a tuple value, got {:?}",
                    other
                )),
            }
        }
        Stmt::Expr(e) => {
            let v = eval_expr(e, env, defs)?;
            Ok(ExecResult::Value(v))
        }
        Stmt::Assign { name, value } => {
            let v = eval_expr(value, env, defs)?;
            env.insert(name.clone(), v);
            Ok(ExecResult::Continue)
        }
        Stmt::Return { explicit_type: _, value } => {
            let v = match value {
                Some(e) => eval_expr(e, env, defs)?,
                None => Value::Int(0),
            };
            Ok(ExecResult::Return(v))
        }
        Stmt::If { cond, then_branch, else_branch } => {
            let cond_val = eval_expr(cond, env, defs)?;
            let is_true = match cond_val {
                Value::Bool(b) => b,
                other => {
                    return Err(format!(
                        "Condition must be bool (implicit int->bool conversion is forbidden), got {:?}",
                        other
                    ))
                }
            };
            if is_true {
                execute_stmts(then_branch, env, defs)
            } else if let Some(els) = else_branch {
                execute_stmts(els, env, defs)
            } else {
                Ok(ExecResult::Continue)
            }
        }
        Stmt::For { var, iterable, body } => {
            let val = eval_expr(iterable, env, defs)?;
            let items: Vec<Value> = match val {
                Value::Array(arr) | Value::Slice(arr) => arr,
                other => return Err(format!("Cannot iterate over {:?}", other)),
            };
            for item in items {
                env.insert(var.clone(), item);
                match execute_stmts(body, env, defs)? {
                    ExecResult::Return(v) => return Ok(ExecResult::Return(v)),
                    ExecResult::Continue | ExecResult::Value(_) => {}
                }
            }
            Ok(ExecResult::Continue)
        }
        Stmt::While { cond, body } => {
            loop {
                let cond_val = eval_expr(cond, env, defs)?;
                let is_true = match cond_val {
                    Value::Bool(b) => b,
                    other => {
                        return Err(format!(
                            "while condition must be bool (implicit int->bool conversion is forbidden), got {:?}",
                            other
                        ))
                    }
                };
                if !is_true {
                    break;
                }
                match execute_stmts(body, env, defs)? {
                    ExecResult::Return(v) => return Ok(ExecResult::Return(v)),
                    ExecResult::Continue | ExecResult::Value(_) => {}
                }
            }
            Ok(ExecResult::Continue)
        }
        Stmt::Fn { .. } => Ok(ExecResult::Continue),
        Stmt::Match { expr, arms } => {
            let val = eval_expr(expr, env, defs)?;
            match val {
                Value::State { name, values } => {
                    for (pattern, body) in arms {
                        match pattern {
                            Pattern::Catch => return execute_stmts(body, env, defs),
                            Pattern::Try { elems } => {
                                if name == "Success" {
                                    let mut binds: Vec<(String, Value)> = Vec::new();
                                    bind_tuple_value(elems, &values, &mut binds)?;
                                    for (k, v) in binds {
                                        env.insert(k, v);
                                    }
                                    return execute_stmts(body, env, defs);
                                }
                            }
                            Pattern::Error => {
                                if name == "Error" {
                                    if let Some(v) = values.first() {
                                        env.insert("error".to_string(), v.clone());
                                    }
                                    return execute_stmts(body, env, defs);
                                }
                            }
                            Pattern::Variant { name: pname, bindings } => {
                                if pname == &name {
                                    for (idx, binding) in bindings.iter().enumerate() {
                                        if binding != "Ignore" {
                                            let v = values.get(idx).cloned().unwrap_or(Value::Int(0));
                                            env.insert(binding.clone(), v);
                                        }
                                    }
                                    return execute_stmts(body, env, defs);
                                }
                            }
                            Pattern::Tuple(_) => {}
                        }
                    }
                    Err(format!("Unhandled state: {}", name))
                }
                Value::Option(opt) => {
                    match opt {
                        Some(inner) => {
                            for (pattern, body) in arms {
                                match pattern {
                                    Pattern::Variant { name: pname, bindings } => {
                                        if pname == "Some" {
                                            for (idx, binding) in bindings.iter().enumerate() {
                                                if binding != "Ignore" {
                                                    let v = if idx == 0 {
                                                        (*inner).clone()
                                                    } else {
                                                        Value::Int(0)
                                                    };
                                                    env.insert(binding.clone(), v);
                                                }
                                            }
                                            return execute_stmts(body, env, defs);
                                        }
                                    }
                                    Pattern::Catch => return execute_stmts(body, env, defs),
                                    Pattern::Try { .. } | Pattern::Error | Pattern::Tuple(_) => {}
                                }
                            }
                            Err("Unhandled Option: Some".to_string())
                        }
                        None => {
                            for (pattern, body) in arms {
                                match pattern {
                                    Pattern::Variant { name: pname, bindings } => {
                                        if pname == "None" {
                                            let _ = bindings;
                                            return execute_stmts(body, env, defs);
                                        }
                                    }
                                    Pattern::Catch => return execute_stmts(body, env, defs),
                                    Pattern::Try { .. } | Pattern::Error | Pattern::Tuple(_) => {}
                                }
                            }
                            Err("Unhandled Option: None".to_string())
                        }
                    }
                }
                Value::Tuple(elems) => {
                    for (pattern, body) in arms {
                        match pattern {
                            Pattern::Catch => {
                                return execute_stmts(body, env, defs);
                            }
                            Pattern::Tuple(pat_elems) | Pattern::Try { elems: pat_elems } => {
                                let mut binds: Vec<(String, Value)> = Vec::new();
                                if bind_tuple_value(pat_elems, &elems, &mut binds).is_ok() {
                                    for (k, v) in binds {
                                        env.insert(k, v);
                                    }
                                    return execute_stmts(body, env, defs);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err("Unhandled tuple value".to_string())
                }
                other => Err(format!(
                    "match expects a state, option, or tuple value, got {:?}",
                    other
                )),
            }
        }
        Stmt::State { .. } => Ok(ExecResult::Continue),
        _ => Ok(ExecResult::Continue),
    }
}
#[cfg(test)]
mod phase10_tests {
    use super::*;
    #[test]
    fn fmt_normalizes_and_is_idempotent() {
        let src = "fn main():\n\t\tprintln(1)   \n\n\n    println(2)\n";
        let once = format_lime_source(src);
        assert!(!once.contains('\t'), "tabs must be normalized: {:?}", once);
        assert!(!once.contains("   \n"), "trailing ws must be trimmed");
        assert!(!once.contains("\n\n\n"), "blank runs must collapse");
        let twice = format_lime_source(&once);
        assert_eq!(once, twice, "formatter must be idempotent");
    }
    #[test]
    fn dce_removes_unused_functions() {
        let src = "fn used():\n    return 1\n\nfn unused():\n    return 2\n\nfn main():\n    println(used())\n";
        let (tokens, locs) = tokenize(src).expect("tokenize");
        let stmts = parse(tokens, locs).expect("parse");
        let mut defs = Defs::new();
        collect_defs(&stmts, &mut defs);
        assert!(defs.functions.contains_key("unused"));
        let removed = eliminate_dead_functions(&mut defs, &stmts);
        assert_eq!(removed, 1, "exactly one unused function removed");
        assert!(!defs.functions.contains_key("unused"));
        assert!(defs.functions.contains_key("used"));
        assert!(defs.functions.contains_key("main"));
    }
    #[test]
    fn type_shorthands_resolve() {
        let d = Defs::new();
        assert_eq!(type_from_str("i", &d), Type::Int);
        assert_eq!(type_from_str("f", &d), Type::Float);
        assert_eq!(type_from_str("b", &d), Type::Bool);
        assert_eq!(type_from_str("s", &d), Type::String);
    }
    #[test]
    fn type_from_str_void_maps_to_unit() {
        let d = Defs::new();
        assert_eq!(type_from_str("void", &d), Type::Unit);
        assert_eq!(type_from_str("unit", &d), Type::Unit);
        assert_eq!(type_to_string(&Type::Unit), "void");
    }
    fn codegen_from_source(src: &str) -> (String, Vec<String>) {
        let (tokens, locs) = tokenize(src).expect("tokenize");
        let stmts = parse(tokens, locs).expect("parse");
        let mut defs = Defs::new();
        collect_defs(&stmts, &mut defs);
        let _ = infer_function_return_types(&mut defs);
        let memory = memory_analyze(&stmts, &defs).expect("memory analyze");
        codegen::emit_llvm(&stmts, &defs, &memory)
    }
    #[test]
    fn codegen_reports_warning_for_unsupported_body() {
        let src = "fn main():\n    let x = missing_fn()\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            !warnings.is_empty(),
            "expected a codegen warning for the unlowerable body\n--- ir ---\n{}",
            ir
        );
        assert!(
            warnings.iter().any(|w| w.contains("main")),
            "warning should name the offending function 'main': {:?}",
            warnings
        );
        assert!(ir.contains("define"), "expected a function definition in IR");
    }
    #[test]
    fn codegen_no_warning_for_supported_body() {
        let src = "fn main():\n    let x = 1 + 2\n    println(x)\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "supported body must not produce warnings: {:?}",
            warnings
        );
        assert!(ir.contains("define"), "expected a function definition in IR");
    }
    #[test]
    fn codegen_list_literal_uses_runtime_alloc() {
        let src = "fn main():\n    let nums = [1, 2, 3]\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "no warnings expected: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
        assert!(
            ir.contains("call i8* @runtime_alloc"),
            "list literal must allocate via runtime_alloc\n--- ir ---\n{}",
            ir
        );
        assert!(
            ir.contains("insertvalue %LimeList"),
            "list literal must build a %LimeList\n--- ir ---\n{}",
            ir
        );
    }
    #[test]
    fn codegen_list_add_set_store_back() {
        let src = "fn main():\n    let nums = [1, 2, 3]\n    nums.add(4)\n    nums.set(0, 9)\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "no warnings expected: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
        assert!(
            ir.contains("call void @runtime_list_add(ptr sret(%LimeList)"),
            "add must use the sret ABI\n--- ir ---\n{}",
            ir
        );
        assert!(
            ir.contains("call void @runtime_list_set(ptr sret(%LimeList)"),
            "set must use the sret ABI\n--- ir ---\n{}",
            ir
        );
        let stores = ir.matches("store %LimeList").count();
        assert!(
            stores >= 3,
            "expected initial store + 2 store-backs, got {} store(s)\n--- ir ---\n{}",
            stores, ir
        );
    }
    #[test]
    fn codegen_get_on_non_int_list_warns() {
        let src = "fn main():\n    let strs = [\"a\", \"b\"]\n    println(strs.get(0))\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.iter().any(|w| w.contains("only supported for lists of Int")),
            "expected a get() support warning, got: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
    }
    #[test]
    fn codegen_string_runtime_methods() {
        let src = "fn main():\n    let s = \"hello\"\n    println(s.length())\n    println(s.slice(1, 3))\n    let t = s + \"!\"\n    println(t)\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "no warnings expected: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
        assert!(
            ir.contains("call i64 @strlen"),
            "length must lower to strlen\n--- ir ---\n{}",
            ir
        );
        assert!(
            ir.contains("call i8* @runtime_str_slice"),
            "slice must lower to runtime_str_slice\n--- ir ---\n{}",
            ir
        );
        assert!(
            ir.contains("call i8* @runtime_str_concat"),
            "concat must lower to runtime_str_concat\n--- ir ---\n{}",
            ir
        );
    }
    #[test]
    fn codegen_chars_bytes_sret_abi() {
        let src = "fn main():\n    let cs = \"abc\".chars()\n    let bs = \"abc\".bytes()\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "no warnings expected: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
        assert!(
            ir.contains("call void @runtime_str_chars(ptr sret(%LimeList)"),
            "chars must use the sret ABI\n--- ir ---\n{}",
            ir
        );
        assert!(
            ir.contains("call void @runtime_str_bytes(ptr sret(%LimeList)"),
            "bytes must use the sret ABI\n--- ir ---\n{}",
            ir
        );
    }
    #[test]
    fn codegen_bare_return_matches_inferred_type() {
        let src = "fn main():\n    println(\"hi\")\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "no warnings expected: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
        assert!(
            ir.contains("define void @main_lime"),
            "a function ending in a bare return must stay void\n--- ir ---\n{}",
            ir
        );
        assert!(
            ir.contains("  ret void\n"),
            "bare return must emit ret void in a void function\n--- ir ---\n{}",
            ir
        );
    }
    #[test]
    fn codegen_bare_return_void_stays_void() {
        let src = "fn main():\n    let x = 1\n    return\n";
        let (ir, warnings) = codegen_from_source(src);
        assert!(
            warnings.is_empty(),
            "no warnings expected: {:?}\n--- ir ---\n{}",
            warnings, ir
        );
        assert!(
            ir.contains("define void @main_lime"),
            "a void body must stay void\n--- ir ---\n{}",
            ir
        );
    }
    fn type_check_source_as(name: &str, src: &str) -> String {
        let (tokens, locs) = tokenize(src).expect("tokenize");
        let (stmts, stmt_locs) = parse_with_locs(tokens, locs, &std::collections::HashSet::new()).expect("parse");
        let mut defs = Defs::new();
        collect_defs(&stmts, &mut defs);
        let loc = make_loc_map(&stmts, &stmt_locs, name);
        match type_check_located(&stmts, &mut defs, &loc) {
            Ok(()) => String::new(),
            Err(e) => e,
        }
    }
    fn warnings_from_source(src: &str) -> Vec<String> {
        let (tokens, locs) = tokenize(src).expect("tokenize");
        let (stmts, stmt_locs) = parse_with_locs(tokens, locs, &std::collections::HashSet::new()).expect("parse");
        let mut defs = Defs::new();
        collect_defs(&stmts, &mut defs);
        let loc = make_loc_map(&stmts, &stmt_locs, "main.lime");
        let mut diags: Vec<Diagnostic> = Vec::new();
        collect_warnings(&stmts, &None, &loc, &mut diags);
        diags.iter().map(render_diagnostic).collect()
    }
    #[test]
    fn type_error_carries_position() {
        for (label, src) in [
            (
                "undef",
                "fn main():\n    println(missing)\n    return\n",
            ),
            (
                "mismatch",
                "fn main():\n    let x = 1\n    let y = x + \"s\"\n    println(y)\n    return\n",
            ),
            (
                "field",
                "fn main():\n    let x = 1\n    let y = x.foo\n    return\n",
            ),
        ] {
            let err = type_check_source_as("main.lime", src);
            assert!(
                err.contains("error[type] main.lime:"),
                "[{}] expected error[type] with file:line:col, got:\n{}",
                label,
                err
            );
            let after = err.split("error[type] main.lime:").nth(1).unwrap_or("");
            assert!(
                after.chars().next().map_or(false, |c| c.is_ascii_digit()),
                "[{}] expected line number after 'main.lime:', got:\n{}",
                label,
                err
            );
        }
    }
    #[test]
    fn valid_program_has_no_type_error() {
        let src = "fn main():\n    let x = 1 + 2\n    println(x)\n    return\n";
        let err = type_check_source_as("main.lime", src);
        assert!(err.is_empty(), "valid program should not error, got:\n{}", err);
    }
    #[test]
    fn multiple_type_errors_collected() {
        let src = "fn main():\n    let x = 1\n    let y = x + \"s\"\n    let z = missing_var\n    let w = 2\n    let q = w.foo\n    println(q)\n";
        let err = type_check_source_as("main.lime", src);
        assert!(
            err.contains("binary '+' type mismatch"),
            "expected type-mismatch error, got:\n{}",
            err
        );
        assert!(
            err.contains("undefined variable 'missing_var'"),
            "expected undefined-variable error, got:\n{}",
            err
        );
        assert!(
            err.contains("field access on non-struct"),
            "expected field-access error, got:\n{}",
            err
        );
        let annotated = err.matches("error[type] main.lime:").count();
        assert!(
            annotated >= 3,
            "expected >=3 annotated errors (one per failing statement), got {}:\n{}",
            annotated,
            err
        );
    }
    #[test]
    fn did_you_mean_suggests_similar_name() {
        let src = r#"
struct Message:
    s: message
fn print(s: s):
    return
fn main():
    let counter = 1
    let n = countre
    prnt("hi")
    let m = Message("x")
    let _ = m.messgae
    return
"#;
        let err = type_check_source_as("main.lime", src);
        assert!(
            err.contains("did you mean 'counter'?"),
            "expected hint for undefined variable 'countre', got:\n{}",
            err
        );
        assert!(
            err.contains("did you mean 'print'?"),
            "expected hint for undefined function 'prnit', got:\n{}",
            err
        );
        assert!(
            err.contains("did you mean 'message'?"),
            "expected hint for unknown field 'messgae', got:\n{}",
            err
        );
    }
    #[test]
    fn no_hint_when_no_close_candidate() {
        let src = "fn main():\n    let q = completely_unrelated_name_xyz\n    return\n";
        let err = type_check_source_as("main.lime", src);
        assert!(
            err.contains("undefined variable 'completely_unrelated_name_xyz'"),
            "expected undefined-variable error, got:\n{}",
            err
        );
        assert!(
            !err.contains("did you mean"),
            "no hint should appear when there is no close candidate, got:\n{}",
            err
        );
    }
    #[test]
    fn warns_on_unused_local() {
        let src = "fn main():\n    let x = 1\n    let y = 2\n    println(y)\n    return\n";
        let warns = warnings_from_source(src);
        assert!(
            warns.iter().any(|w| w.contains("unused variable 'x'")),
            "expected unused-variable warning for 'x', got:\n{:?}",
            warns
        );
        assert!(
            !warns.iter().any(|w| w.contains("unused variable 'y'")),
            "'y' is used, should not be flagged:\n{:?}",
            warns
        );
        assert!(
            warns.iter().any(|w| w.starts_with("warning[type] main.lime:")),
            "warnings must render via render_diagnostic, got:\n{:?}",
            warns
        );
    }
    #[test]
    fn warns_on_unreachable_code() {
        let src = "fn main():\n    let x = 1\n    return x\n    let y = 2\n    return y\n";
        let warns = warnings_from_source(src);
        assert!(
            warns.iter().any(|w| w.contains("unreachable code")),
            "expected unreachable-code warning, got:\n{:?}",
            warns
        );
    }
    #[test]
    fn no_warnings_for_clean_program() {
        let src = "fn main():\n    let x = 1\n    println(x)\n    return\n";
        let warns = warnings_from_source(src);
        assert!(warns.is_empty(), "clean program should warn nothing, got:\n{:?}", warns);
    }
}
