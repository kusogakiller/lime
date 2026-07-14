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

            // Interface 適合検証（struct が宣言した interface を満たすか）
            if let Err(e) = check_interface_conformance(&defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            // Type Checker�E�実行前に型的に正しいか検査�E�E
            if let Err(e) = type_check(&stmts, &defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            if defs.functions.contains_key("main") {
                if let Err(e) = call_function("main", Vec::new(), &defs) {
                    eprintln!("Runtime error: {}", e);
                }
            } else {
                // main が無ぁE��合�Eトップレベル斁E��実衁E
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

        // 改衁E
        if ch == '\n' {
            tokens.push(Token::Newline);
            line += 1;
            col = 1;
            i += 1;
            // 空行�Eコメント行をスキチE�Eし、次の実コード行�EインチE��トで調整する
            let mut indent = 0usize;
            loop {
                // 行頭の空白を消費してインチE��トを計測
                indent = 0;
                while i < n && (chars[i] == ' ' || chars[i] == '\t') {
                    indent += 1;
                    i += 1;
                }
                if i < n && chars[i] == '#' {
                    // コメント行をスキチE�E
                    while i < n && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                if i < n && chars[i] == '\n' {
                    // 空衁E 改行を記録して次の行へ
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

        // コメンチE
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

        // 斁E���EリチE��ル
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

        // 数孁E
        if ch.is_ascii_digit() {
            let start = i;
            let mut is_float = false;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            // 小数点: '.' の直後が数字�E場合�Eみ浮動小数とする
            // �E�E..' は Range 演算子なので消費しなぁE��E
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

        // 識別孁E/ キーワーチE
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

        // 演算孁E
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

    // 末尾のインチE��トを閉じめE
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
    // 予紁E パ�Eサからは生�EしなぁE��Eatch-all は else禁止のため不採用�E�E
    Ignore,
}

// interface のメソッド署名（本体なし）
#[derive(Debug, Clone)]
struct InterfaceMethod {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
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
        type_params: Vec<String>,
        params: Vec<(String, String)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
    },
    Struct {
        name: String,
        type_params: Vec<String>,
        fields: Vec<(String, String)>,
        methods: Vec<Stmt>,
        implements: Vec<String>,
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
    Interface {
        name: String,
        methods: Vec<InterfaceMethod>,
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
            Token::Interface => self.parse_interface(),
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::Return => self.parse_return(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            _ => {
                // 代入斁E Ident '=' expr
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

        // インチE��ト開姁E
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

        // インチE��ト終亁E
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

        // ジェネリック関数: fn name(T)(args): の (T) 部分
        let type_params = self.parse_type_params(true)?;

        self.expect(Token::LParen)?;

        let mut params = Vec::new();

        while self.current() != &Token::RParen {
            // Lime構文: <type>: <name>  （名前は省略可: 型のみの場合は "_" とする）
            let param_type = self.parse_type()?;

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

        // 署名終端の : （戻り型省略時はこれが終端）
        self.expect(Token::Colon)?;

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

        // 戻り型指定時はさらに終端の : が続く
        if return_type.is_some() {
            self.expect(Token::Colon)?;
        }

        let body = self.parse_block()?;

        Ok(Stmt::Fn {
            name,
            type_params,
            params,
            return_type,
            body,
        })
    }

    fn parse_type(&mut self) -> Result<String, String> {
        // Option(T) 記況E Option キーワード�E直後が ( なめEGeneric 引数を解极E
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
        // T? 省略記況E 後ろに ? が続く場合�E Option(T) とする
        if self.current() == &Token::Question {
            self.advance();
            return Ok(format!("Option({})", base));
        }
        Ok(base)
    }

    // 先読み: 現在位置の (...) が「型引数リスト」として解析できるか試す。
    // 成功すれば Some([型文字列...]) を返し、位置は ) の直後に進む。
    // 失敗(値引数など)なら None を返し、位置は元に戻る。
    fn try_parse_type_args(&mut self) -> Option<Vec<String>> {
        let save = self.pos;
        if self.current() != &Token::LParen {
            return None;
        }
        self.advance(); // '('
        let mut args = Vec::new();
        if self.current() == &Token::RParen {
            self.advance();
            return Some(args);
        }
        loop {
            match self.parse_type() {
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

    // ジェネリチE��型パラメータ: Name(T, U) の (T, U) 部刁E��解极E
    // require_paren_after = true の場吁E関数)、最初�E (...) の直後が ( ならジェネリチE��、E
    //   そうでなければそれは引数リストなのでジェネリチE��無し、E
    // require_paren_after = false の場吁Estruct/state)、E...) があれ�E常にジェネリチE��、E
    fn parse_type_params(&mut self, require_paren_after: bool) -> Result<Vec<String>, String> {
        // 現在ぁE( で、その対応すめE) の直後が ( ならジェネリチE��とみなぁE
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
                            // ) の次のト�Eクンを覗く
                            let next_is_paren =
                                i + 1 < tokens.len() && tokens[i + 1] == Token::LParen;
                            if require_paren_after {
                                if next_is_paren {
                                    // ジェネリチE��あり
                                    break;
                                } else {
                                    // 引数リストとみなぁE
                                    return Ok(Vec::new());
                                }
                            } else {
                                // struct/state: (...) は常にジェネリチE��
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

        // ジェネリチE�� (T, U) を消費
        self.advance(); // '('
        let mut params = Vec::new();
        while self.current() != &Token::RParen && self.current() != &Token::Eof {
            match self.current() {
                Token::Ident(n) => {
                    params.push(n.clone());
                    self.advance();
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

        let type_params = self.parse_type_params(false)?;

        // 任意の interface 適合宣言: struct Dog implements Animal, Pet:
        let mut implements = Vec::new();
        if self.current() == &Token::Ident("implements".to_string()) {
            self.advance();
            loop {
                match self.current() {
                    Token::Ident(n) => {
                        implements.push(n.clone());
                        self.advance();
                    }
                    _ => return Err(format!("Expected interface name, got {:?}", self.current())),
                }
                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(Token::Colon)?;

        // ブロチE��開姁E
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

            // メソチE��定義
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

        Ok(Stmt::Struct {
            name,
            type_params,
            fields,
            methods,
            implements,
        })
    }

    fn parse_interface(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Interface)?;

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected interface name".to_string()),
        };
        self.advance();

        self.expect(Token::Colon)?;

        // ブロチE��開姁E
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

            // メソチE��署名: fn name(params) ret:  （本体なし）
            if self.current() == &Token::Fn {
                self.advance();
                let mname = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err("Expected method name".to_string()),
                };
                self.advance();
                // 型引数は interface では非対象（Phase 1）
                let _ = self.parse_type_params(true);
                self.expect(Token::LParen)?;
                let mut params = Vec::new();
                while self.current() != &Token::RParen && self.current() != &Token::Eof {
                    let param_type = self.parse_type()?;
                    // 名前は省略可能（型のみの署名を許可）
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
                // 署名終端の : （戻り型省略時はこれが終端）
                self.expect(Token::Colon)?;
                let return_type = match self.current() {
                    Token::Int | Token::Float | Token::Str | Token::Bool => {
                        Some(self.parse_type()?)
                    }
                    Token::Ident(rn) => {
                        let t = rn.clone();
                        self.advance();
                        Some(t)
                    }
                    _ => None,
                };
                // 戻り型指定時はさらに終端の : が続く
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

        Ok(Stmt::Interface { name, methods })
    }

    fn parse_state(&mut self) -> Result<Stmt, String> {
        self.expect(Token::State)?;

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            other => return Err(format!("Expected state name, got {:?}", other)),
        };
        self.advance();

        // ジェネリチE��引数 state Result(T): を保持
        let type_params = self.parse_type_params(false)?;

        self.expect(Token::Colon)?;

        // ブロチE��開姁E
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

            // Variant のペイロード指定�E今回はスキチE�E
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

        // 条件式�E括弧で囲む�E�仕様！E
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

        // ブロチE��開姁E
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

            // Variant吁E
            let name = match self.current() {
                Token::Ident(n) => n.clone(),
                other => return Err(format!("Expected variant name in match, got {:?}", other)),
            };
            self.advance();

            // 束縛！Egnore を含む�E�E
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

        eprintln!("DEBUG parse_fn body of '{}' current={:?}", name, self.current());
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

    // 演算子優先頁E��（高�E低！E not > * / % > + - > < > <= >= > == != > and > or
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

    // 後置修飾�E�呼び出し�EメソチE��・フィールドアクセス�E�を処琁E
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_atom()?;

        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let name = match self.current().clone() {
                        Token::Ident(n) => n,
                        // 明示変換メソチE��: .int() / .float() / .str()
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
                    // 識別子に対する呼び出し。ジェネリック構築 Point(int)(1, 2) を判別。
                    if let Expr::Ident(func) = &expr {
                        // 先読み: (...) が型引数リストなら Point(int) として扱う
                        let save = self.pos;
                        if let Some(type_args) =
                            self.try_parse_type_args()
                        {
                            let typed_name = format!("{}({})", func, type_args.join(", "));
                            // 直後に (values) が続けば構築呼び出し
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
                                // (values) が続かない場合は通常の関数呼び出しとする: 位置を戻す
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
                            // 通常の関数呼び出し: 位置を戻して値引数を解析
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
                // None は引数なし�E Option コンストラクタ�E�括弧不要E��E
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
            // これら�E Lexer キーワーチE(Token::Int 筁E だが、E
            // 直後に '(' が続く場合�E変換呼び出しとして扱ぁE��E
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
    type_params: Vec<String>,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
}

// struct定義�E�フィールチE+ メソチE���E�E
#[derive(Clone)]
struct StructDef {
    // ジェネリチE��型パラメータ�E�非ジェネリチE��なら空�E�E
    type_params: Vec<String>,
    // フィールチE(フィールド名, 型名) を定義頁E��保持
    fields: Vec<(String, String)>,
    // メソチE��吁E-> 定義
    methods: HashMap<String, FunctionDef>,
    // 適合する interface 名一覧（暗黙実装の宣言）
    implements: Vec<String>,
}

// interface 定義: メソッド署名の集合
#[derive(Clone)]
struct InterfaceDef {
    methods: Vec<InterfaceMethod>,
}

// プログラム全体で共有する型定義�E�読み取り専用�E�E
struct Defs {
    // struct吁E-> 定義
    structs: HashMap<String, StructDef>,
    // state variant吁E-> 所属すめEstate吁E
    state_variants: HashMap<String, String>,
    // state吁E-> variant名一覧�E�網羁E��検査用�E�E
    states: HashMap<String, Vec<String>>,
    // 関数吁E-> 定義
    functions: HashMap<String, FunctionDef>,
    // interface吁E-> 定義
    interfaces: HashMap<String, InterfaceDef>,
}

impl Defs {
    fn new() -> Self {
        Defs {
            structs: HashMap::new(),
            state_variants: HashMap::new(),
            states: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
        }
    }
}

fn collect_defs(stmts: &[Stmt], defs: &mut Defs) {
    for stmt in stmts {
        match stmt {
            Stmt::Struct {
                name,
                type_params,
                fields,
                methods,
                implements,
            } => {
                let mut method_map = HashMap::new();
                for m in methods {
                    if let Stmt::Fn {
                        name: mname,
                        type_params: mtp,
                        params,
                        return_type,
                        body,
                    } = m
                    {
                        method_map.insert(
                            mname.clone(),
                            FunctionDef {
                                type_params: mtp.clone(),
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
                        type_params: type_params.clone(),
                        fields: fields.clone(),
                        methods: method_map,
                        implements: implements.clone(),
                    },
                );
            }
            Stmt::Interface { name, methods } => {
                defs.interfaces.insert(
                    name.clone(),
                    InterfaceDef {
                        methods: methods.clone(),
                    },
                );
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
                // type_params は Phase 1 では保持のみ�E�未使用�E�E
                let _ = type_params;
            }
            Stmt::Fn {
                name,
                type_params,
                params,
                return_type,
                body,
            } => {
                defs.functions.insert(
                    name.clone(),
                    FunctionDef {
                        type_params: type_params.clone(),
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

// 型を表現する、Eime は暗黙型変換を許さなぁE��め、厳寁E��一致を検査する、E
// Unknown は「型が判明しなぁE��絁E��み/StringBuilder等）」また�E「検査を緩和する」用途、E
#[derive(Debug, Clone, PartialEq)]
enum Type {
    Int,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Struct(String),
    State(String),
    Interface(String),
    List(Box<Type>),
    Option(Box<Type>),
    Unit,
    Unknown,
}

// 変数吁E-> 垁Eを管琁E��る環墁E
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

// 型名斁E���E -> Type への変換�E�宣言型と値型�E比輁E��使用�E�E
fn type_from_str(s: &str, defs: &Defs) -> Type {
    // Option(T) また�E T? 記法をサポ�EチE
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
            // ジェネリチE��型参照 Base(Arg, ...) はベ�Eス名で照吁E
            let base = match s.find('(') {
                Some(i) => &s[..i],
                None => s,
            };
            if defs.structs.contains_key(base) {
                Type::Struct(base.to_string())
            } else if defs.states.contains_key(base) {
                Type::State(base.to_string())
            } else if defs.interfaces.contains_key(base) {
                Type::Interface(base.to_string())
            } else {
                Type::Unknown
            }
        }
    }
}

// Unknown を含む比輁E��許容する等価判宁E
fn type_eq(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Option(inner_a), Type::Option(inner_b)) => type_eq(inner_a, inner_b),
        (Type::List(inner_a), Type::List(inner_b)) => type_eq(inner_a, inner_b),
        (Type::Array(inner_a), Type::Array(inner_b)) => type_eq(inner_a, inner_b),
        _ => a == b,
    }
}

// struct が宣言した interface を実際に満たすか検証する（Phase 1: メソチE�署名一致）
fn check_interface_conformance(defs: &Defs) -> Result<(), String> {
    for (sname, sdef) in &defs.structs {
        for iface_name in &sdef.implements {
            let iface = defs.interfaces.get(iface_name).ok_or_else(|| {
                format!(
                    "Struct '{}' implements unknown interface '{}'",
                    sname, iface_name
                )
            })?;

            for im in &iface.methods {
                let mdef = sdef.methods.get(&im.name).ok_or_else(|| {
                    format!(
                        "Struct '{}' does not implement method '{}' required by interface '{}'",
                        sname, im.name, iface_name
                    )
                })?;

                // 引数数の一致
                if mdef.params.len() != im.params.len() {
                    return Err(format!(
                        "Method '{}' on struct '{}' has {} param(s), but interface '{}' requires {}",
                        im.name, sname, mdef.params.len(), iface_name, im.params.len()
                    ));
                }

                // 引数型の一致（名前は無視、型のみ）
                for (idx, ((_, mptype), (_, iptype))) in
                    mdef.params.iter().zip(im.params.iter()).enumerate()
                {
                    let expected = type_from_str(iptype, defs);
                    let actual = type_from_str(mptype, defs);
                    if !type_eq(&actual, &expected) {
                        return Err(format!(
                            "Method '{}' param {} on struct '{}' has type {:?}, but interface '{}' requires {:?}",
                            im.name, idx, sname, actual, iface_name, expected
                        ));
                    }
                }

                // 戻り型の一致
                let want_ret = match &im.return_type {
                    Some(rt) => type_from_str(rt, defs),
                    None => Type::Unit,
                };
                let got_ret = match &mdef.return_type {
                    Some(rt) => type_from_str(rt, defs),
                    None => Type::Unit,
                };
                if !type_eq(&got_ret, &want_ret) {
                    return Err(format!(
                        "Method '{}' on struct '{}' returns {:?}, but interface '{}' requires {:?}",
                        im.name, sname, got_ret, iface_name, want_ret
                    ));
                }
            }
        }
    }
    Ok(())
}

// struct が interface を実装（宣言）しているか
fn struct_implements(defs: &Defs, struct_name: &str, iface_name: &str) -> bool {
    match defs.structs.get(struct_name) {
        Some(sdef) => sdef.implements.iter().any(|i| i == iface_name),
        None => false,
    }
}

// 型の整合判定（interface への struct 代入・引渡しを許可）
fn type_matches(defs: &Defs, actual: &Type, expected: &Type) -> bool {
    if let (Type::Struct(sname), Type::Interface(iface)) = (actual, expected) {
        if struct_implements(defs, sname, iface) {
            return true;
        }
    }
    type_eq(actual, expected)
}
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
            // Range は int の List として扱ぁE
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
                // 比輁E��箁E 結果は常に Bool
                "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                    if !type_eq(&lt, &rt) {
                        return Err(format!(
                            "Type error: cannot compare {:?} with {:?}",
                            lt, rt
                        ));
                    }
                    Ok(Type::Bool)
                }
                // 論理演箁E
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
                // 算術演箁E 左右同型
                "+" | "-" | "*" | "/" | "%" => {
                    if !type_eq(&lt, &rt) {
                        return Err(format!(
                            "Type error: binary '{}' type mismatch (left {:?}, right {:?})",
                            op, lt, rt
                        ));
                    }
                    // 斁E���E連結も + で許可
                    Ok(lt)
                }
                other => Err(format!("Type error: unknown binary operator '{}'", other)),
            }
        }

        Expr::Call { func, args } => {
            // 絁E��み
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
                    // StringBuilder は型モチE��になぁE��めEUnknown で緩咁E
                    Ok(Type::Unknown)
                }
                // 明示型変換 API�E�暗黙変換禁止のための意図皁E��換�E�E
                // bool 変換は禁止�E�数値 -> bool 不可�E�E
        "int" | "float" | "str" => {
            if args.len() != 1 {
                return Err(format!(
                    "Type error: {}() takes exactly 1 argument",
                    func
                ));
            }
            // 引数は任意型を受容�E�Enknown も含む�E�E
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
                    // Struct / State コンストラクタ�E�ジェネリチE�� Base(Arg) も�Eース名で照合！E
                    let base = match other.find('(') {
                        Some(i) => &other[..i],
                        None => other,
                    };
                    if let Some(struct_def) = defs.structs.get(base) {
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
                                return Err(format!(
                                    "Type error: field '{}' of {} expects {:?}, got {:?} (arg {})",
                                    fname, base, expected, at, i
                                ));
                            }
                        }
                        return Ok(Type::Struct(base.to_string()));
                    }

                    // State constructor
                    if let Some(state_name) = defs.state_variants.get(base) {
                        for a in args {
                            check_expr(a, env, defs)?;
                        }
                        return Ok(Type::State(state_name.clone()));
                    }

                    // 関数呼び出ぁE
                    if let Some(fdef) = defs.functions.get(base) {
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
                                return Err(format!(
                                    "Type error: argument '{}' of {} expects {:?}, got {:?}",
                                    pname, base, expected, at
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
                // Interface 型へのメソチE��呼び出し: interface 署名で検査し、戻り型を返す
                // （実際のディスパチEは実行時の具象 struct メソチEで行われる）
                Type::Interface(iface) => {
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
                            return Err(format!(
                                "Type error: argument of interface {}.{} expects {:?}, got {:?}",
                                iface, method, expected, at
                            ));
                        }
                    }
                    return Ok(match &imsig.return_type {
                        Some(rt) => type_from_str(rt, defs),
                        None => Type::Unit,
                    });
                }
                // String メソチE���E�型付き�E�E
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
                // List メソチE���E�型付き�E�E
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
                // StringBuilder / Array / そ�E他型モチE��夁E 緩咁E
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: no method '{}' on type {:?}",
                    method, other
                )),
            }
        }
    }
}

// 斁E��検査。expected_return は直近�E関数の戻り型�E�指定なし�E None�E�E
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
                // interface 型への代入: 値の具象 struct が interface を実装していれば許可
                if let (Type::Interface(iface), Type::Struct(sname)) = (&declared, &v_ty) {
                    if !struct_implements(defs, sname, iface) {
                        return Err(format!(
                            "Type error: let '{}' expects interface '{}', but struct '{}' does not implement it",
                            name, iface, sname
                        ));
                    }
                } else {
                    return Err(format!(
                        "Type error: let '{}' expects {:?}, got {:?}",
                        name, declared, v_ty
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
            // Iterable の要素型をループ変数の型として環墁E��注入
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

            // 網羁E��検査�E�Etate 型�E場合�Eみ�E�E
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

                        // 束縛変数を環墁E��追加�E�Eariant のペイロード型は未保持のため Unknown�E�E
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
                // Option は Some / None の両方を網羁E��E��E
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
                // State / Option 型以外�E吁E�Eのボディのみ検査�E�束縛なし！E
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
            // 既存変数への代入�E�未宣言ならエラー�E�E
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

        // 定義系は collect_defs で登録済み。ここでは本斁E��検査する、E
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

// 関数本斁E��検査�E�Earams を環墁E��注入�E�E
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

// プログラム全体�E型検査
fn type_check(stmts: &[Stmt], defs: &Defs) -> Result<(), String> {
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                name,
                type_params,
                params,
                return_type,
                body,
            } => {
                let _ = type_params;
                check_function(params, return_type, body, defs)
                    .map_err(|e| format!("In function '{}': {}", name, e))?;
            }
            Stmt::Struct {
                name,
                type_params,
                fields,
                methods,
                implements: _,
            } => {
                // メソチE��検査: フィールドを環墁E��注入
                let mut env = TypeEnv::new();
                for (fname, ftype) in fields {
                    env.insert(fname.clone(), type_from_str(ftype, defs));
                }
                for m in methods {
                    if let Stmt::Fn {
                        name: mname,
                        type_params: _,
                        params,
                        return_type,
                        body,
                    } = m
                    {
                        // フィールド環墁E+ 引数を注入
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
            // トップレベルの実行文�E�Eain が無ぁE�Eログラム用�E�も検査
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

// String のメソチE��評価�E�Elen / .byte_len / .chars / .bytes / .slice�E�E
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
            // 斁E��単位�EインチE��クスでスライス
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

// List�E�Erray 値�E��EメソチE��評価�E�Eadd / .len / .get / .set�E�E
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
                // 明示型変換 API�E�暗黙変換禁止�E�E
                // bool 変換は禁止�E�数値 -> bool 不可�E�E
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
                    // Constructor判宁E 1. Struct ↁE2. State Variant ↁE3. Function ↁE4. エラー
                    // ジェネリチE�� Base(Arg) も�Eース名で照吁E
                    let base = match other.find('(') {
                        Some(i) => &other[..i],
                        None => other,
                    };
                    if let Some(struct_def) = defs.structs.get(base) {
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
                            name: base.to_string(),
                            fields,
                        })
                    } else if defs.state_variants.contains_key(base) {
                        let mut values = Vec::new();
                        for a in args {
                            values.push(eval_expr(a, env, defs)?);
                        }
                        Ok(Value::State {
                            name: base.to_string(),
                            values,
                        })
                    } else if defs.functions.contains_key(base) {
                        let mut arg_vals = Vec::new();
                        for a in args {
                            arg_vals.push(eval_expr(a, env, defs)?);
                        }
                        call_function(base, arg_vals, defs)
                    } else {
                        Err(format!("Unknown function: {}", func))
                    }
                }
            }
        }
        Expr::MethodCall { object, method, args } => {
            // 引数を�Eに評価
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(eval_expr(a, env, defs)?);
            }

            // 変数を対象にした呼び出し�E書き換えを反映できる
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
                        // add/set は値を更新、それ以外�E一時値を返す
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
                // 一時値に対する読み取り専用メソチE��
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
                    // 終端を含まなぁE��E方式！E
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

    // 新しいローカル環墁E フィールドを暗黙注入�E�Eelf/this なし！E
    let mut local: HashMap<String, Value> = HashMap::new();
    for (fname, fval) in fields {
        local.insert(fname.clone(), fval.clone());
    }

    // 引数束縛（フィールドと同名なら引数が優先！E
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

        // 関数定義は collect_defs で登録済み。実行時は何もしなぁE��E
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
                                            // None は束縛なぁE
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

        // state 宣言は型定義。実行時は何もしなぁE��意味付けは次段階！E
        Stmt::State { .. } => Ok(ExecResult::Continue),

        _ => Ok(ExecResult::Continue),
    }
}
