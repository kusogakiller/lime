use std::env;
use std::fs;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: lime <file.lime>");
        return;
    }

    let source = match fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };

    // Lexer
    let tokens = match tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            return;
        }
    };

    println!("=== Tokens ===");
    for token in &tokens {
        println!("{:?}", token);
    }
    println!();

    // Parser
    match parse(tokens) {
        Ok(stmts) => {
            println!("=== AST ===");
            for stmt in &stmts {
                println!("{:#?}", stmt);
            }
            println!();

            // Interpreter
            println!("=== Execution ===");
            let mut defs = Defs::new();
            collect_defs(&stmts, &mut defs);

            // Type Checker（実行前に型的に正しいか検査）
            if let Err(e) = type_check(&stmts, &defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            if defs.functions.contains_key("main") {
                if let Err(e) = call_function("main", Vec::new(), &defs) {
                    eprintln!("Runtime error: {}", e);
                }
            } else {
                // main が無い場合はトップレベル文を実行
                let mut env = HashMap::new();
                if let Err(e) = execute_stmts(&stmts, &mut env, &defs) {
                    eprintln!("Runtime error: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Parser error: {}", e);
        }
    }
}

// ===== Lexer =====
#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Keywords
    Fn, Struct, Interface, State, Let, Mut, If, Else, Match, Return,
    Async, Await, Unsafe, True, False, Where, For, While,
    Int, Float, Str, Bool, Option,

    // Operators
    Plus, Minus, Star, Slash, Percent,
    Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign,
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
    And, Or, Not,
    Dot, DoubleDot,

    // Delimiters
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Colon, DoubleColon, Semicolon, Comma, Arrow, FatArrow, Question,

    // Indents
    Indent, Dedent, Newline,

    // Literals
    IntLit(i64), FloatLit(f64), StringLit(String), Ident(String),

    Eof,
}

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();

    let mut tokens: Vec<Token> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];

    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while i < n {
        let ch = chars[i];

        // 改行
        if ch == '\n' {
            tokens.push(Token::Newline);
            line += 1;
            col = 1;
            i += 1;
            // 空行・コメント行をスキップし、次の実コード行のインデントで調整する
            let mut indent = 0usize;
            loop {
                // 行頭の空白を消費してインデントを計測
                indent = 0;
                while i < n && (chars[i] == ' ' || chars[i] == '\t') {
                    indent += 1;
                    i += 1;
                }
                if i < n && chars[i] == '#' {
                    // コメント行をスキップ
                    while i < n && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                if i < n && chars[i] == '\n' {
                    // 空行: 改行を記録して次の行へ
                    tokens.push(Token::Newline);
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
                    tokens.push(Token::Indent);
                } else if indent < current {
                    while indent_stack.len() > 1 && *indent_stack.last().unwrap() > indent {
                        indent_stack.pop();
                        tokens.push(Token::Dedent);
                    }
                }
            }
            continue;
        }

        // コメント
        if ch == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // 空白
        if ch == ' ' || ch == '\t' {
            i += 1;
            col += 1;
            continue;
        }

        // 文字列リテラル
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
            i += 1; // closing quote
            tokens.push(Token::StringLit(s));
            col += 1;
            continue;
        }

        // 数字
        if ch.is_ascii_digit() {
            let start = i;
            let mut is_float = false;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            // 小数点: '.' の直後が数字の場合のみ浮動小数とする
            // （'..' は Range 演算子なので消費しない）
            if i < n && chars[i] == '.' && i + 1 < n && chars[i + 1].is_ascii_digit() {
                is_float = true;
                i += 1; // '.'
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let num: String = chars[start..i].iter().collect();
            if is_float {
                match num.parse::<f64>() {
                    Ok(f) => tokens.push(Token::FloatLit(f)),
                    Err(_) => return Err(format!("Invalid float literal: {}", num)),
                }
            } else {
                match num.parse::<i64>() {
                    Ok(v) => tokens.push(Token::IntLit(v)),
                    Err(_) => return Err(format!("Invalid integer literal: {}", num)),
                }
            }
            continue;
        }

        // 識別子 / キーワード
        if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }

            let ident: String = chars[start..i].iter().collect();

            let token = match ident.as_str() {
                "fn" => Token::Fn,
                "struct" => Token::Struct,
                "interface" => Token::Interface,
                "state" => Token::State,
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

                _ => Token::Ident(ident),
            };

            tokens.push(token);
            col += i - start;
            continue;
        }

        // 演算子
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

        tokens.push(op);
        col += 1;
    }

    // 末尾のインデントを閉じる
    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Token::Dedent);
    }

    tokens.push(Token::Eof);

    Ok(tokens)
}

// ===== Parser =====
#[derive(Debug, Clone)]
enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    Ident(String),
    BinOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
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
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
enum Pattern {
    Variant {
        name: String,
        bindings: Vec<String>,
    },
    // 予約: パーサからは生成しない（catch-all は else禁止のため不採用）
    Ignore,
}

#[derive(Debug, Clone)]
enum Stmt {
    Let {
        mutable: bool,
        name: String,
        type_hint: Option<String>,
        value: Expr,
    },
    Fn {
        name: String,
        params: Vec<(String, String)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Struct {
        name: String,
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
        variants: Vec<String>,
    },
    Return(Option<Expr>),
    Expr(Expr),
    Assign {
        name: String,
        value: Expr,
    },
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
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
            Err(format!("Expected {:?}, got {:?}", expected, self.current()))
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

            stmts.push(self.parse_stmt()?);
        }

        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.current() {
            Token::Let => self.parse_let(),
            Token::Fn => self.parse_fn(),
            Token::Struct => self.parse_struct(),
            Token::State => self.parse_state(),
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::Return => self.parse_return(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            _ => {
                // 代入文: Ident '=' expr
                if let Token::Ident(name) = self.current().clone() {
                    if self.peek() == &Token::Assign {
                        self.advance(); // Ident
                        self.advance(); // '='
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

        // Lime構文: let [mut] <type>: <name> = <expr>
        // 型推論時は型を省略可: let [mut] <name> = <expr>
        let has_type = match self.current() {
            Token::Int | Token::Float | Token::Str | Token::Bool | Token::Option => true,
            Token::Ident(_) => self.peek() == &Token::Colon,
            _ => false,
        };

        let type_hint = if has_type {
            let t = self.parse_type()?;
            self.expect(Token::Colon)?;
            Some(t)
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

        Ok(Stmt::Let { mutable, name, type_hint, value })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();

        // 改行を消費
        if self.current() == &Token::Newline {
            self.advance();
        }

        // インデント開始
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

        // インデント終了
        if self.current() == &Token::Dedent {
            self.advance();
        }

        Ok(stmts)
    }

    fn parse_fn(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Fn)?;

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected function name".to_string()),
        };
        self.advance();

        self.expect(Token::LParen)?;

        let mut params = Vec::new();

        while self.current() != &Token::RParen {
            // Lime構文: <type>: <name>
            let param_type = self.parse_type()?;

            self.expect(Token::Colon)?;

            let param_name = match self.current() {
                Token::Ident(n) => n.clone(),
                _ => return Err("Expected parameter name".to_string()),
            };

            self.advance();

            params.push((param_name, param_type));

            if self.current() == &Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RParen)?;

        let return_type = match self.current() {
            Token::Int |
            Token::Float |
            Token::Str |
            Token::Bool => {
                Some(self.parse_type()?)
            }

            Token::Ident(name) => {
                let t = name.clone();
                self.advance();
                Some(t)
            }

            _ => None,
        };


        self.expect(Token::Colon)?;

        let body = self.parse_block()?;

        Ok(Stmt::Fn {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_type(&mut self) -> Result<String, String> {
        // Option(T) 記法: Option キーワードの直後が ( なら Generic 引数を解析
        if let Token::Option = self.current() {
            self.advance();
            self.expect(Token::LParen)?;
            let inner = self.parse_type()?;
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
        // T? 省略記法: 後ろに ? が続く場合は Option(T) とする
        if self.current() == &Token::Question {
            self.advance();
            return Ok(format!("Option({})", base));
        }
        Ok(base)
    }

    fn parse_struct(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Struct)?;

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected struct name".to_string()),
        };
        self.advance();

        self.expect(Token::Colon)?;

        // ブロック開始
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

            // メソッド定義
            if self.current() == &Token::Fn {
                methods.push(self.parse_fn()?);
                continue;
            }

            // Lime構文: <type>: <name>
            let field_type = self.parse_type()?;

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

        Ok(Stmt::Struct { name, fields, methods })
    }

    fn parse_state(&mut self) -> Result<Stmt, String> {
        self.expect(Token::State)?;

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            other => return Err(format!("Expected state name, got {:?}", other)),
        };
        self.advance();

        // ジェネリック引数 state Result(T): は今回はスキップ
        if self.current() == &Token::LParen {
            self.advance();
            while self.current() != &Token::RParen && self.current() != &Token::Eof {
                self.advance();
            }
            self.expect(Token::RParen)?;
        }

        self.expect(Token::Colon)?;

        // ブロック開始
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

            // Variant のペイロード指定は今回はスキップ
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

        Ok(Stmt::State { name, variants })
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

        Ok(Stmt::Return(expr))
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

        // 条件式は括弧で囲む（仕様）
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;

        self.expect(Token::Colon)?;

        let body = self.parse_block()?;

        Ok(Stmt::While { cond, body })
    }

    fn parse_match(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Match)?;

        let expr = self.parse_expr()?;
        self.expect(Token::Colon)?;

        // ブロック開始
        if self.current() == &Token::Newline {
            self.advance();
        }
        if self.current() == &Token::Indent {
            self.advance();
        } else {
            return Err("Expected indented match body".to_string());
        }

        let mut arms = Vec::new();
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            if matches!(self.current(), Token::Newline | Token::Indent) {
                self.advance();
                continue;
            }

            // else は禁止
            if self.current() == &Token::Else {
                return Err("Match else is not allowed (exhaustive match required)".to_string());
            }

            // Variant名
            let name = match self.current() {
                Token::Ident(n) => n.clone(),
                other => return Err(format!("Expected variant name in match, got {:?}", other)),
            };
            self.advance();

            // 束縛（Ignore を含む）
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

            self.expect(Token::Colon)?;

            let body = self.parse_block()?;

            arms.push((Pattern::Variant { name, bindings }, body));
        }

        if self.current() == &Token::Dedent {
            self.advance();
        }

        Ok(Stmt::Match { expr, arms })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_binary(0)
    }

    // 演算子優先順位（高→低）: not > * / % > + - > < > <= >= > == != > and > or
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
            _ => self.parse_postfix(),
        }
    }

    // 後置修飾（呼び出し・メソッド・フィールドアクセス）を処理
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_atom()?;

        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let name = match self.current().clone() {
                        Token::Ident(n) => n,
                        // 明示変換メソッド: .int() / .float() / .str()
                        Token::Int => "int".to_string(),
                        Token::Float => "float".to_string(),
                        Token::Str => "str".to_string(),
                        other => return Err(format!("Expected method/field name, got {:?}", other)),
                    };
                    self.advance();
                    if self.current() == &Token::LParen {
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
                            object: Box::new(expr),
                            method: name,
                            args,
                        };
                    } else {
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field: name,
                        };
                    }
                }
                Token::LParen => {
                    // 識別子に対する関数呼び出し
                    if let Expr::Ident(func) = &expr {
                        self.advance();
                        let mut args = Vec::new();
                        while self.current() != &Token::RParen && self.current() != &Token::Eof {
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
                // None は引数なしの Option コンストラクタ（括弧不要）
                if name == "None" {
                    Ok(Expr::Call {
                        func: "None".to_string(),
                        args: vec![],
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            // 明示型変換 API: int(..) / float(..) / str(..)
            // これらは Lexer キーワード (Token::Int 等) だが、
            // 直後に '(' が続く場合は変換呼び出しとして扱う。
            Token::Int | Token::Float | Token::Str => {
                if self.peek() == &Token::LParen {
                    let name = match self.current() {
                        Token::Int => "int".to_string(),
                        Token::Float => "float".to_string(),
                        Token::Str => "str".to_string(),
                        _ => unreachable!(),
                    };
                    self.advance(); // キーワードを消費
                    self.advance(); // '(' を消費
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
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
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
            _ => Err(format!("Unexpected token: {:?}", self.current())),
        }
    }
}

fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

// ===== Simple Interpreter =====
#[derive(Debug, Clone)]
enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Array(Vec<Value>),
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
}

// ユーザー定義関数
#[derive(Clone)]
struct FunctionDef {
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
}

// struct定義（フィールド + メソッド）
#[derive(Clone)]
struct StructDef {
    // フィールド (フィールド名, 型名) を定義順で保持
    fields: Vec<(String, String)>,
    // メソッド名 -> 定義
    methods: HashMap<String, FunctionDef>,
}

// プログラム全体で共有する型定義（読み取り専用）
struct Defs {
    // struct名 -> 定義
    structs: HashMap<String, StructDef>,
    // state variant名 -> 所属する state名
    state_variants: HashMap<String, String>,
    // state名 -> variant名一覧（網羅性検査用）
    states: HashMap<String, Vec<String>>,
    // 関数名 -> 定義
    functions: HashMap<String, FunctionDef>,
}

impl Defs {
    fn new() -> Self {
        Defs {
            structs: HashMap::new(),
            state_variants: HashMap::new(),
            states: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

fn collect_defs(stmts: &[Stmt], defs: &mut Defs) {
    for stmt in stmts {
        match stmt {
            Stmt::Struct { name, fields, methods } => {
                let mut method_map = HashMap::new();
                for m in methods {
                    if let Stmt::Fn { name: mname, params, return_type, body } = m {
                        method_map.insert(
                            mname.clone(),
                            FunctionDef {
                                params: params.clone(),
                                return_type: return_type.clone(),
                                body: body.clone(),
                            },
                        );
                    }
                }
                defs.structs.insert(
                    name.clone(),
                    StructDef {
                        fields: fields.clone(),
                        methods: method_map,
                    },
                );
            }
            Stmt::State { name, variants } => {
                for v in variants {
                    defs.state_variants.insert(v.clone(), name.clone());
                }
                defs.states.insert(name.clone(), variants.clone());
            }
            Stmt::Fn { name, params, return_type, body } => {
                defs.functions.insert(
                    name.clone(),
                    FunctionDef {
                        params: params.clone(),
                        return_type: return_type.clone(),
                        body: body.clone(),
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
        }
    }
}

// ===== Type Checker =====

// 型を表現する。Lime は暗黙型変換を許さないため、厳密な一致を検査する。
// Unknown は「型が判明しない（組込み/StringBuilder等）」または「検査を緩和する」用途。
#[derive(Debug, Clone, PartialEq)]
enum Type {
    Int,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Struct(String),
    State(String),
    List(Box<Type>),
    Option(Box<Type>),
    Unit,
    Unknown,
}

// 変数名 -> 型 を管理する環境
#[derive(Debug, Clone)]
struct TypeEnv {
    vars: HashMap<String, Type>,
}

impl TypeEnv {
    fn new() -> Self {
        TypeEnv {
            vars: HashMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&Type> {
        self.vars.get(name)
    }

    fn insert(&mut self, name: String, ty: Type) {
        self.vars.insert(name, ty);
    }
}

// 型名文字列 -> Type への変換（宣言型と値型の比較に使用）
fn type_from_str(s: &str, defs: &Defs) -> Type {
    // Option(T) または T? 記法をサポート
    if let Some(inner) = s.strip_prefix("Option(") {
        if let Some(inner) = inner.strip_suffix(')') {
            return Type::Option(Box::new(type_from_str(inner, defs)));
        }
    }
    if let Some(inner) = s.strip_suffix('?') {
        return Type::Option(Box::new(type_from_str(inner, defs)));
    }
    match s {
        "int" => Type::Int,
        "float" => Type::Float,
        "bool" => Type::Bool,
        "str" => Type::String,
        _ => {
            if defs.structs.contains_key(s) {
                Type::Struct(s.to_string())
            } else if defs.states.contains_key(s) {
                Type::State(s.to_string())
            } else {
                Type::Unknown
            }
        }
    }
}

// Unknown を含む比較を許容する等価判定
fn type_eq(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Option(inner_a), Type::Option(inner_b)) => type_eq(inner_a, inner_b),
        (Type::List(inner_a), Type::List(inner_b)) => type_eq(inner_a, inner_b),
        (Type::Array(inner_a), Type::Array(inner_b)) => type_eq(inner_a, inner_b),
        _ => a == b,
    }
}

// 式の型を検査し、その型を返す
fn check_expr(expr: &Expr, env: &TypeEnv, defs: &Defs) -> Result<Type, String> {
    match expr {
        Expr::IntLit(_) => Ok(Type::Int),
        Expr::FloatLit(_) => Ok(Type::Float),
        Expr::StringLit(_) => Ok(Type::String),
        Expr::BoolLit(_) => Ok(Type::Bool),

        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Type error: undefined variable '{}'", name)),

        Expr::Range { start, end } => {
            let st = check_expr(start, env, defs)?;
            let et = check_expr(end, env, defs)?;
            if st != Type::Int && st != Type::Unknown {
                return Err(format!("Type error: range start must be int (got {:?})", st));
            }
            if et != Type::Int && et != Type::Unknown {
                return Err(format!("Type error: range end must be int (got {:?})", et));
            }
            // Range は int の List として扱う
            Ok(Type::List(Box::new(Type::Int)))
        }

        Expr::Array(elements) => {
            let mut elem_ty = Type::Unknown;
            for e in elements {
                let t = check_expr(e, env, defs)?;
                if elem_ty == Type::Unknown {
                    elem_ty = t.clone();
                } else if !type_eq(&elem_ty, &t) {
                    return Err(format!(
                        "Type error: list element type mismatch (expected {:?}, got {:?})",
                        elem_ty, t
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

        Expr::BinOp { left, op, right } => {
            let lt = check_expr(left, env, defs)?;
            let rt = check_expr(right, env, defs)?;

            match op.as_str() {
                // 比較演算: 結果は常に Bool
                "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                    if !type_eq(&lt, &rt) {
                        return Err(format!(
                            "Type error: cannot compare {:?} with {:?}",
                            lt, rt
                        ));
                    }
                    Ok(Type::Bool)
                }
                // 論理演算
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
                // 算術演算: 左右同型
                "+" | "-" | "*" | "/" | "%" => {
                    if !type_eq(&lt, &rt) {
                        return Err(format!(
                            "Type error: binary '{}' type mismatch (left {:?}, right {:?})",
                            op, lt, rt
                        ));
                    }
                    // 文字列連結も + で許可
                    Ok(lt)
                }
                other => Err(format!("Type error: unknown binary operator '{}'", other)),
            }
        }

        Expr::Call { func, args } => {
            // 組込み
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
                    // StringBuilder は型モデルにないため Unknown で緩和
                    Ok(Type::Unknown)
                }
                // 明示型変換 API（暗黙変換禁止のための意図的変換）
                // bool 変換は禁止（数値 -> bool 不可）
        "int" | "float" | "str" => {
            if args.len() != 1 {
                return Err(format!(
                    "Type error: {}() takes exactly 1 argument",
                    func
                ));
            }
            // 引数は任意型を受容（Unknown も含む）
            check_expr(&args[0], env, defs)?;
            match func.as_str() {
                "int" => Ok(Type::Int),
                "float" => Ok(Type::Float),
                "str" => Ok(Type::String),
                _ => Ok(Type::Unknown),
            }
        }
        // Option コンストラクタ: Some(x) -> Option(T), None -> Option(Unknown)
        "Some" => {
            if args.len() != 1 {
                return Err("Type error: Some() takes exactly 1 argument".to_string());
            }
            let inner_ty = check_expr(&args[0], env, defs)?;
            Ok(Type::Option(Box::new(inner_ty)))
        }
        "None" => {
            if args.len() != 0 {
                return Err("Type error: None() takes no arguments".to_string());
            }
            Ok(Type::Option(Box::new(Type::Unknown)))
        }
        other => {
                    // Struct constructor
                    if let Some(struct_def) = defs.structs.get(other) {
                        if args.len() != struct_def.fields.len() {
                            return Err(format!(
                                "Type error: {} expects {} field(s), got {}",
                                other,
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
                                return Err(format!(
                                    "Type error: field '{}' of {} expects {:?}, got {:?} (arg {})",
                                    fname, other, expected, at, i
                                ));
                            }
                        }
                        return Ok(Type::Struct(other.to_string()));
                    }

                    // State constructor
                    if let Some(state_name) = defs.state_variants.get(other) {
                        for a in args {
                            check_expr(a, env, defs)?;
                        }
                        return Ok(Type::State(state_name.clone()));
                    }

                    // 関数呼び出し
                    if let Some(fdef) = defs.functions.get(other) {
                        if args.len() != fdef.params.len() {
                            return Err(format!(
                                "Type error: function {} expects {} argument(s), got {}",
                                other,
                                fdef.params.len(),
                                args.len()
                            ));
                        }
                        for ((pname, ptype), arg) in fdef.params.iter().zip(args.iter()) {
                            let at = check_expr(arg, env, defs)?;
                            let expected = type_from_str(ptype, defs);
                            if !type_eq(&at, &expected) {
                                return Err(format!(
                                    "Type error: argument '{}' of {} expects {:?}, got {:?}",
                                    pname, other, expected, at
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
                                return Err(format!(
                                    "Type error: argument '{}' of {}.{} expects {:?}, got {:?}",
                                    pname, name, method, expected, at
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
                // String メソッド（型付き）
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
                    other => Err(format!(
                        "Type error: unknown method '{}' on str",
                        other
                    )),
                },
                // List メソッド（型付き）
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
                    other => Err(format!(
                        "Type error: unknown method '{}' on List",
                        other
                    )),
                },
                // StringBuilder / Array / その他型モデル外: 緩和
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: no method '{}' on type {:?}",
                    method, other
                )),
            }
        }
    }
}

// 文を検査。expected_return は直近の関数の戻り型（指定なしは None）
fn check_stmt(
    stmt: &Stmt,
    env: &mut TypeEnv,
    defs: &Defs,
    expected_return: Option<&Type>,
) -> Result<(), String> {
    match stmt {
        Stmt::Let { name, type_hint, value, .. } => {
            let v_ty = check_expr(value, env, defs)?;
            let declared = match type_hint {
                Some(h) => type_from_str(h, defs),
                None => Type::Unknown,
            };
            if declared != Type::Unknown && !type_eq(&v_ty, &declared) {
                return Err(format!(
                    "Type error: let '{}' expects {:?}, got {:?}",
                    name, declared, v_ty
                ));
            }
            let bind_ty = if declared != Type::Unknown {
                declared
            } else {
                v_ty
            };
            env.insert(name.clone(), bind_ty);
            Ok(())
        }

        Stmt::Return(expr) => match expr {
            Some(e) => {
                let v_ty = check_expr(e, env, defs)?;
                match expected_return {
                    Some(rt) if *rt != Type::Unknown && !type_eq(rt, &v_ty) => Err(format!(
                        "Type error: return type mismatch (expected {:?}, got {:?})",
                        rt, v_ty
                    )),
                    _ => Ok(()),
                }
            }
            None => Ok(()),
        },

        Stmt::If { cond, then_branch, else_branch } => {
            let c_ty = check_expr(cond, env, defs)?;
            if c_ty != Type::Bool && c_ty != Type::Unknown {
                return Err(format!(
                    "Type error: if condition must be bool (got {:?})",
                    c_ty
                ));
            }
            check_stmts(then_branch, env, defs, expected_return)?;
            if let Some(els) = else_branch {
                check_stmts(els, env, defs, expected_return)?;
            }
            Ok(())
        }

        Stmt::For { var, iterable, body } => {
            let iter_ty = check_expr(iterable, env, defs)?;
            // Iterable の要素型をループ変数の型として環境に注入
            let elem_ty = match &iter_ty {
                Type::List(elem) => (&**elem).clone(),
                Type::Array(elem) => (&**elem).clone(),
                _ => Type::Unknown,
            };
            let mut loop_env = env.clone();
            loop_env.insert(var.clone(), elem_ty);
            check_stmts(body, &mut loop_env, defs, expected_return)?;
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
            check_stmts(body, env, defs, expected_return)?;
            Ok(())
        }

        Stmt::Match { expr, arms } => {
            let m_ty = check_expr(expr, env, defs)?;

            // 網羅性検査（State 型の場合のみ）
            if let Type::State(state_name) = &m_ty {
                let variants = defs
                    .states
                    .get(state_name)
                    .cloned()
                    .unwrap_or_default();

                let mut covered: Vec<String> = Vec::new();
                for (pattern, body) in arms {
                    if let Pattern::Variant { name: pname, bindings } = pattern {
                        if !variants.contains(pname) {
                            return Err(format!(
                                "Type error: unknown variant '{}' for state {}",
                                pname, state_name
                            ));
                        }
                        covered.push(pname.clone());

                        // 束縛変数を環境に追加（variant のペイロード型は未保持のため Unknown）
                        let mut arm_env = env.clone();
                        for b in bindings {
                            if b != "Ignore" {
                                arm_env.insert(b.clone(), Type::Unknown);
                            }
                        }
                        check_stmts(body, &mut arm_env, defs, expected_return)?;
                    }
                }

                for v in &variants {
                    if !covered.contains(v) {
                        return Err(format!(
                            "Type error: match on state {} is not exhaustive (missing variant '{}')",
                            state_name, v
                        ));
                    }
                }
            } else if let Type::Option(_) = &m_ty {
                // Option は Some / None の両方を網羅必須
                let variants = vec!["Some".to_string(), "None".to_string()];
                let mut covered: Vec<String> = Vec::new();
                for (pattern, body) in arms {
                    if let Pattern::Variant { name: pname, bindings } = pattern {
                        if !variants.contains(pname) {
                            return Err(format!(
                                "Type error: unknown variant '{}' for Option",
                                pname
                            ));
                        }
                        covered.push(pname.clone());

                        let mut arm_env = env.clone();
                        for b in bindings {
                            if b != "Ignore" {
                                arm_env.insert(b.clone(), Type::Unknown);
                            }
                        }
                        check_stmts(body, &mut arm_env, defs, expected_return)?;
                    }
                }
                for v in &variants {
                    if !covered.contains(v) {
                        return Err(format!(
                            "Type error: match on Option is not exhaustive (missing variant '{}')",
                            v
                        ));
                    }
                }
            } else {
                // State / Option 型以外は各腕のボディのみ検査（束縛なし）
                for (_, body) in arms {
                    check_stmts(body, env, defs, expected_return)?;
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
            // 既存変数への代入（未宣言ならエラー）
            match env.get(name) {
                Some(existing) => {
                    if !type_eq(existing, &v_ty) {
                        return Err(format!(
                            "Type error: assign to '{}' expects {:?}, got {:?}",
                            name, existing, v_ty
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

        // 定義系は collect_defs で登録済み。ここでは本文を検査する。
        Stmt::Fn { .. } => Ok(()),
        Stmt::Struct { .. } => Ok(()),
        Stmt::State { .. } => Ok(()),
        _ => Ok(()),
    }
}

fn check_stmts(
    stmts: &[Stmt],
    env: &mut TypeEnv,
    defs: &Defs,
    expected_return: Option<&Type>,
) -> Result<(), String> {
    for s in stmts {
        check_stmt(s, env, defs, expected_return)?;
    }
    Ok(())
}

// 関数本文を検査（params を環境に注入）
fn check_function(
    params: &[(String, String)],
    return_type: &Option<String>,
    body: &[Stmt],
    defs: &Defs,
) -> Result<(), String> {
    let mut env = TypeEnv::new();
    for (pname, ptype) in params {
        env.insert(pname.clone(), type_from_str(ptype, defs));
    }
    let rt = return_type.as_ref().map(|r| type_from_str(r, defs));
    check_stmts(body, &mut env, defs, rt.as_ref())
}

// プログラム全体の型検査
fn type_check(stmts: &[Stmt], defs: &Defs) -> Result<(), String> {
    for stmt in stmts {
        match stmt {
            Stmt::Fn { name, params, return_type, body } => {
                check_function(params, return_type, body, defs)
                    .map_err(|e| format!("In function '{}': {}", name, e))?;
            }
            Stmt::Struct { name, fields, methods } => {
                // メソッド検査: フィールドを環境に注入
                let mut env = TypeEnv::new();
                for (fname, ftype) in fields {
                    env.insert(fname.clone(), type_from_str(ftype, defs));
                }
                for m in methods {
                    if let Stmt::Fn { name: mname, params, return_type, body } = m {
                        // フィールド環境 + 引数を注入
                        let mut menv = env.clone();
                        for (pname, ptype) in params {
                            menv.insert(pname.clone(), type_from_str(ptype, defs));
                        }
                        let rt = return_type.as_ref().map(|r| type_from_str(r, defs));
                        check_stmts(body, &mut menv, defs, rt.as_ref())
                            .map_err(|e| format!("In method '{}.{}': {}", name, mname, e))?;
                    }
                }
            }
            // トップレベルの実行文（main が無いプログラム用）も検査
            Stmt::Let { .. }
            | Stmt::If { .. }
            | Stmt::Match { .. }
            | Stmt::Return(_)
            | Stmt::Expr(_)
            | Stmt::Assign { .. } => {
                let mut env = TypeEnv::new();
                check_stmt(stmt, &mut env, defs, None)?;
            }
            _ => {}
        }
    }
    Ok(())
}

// String のメソッド評価（.len / .byte_len / .chars / .bytes / .slice）
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
            // 文字単位のインデックスでスライス
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
        other => Err(format!("Unknown String method: {}", other)),
    }
}

// List（Array 値）のメソッド評価（.add / .len / .get / .set）
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

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>, defs: &Defs) -> Result<Value, String> {
    match expr {
        Expr::IntLit(n) => Ok(Value::Int(*n)),
        Expr::FloatLit(f) => Ok(Value::Float(*f)),
        Expr::StringLit(s) => Ok(Value::String(s.clone())),
        Expr::BoolLit(b) => Ok(Value::Bool(*b)),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Undefined variable: {}", name)),
        Expr::BinOp { left, op, right } => {
            let l = eval_expr(left, env, defs)?;
            let r = eval_expr(right, env, defs)?;
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
                _ => Err("Type mismatch in binary operation".to_string()),
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
        Expr::Call { func, args } => {
            // Built-in functions
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
                // 明示型変換 API（暗黙変換禁止）
                // bool 変換は禁止（数値 -> bool 不可）
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
        // Option コンストラクタ: Some(x) -> Option(Some(x)), None -> Option(None)
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
        other => {
                    // Constructor判定: 1. Struct → 2. State Variant → 3. エラー
                    if let Some(struct_def) = defs.structs.get(other) {
                        let field_defs = &struct_def.fields;
                        let mut values = Vec::new();
                        for a in args {
                            values.push(eval_expr(a, env, defs)?);
                        }
                        if values.len() != field_defs.len() {
                            return Err(format!(
                                "{} expects {} field(s), got {}",
                                other,
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
                            name: other.to_string(),
                            fields,
                        })
                    } else if defs.state_variants.contains_key(other) {
                        let mut values = Vec::new();
                        for a in args {
                            values.push(eval_expr(a, env, defs)?);
                        }
                        Ok(Value::State {
                            name: other.to_string(),
                            values,
                        })
                    } else if defs.functions.contains_key(other) {
                        let mut arg_vals = Vec::new();
                        for a in args {
                            arg_vals.push(eval_expr(a, env, defs)?);
                        }
                        call_function(other, arg_vals, defs)
                    } else {
                        Err(format!("Unknown function: {}", func))
                    }
                }
            }
        }
        Expr::MethodCall { object, method, args } => {
            // 引数を先に評価
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(eval_expr(a, env, defs)?);
            }

            // 変数を対象にした呼び出しは書き換えを反映できる
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
                        let result = eval_list_method(&arr, method, &arg_vals)?;
                        // add/set は値を更新、それ以外は一時値を返す
                        if method == "add" || method == "set" {
                            env.insert(var.clone(), result.clone());
                        }
                        Ok(result)
                    }
                    Value::String(s) => eval_string_method(&s, method, &arg_vals),
                    Value::Struct { name, fields } => {
                        call_method(&name, &fields, method, arg_vals, defs)
                    }
                    other => Err(format!(
                        "Type {:?} has no method '{}'",
                        other, method
                    )),
                }
            } else {
                // 一時値に対する読み取り専用メソッド
                let obj = eval_expr(object, env, defs)?;
                match obj {
                    Value::StringBuilder(s) => match method.as_str() {
                        "build" => Ok(Value::String(s)),
                        other => Err(format!("Unknown StringBuilder method: {}", other)),
                    },
                    Value::Array(arr) => eval_list_method(&arr, method, &arg_vals),
                    Value::String(s) => eval_string_method(&s, method, &arg_vals),
                    Value::Struct { name, fields } => {
                        call_method(&name, &fields, method, arg_vals, defs)
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
                    // 終端を含まない（A方式）
                    while i < *b {
                        values.push(Value::Int(i));
                        i += 1;
                    }
                    Ok(Value::Array(values))
                }
                _ => Err("Range requires integer bounds".to_string()),
            }
        }
    }
}

fn call_function(
    name: &str,
    args: Vec<Value>,
    defs: &Defs,
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
    for ((param_name, _param_type), val) in func.params.iter().zip(args.into_iter()) {
        local.insert(param_name.clone(), val);
    }

    match execute_stmts(&func.body, &mut local, defs)? {
        ExecResult::Return(v) => Ok(v),
        ExecResult::Continue => Ok(Value::Int(0)),
    }
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

    // 新しいローカル環境: フィールドを暗黙注入（self/this なし）
    let mut local: HashMap<String, Value> = HashMap::new();
    for (fname, fval) in fields {
        local.insert(fname.clone(), fval.clone());
    }

    // 引数束縛（フィールドと同名なら引数が優先）
    for ((param_name, _param_type), val) in func.params.iter().zip(args.into_iter()) {
        local.insert(param_name.clone(), val);
    }

    match execute_stmts(&func.body, &mut local, defs)? {
        ExecResult::Return(v) => Ok(v),
        ExecResult::Continue => Ok(Value::Int(0)),
    }
}

#[derive(Debug)]
enum ExecResult {
    Continue,
    Return(Value),
}

fn execute_stmts(
    stmts: &[Stmt],
    env: &mut HashMap<String, Value>,
    defs: &Defs,
) -> Result<ExecResult, String> {
    let mut result = ExecResult::Continue;
    for stmt in stmts {
        result = execute_stmt(stmt, env, defs)?;
        if let ExecResult::Return(_) = result {
            break;
        }
    }
    Ok(result)
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
        Stmt::Expr(e) => {
            eval_expr(e, env, defs)?;
            Ok(ExecResult::Continue)
        }
        Stmt::Assign { name, value } => {
            let v = eval_expr(value, env, defs)?;
            env.insert(name.clone(), v);
            Ok(ExecResult::Continue)
        }
        Stmt::Return(expr) => {
            let v = match expr {
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
            // Iterable: List(T) / Range
            let items: Vec<Value> = match val {
                Value::Array(arr) => arr,
                other => return Err(format!("Cannot iterate over {:?}", other)),
            };
            for item in items {
                env.insert(var.clone(), item);
                match execute_stmts(body, env, defs)? {
                    ExecResult::Return(v) => return Ok(ExecResult::Return(v)),
                    ExecResult::Continue => {}
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
                    ExecResult::Continue => {}
                }
            }
            Ok(ExecResult::Continue)
        }

        // 関数定義は collect_defs で登録済み。実行時は何もしない。
        Stmt::Fn { .. } => Ok(ExecResult::Continue),

        Stmt::Match { expr, arms } => {
            let val = eval_expr(expr, env, defs)?;

            match val {
                Value::State { name, values } => {
                    for (pattern, body) in arms {
                        match pattern {
                            Pattern::Variant { name: pname, bindings } => {
                                if pname == &name {
                                    for (idx, binding) in bindings.iter().enumerate() {
                                        if binding != "Ignore" {
                                            let v = values
                                                .get(idx)
                                                .cloned()
                                                .unwrap_or(Value::Int(0));
                                            env.insert(binding.clone(), v);
                                        }
                                    }
                                    return execute_stmts(body, env, defs);
                                }
                            }
                            Pattern::Ignore => {}
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
                                    Pattern::Ignore => {}
                                }
                            }
                            Err("Unhandled Option: Some".to_string())
                        }
                        None => {
                            for (pattern, body) in arms {
                                match pattern {
                                    Pattern::Variant { name: pname, bindings } => {
                                        if pname == "None" {
                                            // None は束縛なし
                                            let _ = bindings;
                                            return execute_stmts(body, env, defs);
                                        }
                                    }
                                    Pattern::Ignore => {}
                                }
                            }
                            Err("Unhandled Option: None".to_string())
                        }
                    }
                }
                other => Err(format!(
                    "match expects a state or option value, got {:?}",
                    other
                )),
            }
        }

        // state 宣言は型定義。実行時は何もしない（意味付けは次段階）
        Stmt::State { .. } => Ok(ExecResult::Continue),

        _ => Ok(ExecResult::Continue),
    }
}
