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
        Ok(mut stmts) => {
            println!("=== AST ===");
            for stmt in &stmts {
                println!("{:#?}", stmt);
            }
            println!();

            // Interpreter
            println!("=== Execution ===");
            let mut defs = Defs::new();
            collect_defs(&stmts, &mut defs);

            // Interface 驕ｩ蜷域､懆ｨｼ・・truct 縺悟ｮ｣險縺励◆ interface 繧呈ｺ縺溘☆縺具ｼ・
            if let Err(e) = check_interface_conformance(&defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            // 貍皮ｮ怜ｭ舌ｒ髱咏噪縺ｫ隗｣豎ｺ縺・AST 縺ｫ譖ｸ縺崎ｾｼ繧・亥ｮ溯｡梧凾縺ｯ縺薙・諠・ｱ縺ｮ縺ｿ菴ｿ逕ｨ・・
            resolve_operators_stmts(&mut stmts, &defs);

            // Type Checker・ｽE・ｽ螳溯｡悟燕縺ｫ蝙狗噪縺ｫ豁｣縺励＞縺区､懈渊・ｽE・ｽE
            if let Err(e) = type_check(&stmts, &defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            if defs.functions.contains_key("main") {
                if let Err(e) = call_function("main", Vec::new(), &defs) {
                    eprintln!("Runtime error: {}", e);
                }
            } else {
                // main 縺檎┌縺・・ｽ・ｽ蜷茨ｿｽE繝医ャ繝励Ξ繝吶Ν譁・・ｽ・ｽ螳溯｡・
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

        // 謾ｹ陦・
        if ch == '\n' {
            tokens.push(Token::Newline);
            line += 1;
            col = 1;
            i += 1;
            // 遨ｺ陦鯉ｿｽE繧ｳ繝｡繝ｳ繝郁｡後ｒ繧ｹ繧ｭ繝・・ｽE縺励∵ｬ｡縺ｮ螳溘さ繝ｼ繝芽｡鯉ｿｽE繧､繝ｳ繝・・ｽ・ｽ繝医〒隱ｿ謨ｴ縺吶ｋ
            let mut indent = 0usize;
            loop {
                // 陦碁ｭ縺ｮ遨ｺ逋ｽ繧呈ｶ郁ｲｻ縺励※繧､繝ｳ繝・・ｽ・ｽ繝医ｒ險域ｸｬ
                indent = 0;
                while i < n && (chars[i] == ' ' || chars[i] == '\t') {
                    indent += 1;
                    i += 1;
                }
                if i < n && chars[i] == '#' {
                    // 繧ｳ繝｡繝ｳ繝郁｡後ｒ繧ｹ繧ｭ繝・・ｽE
                    while i < n && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                if i < n && chars[i] == '\n' {
                    // 遨ｺ陦・ 謾ｹ陦後ｒ險倬鹸縺励※谺｡縺ｮ陦後∈
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

        // 繧ｳ繝｡繝ｳ繝・
        if ch == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // 遨ｺ逋ｽ
        if ch == ' ' || ch == '\t' {
            i += 1;
            col += 1;
            continue;
        }

        // 譁・・ｽ・ｽ・ｽE繝ｪ繝・・ｽ・ｽ繝ｫ
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

        // 謨ｰ蟄・
        if ch.is_ascii_digit() {
            let start = i;
            let mut is_float = false;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            // 蟆乗焚轤ｹ: '.' 縺ｮ逶ｴ蠕後′謨ｰ蟄暦ｿｽE蝣ｴ蜷茨ｿｽE縺ｿ豬ｮ蜍募ｰ乗焚縺ｨ縺吶ｋ
            // ・ｽE・ｽE..' 縺ｯ Range 貍皮ｮ怜ｭ舌↑縺ｮ縺ｧ豸郁ｲｻ縺励↑縺・・ｽ・ｽE
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

        // 隴伜挨蟄・/ 繧ｭ繝ｼ繝ｯ繝ｼ繝・
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

                "and" => Token::And,
                "or" => Token::Or,
                "not" => Token::Not,

                _ => Token::Ident(ident),
            };

            tokens.push(token);
            col += i - start;
            continue;
        }

        // 貍皮ｮ怜ｭ・
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

    // 譛ｫ蟆ｾ縺ｮ繧､繝ｳ繝・・ｽ・ｽ繝医ｒ髢峨§繧・
    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Token::Dedent);
    }

    tokens.push(Token::Eof);

    Ok(tokens)
}

// ===== Parser =====

// BinOp 縺ｮ貍皮ｮ怜ｭ占ｧ｣豎ｺ邨先棡・・ypeChecker 縺ｮ縺ｿ縺瑚ｨｭ螳壹＠縲。ackend 縺ｯ縺昴・縺ｾ縺ｾ螳溯｡鯉ｼ・
#[derive(Debug, Clone, PartialEq)]
enum ResolvedOperator {
    // 邨・∩霎ｼ縺ｿ貍皮ｮ暦ｼ・nt/float/str 遲峨・譌｢蟄俶ｼ皮ｮ励ｒ邯ｭ謖・ｼ・
    Builtin,
    // Operator Interface 邨檎罰縺ｮ隗｣豎ｺ: 蜻ｼ縺ｳ蜃ｺ縺吶Γ繧ｽ繝・ラ蜷阪→蜈・・貍皮ｮ怜ｭ舌・
    // 萓・ Add.add / Equal.equal / Compare.compare縲・
    // Interpreter/Backend 縺ｯ method 繧貞他縺ｳ蜃ｺ縺励｛p 縺ｧ邨先棡繧定ｧ｣驥医☆繧・
    // ・・= 縺ｯ equal() 縺ｮ蜷ｦ螳壹・ > <= >= 縺ｯ compare() 縺ｮ隨ｦ蜿ｷ縺ｨ豈碑ｼ・ｼ峨・
    MethodCall { method: String, op: String },
}

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
        // TypeChecker 縺瑚ｧ｣豎ｺ貂医∩諠・ｱ繧呈ｼ邏搾ｼ域悴隗｣豎ｺ譎ゅ・ None・・
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
    // 莠育ｴ・ 繝托ｿｽE繧ｵ縺九ｉ縺ｯ逕滂ｿｽE縺励↑縺・・ｽ・ｽEatch-all 縺ｯ else遖∵ｭ｢縺ｮ縺溘ａ荳肴治逕ｨ・ｽE・ｽE
    Ignore,
}

// interface 縺ｮ繝｡繧ｽ繝・ラ鄂ｲ蜷搾ｼ域悽菴薙↑縺暦ｼ・
#[derive(Debug, Clone)]
struct InterfaceMethod {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
}

// interface 螳夂ｾｩ・医ず繧ｧ繝阪Μ繝・け蝙句ｼ墓焚繧剃ｿ晄戟・・
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
        type_params: Vec<String>,
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
                // 莉｣蜈･譁・ Ident '=' expr
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

        // Lime讒区枚: let [mut] <type>: <name> = <expr>
        // 蝙区耳隲匁凾縺ｯ蝙九ｒ逵∫払蜿ｯ: let [mut] <name> = <expr>
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

        // 謾ｹ陦後ｒ豸郁ｲｻ
        if self.current() == &Token::Newline {
            self.advance();
        }

        // 繧､繝ｳ繝・・ｽ・ｽ繝磯幕蟋・
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

        // 繧､繝ｳ繝・・ｽ・ｽ繝育ｵゆｺ・
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

        // 繧ｸ繧ｧ繝阪Μ繝・け髢｢謨ｰ: fn name(T)(args): 縺ｮ (T) 驛ｨ蛻・
        let type_params = self.parse_type_params(true)?;

        self.expect(Token::LParen)?;

        let mut params = Vec::new();

        while self.current() != &Token::RParen {
            // Lime讒区枚: <type>: <name>  ・亥錐蜑阪・逵∫払蜿ｯ: 蝙九・縺ｿ縺ｮ蝣ｴ蜷医・ "_" 縺ｨ縺吶ｋ・・
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

        // 鄂ｲ蜷咲ｵらｫｯ縺ｮ : ・域綾繧雁梛逵∫払譎ゅ・縺薙ｌ縺檎ｵらｫｯ・・
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

        // 謌ｻ繧雁梛謖・ｮ壽凾縺ｯ縺輔ｉ縺ｫ邨らｫｯ縺ｮ : 縺檎ｶ壹￥
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
        // Option(T) 險俶ｳ・ Option 繧ｭ繝ｼ繝ｯ繝ｼ繝会ｿｽE逶ｴ蠕後′ ( 縺ｪ繧・Generic 蠑墓焚繧定ｧ｣譫・
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
        // T? 逵∫払險俶ｳ・ 蠕後ｍ縺ｫ ? 縺檎ｶ壹￥蝣ｴ蜷茨ｿｽE Option(T) 縺ｨ縺吶ｋ
        if self.current() == &Token::Question {
            self.advance();
            return Ok(format!("Option({})", base));
        }
        Ok(base)
    }

    // 蜈郁ｪｭ縺ｿ: 迴ｾ蝨ｨ菴咲ｽｮ縺ｮ (...) 縺後悟梛蠑墓焚繝ｪ繧ｹ繝医阪→縺励※隗｣譫舌〒縺阪ｋ縺玖ｩｦ縺吶・
    // 謌仙粥縺吶ｌ縺ｰ Some([蝙区枚蟄怜・...]) 繧定ｿ斐＠縲∽ｽ咲ｽｮ縺ｯ ) 縺ｮ逶ｴ蠕後↓騾ｲ繧縲・
    // 螟ｱ謨・蛟､蠑墓焚縺ｪ縺ｩ)縺ｪ繧・None 繧定ｿ斐＠縲∽ｽ咲ｽｮ縺ｯ蜈・↓謌ｻ繧九・
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

    // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蝙九ヱ繝ｩ繝｡繝ｼ繧ｿ: Name(T, U) 縺ｮ (T, U) 驛ｨ蛻・・ｽ・ｽ隗｣譫・
    // require_paren_after = true 縺ｮ蝣ｴ蜷・髢｢謨ｰ)縲∵怙蛻晢ｿｽE (...) 縺ｮ逶ｴ蠕後′ ( 縺ｪ繧峨ず繧ｧ繝阪Μ繝・・ｽ・ｽ縲・
    //   縺昴≧縺ｧ縺ｪ縺代ｌ縺ｰ縺昴ｌ縺ｯ蠑墓焚繝ｪ繧ｹ繝医↑縺ｮ縺ｧ繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ辟｡縺励・
    // require_paren_after = false 縺ｮ蝣ｴ蜷・struct/state)縲・...) 縺後≠繧鯉ｿｽE蟶ｸ縺ｫ繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ縲・
    fn parse_type_params(&mut self, require_paren_after: bool) -> Result<Vec<String>, String> {
        // 迴ｾ蝨ｨ縺・( 縺ｧ縲√◎縺ｮ蟇ｾ蠢懊☆繧・) 縺ｮ逶ｴ蠕後′ ( 縺ｪ繧峨ず繧ｧ繝阪Μ繝・・ｽ・ｽ縺ｨ縺ｿ縺ｪ縺・
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
                            // ) 縺ｮ谺｡縺ｮ繝茨ｿｽE繧ｯ繝ｳ繧定ｦ励￥
                            let next_is_paren =
                                i + 1 < tokens.len() && tokens[i + 1] == Token::LParen;
                            if require_paren_after {
                                if next_is_paren {
                                    // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ縺ゅｊ
                                    break;
                                } else {
                                    // 蠑墓焚繝ｪ繧ｹ繝医→縺ｿ縺ｪ縺・
                                    return Ok(Vec::new());
                                }
                            } else {
                                // struct/state: (...) 縺ｯ蟶ｸ縺ｫ繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ
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

        // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ (T, U) 繧呈ｶ郁ｲｻ
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

        self.expect(Token::Colon)?;

        // 繝悶Ο繝・・ｽ・ｽ髢句ｧ・
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

            // 繝｡繧ｽ繝・・ｽ・ｽ螳夂ｾｩ
            if self.current() == &Token::Fn {
                methods.push(self.parse_fn()?);
                continue;
            }

            // Lime讒区枚: <type>: <name>
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
        })
    }

    fn parse_interface(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Interface)?;

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err("Expected interface name".to_string()),
        };
        self.advance();

        // 繧ｸ繧ｧ繝阪Μ繝・け蝙句ｼ墓焚: interface Add(T):
        let type_params = self.parse_type_params(false)?;

        self.expect(Token::Colon)?;

        // 繝悶Ο繝・・ｽ・ｽ髢句ｧ・
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

            // 繝｡繧ｽ繝・・ｽ・ｽ鄂ｲ蜷・ fn name(params) ret:  ・域悽菴薙↑縺暦ｼ・
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
                    let param_type = self.parse_type()?;
                    // 蜷榊燕縺ｯ逵∫払蜿ｯ閭ｽ・亥梛縺ｮ縺ｿ縺ｮ鄂ｲ蜷阪ｒ險ｱ蜿ｯ・・
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
                // 鄂ｲ蜷咲ｵらｫｯ縺ｮ : ・域綾繧雁梛逵∫払譎ゅ・縺薙ｌ縺檎ｵらｫｯ・・
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
                // 謌ｻ繧雁梛謖・ｮ壽凾縺ｯ縺輔ｉ縺ｫ邨らｫｯ縺ｮ : 縺檎ｶ壹￥
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

        // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蠑墓焚 state Result(T): 繧剃ｿ晄戟
        let type_params = self.parse_type_params(false)?;

        self.expect(Token::Colon)?;

        // 繝悶Ο繝・・ｽ・ｽ髢句ｧ・
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

            // Variant 縺ｮ繝壹う繝ｭ繝ｼ繝画欠螳夲ｿｽE莉雁屓縺ｯ繧ｹ繧ｭ繝・・ｽE
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

        // 譚｡莉ｶ蠑擾ｿｽE諡ｬ蠑ｧ縺ｧ蝗ｲ繧・ｽE・ｽ莉墓ｧ假ｼ・
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

        // 繝悶Ο繝・・ｽ・ｽ髢句ｧ・
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

            // else 縺ｯ遖∵ｭ｢
            if self.current() == &Token::Else {
                return Err("Match else is not allowed (exhaustive match required)".to_string());
            }

            // Variant蜷・
            let name = match self.current() {
                Token::Ident(n) => n.clone(),
                other => return Err(format!("Expected variant name in match, got {:?}", other)),
            };
            self.advance();

            // 譚溽ｸ幢ｼ・gnore 繧貞性繧・ｽE・ｽE
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

    // 貍皮ｮ怜ｭ仙━蜈磯・・ｽ・ｽ・磯ｫ假ｿｽE菴趣ｼ・ not > * / % > + - > < > <= >= > == != > and > or
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
            _ => self.parse_postfix(),
        }
    }

    // 蠕檎ｽｮ菫ｮ鬟ｾ・ｽE・ｽ蜻ｼ縺ｳ蜃ｺ縺暦ｿｽE繝｡繧ｽ繝・・ｽ・ｽ繝ｻ繝輔ぅ繝ｼ繝ｫ繝峨い繧ｯ繧ｻ繧ｹ・ｽE・ｽ繧貞・逅・
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_atom()?;

        loop {
            match self.current() {
                Token::Dot => {
                    self.advance();
                    let name = match self.current().clone() {
                        Token::Ident(n) => n,
                        // 譏守､ｺ螟画鋤繝｡繧ｽ繝・・ｽ・ｽ: .int() / .float() / .str()
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
                    // 隴伜挨蟄舌↓蟇ｾ縺吶ｋ蜻ｼ縺ｳ蜃ｺ縺励ゅず繧ｧ繝阪Μ繝・け讒狗ｯ・Point(int)(1, 2) 繧貞愛蛻･縲・
                    if let Expr::Ident(func) = &expr {
                        // 蜈郁ｪｭ縺ｿ: (...) 縺悟梛蠑墓焚繝ｪ繧ｹ繝医↑繧・Point(int) 縺ｨ縺励※謇ｱ縺・
                        let save = self.pos;
                        if let Some(type_args) =
                            self.try_parse_type_args()
                        {
                            let typed_name = format!("{}({})", func, type_args.join(", "));
                            // 逶ｴ蠕後↓ (values) 縺檎ｶ壹￠縺ｰ讒狗ｯ牙他縺ｳ蜃ｺ縺・
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
                                // (values) 縺檎ｶ壹°縺ｪ縺・ｴ蜷医・騾壼ｸｸ縺ｮ髢｢謨ｰ蜻ｼ縺ｳ蜃ｺ縺励→縺吶ｋ: 菴咲ｽｮ繧呈綾縺・
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
                            // 騾壼ｸｸ縺ｮ髢｢謨ｰ蜻ｼ縺ｳ蜃ｺ縺・ 菴咲ｽｮ繧呈綾縺励※蛟､蠑墓焚繧定ｧ｣譫・
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
                // None 縺ｯ蠑墓焚縺ｪ縺暦ｿｽE Option 繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ・ｽE・ｽ諡ｬ蠑ｧ荳崎ｦ・・ｽ・ｽE
                if name == "None" {
                    Ok(Expr::Call {
                        func: "None".to_string(),
                        args: vec![],
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            // 譏守､ｺ蝙句､画鋤 API: int(..) / float(..) / str(..)
            // 縺薙ｌ繧会ｿｽE Lexer 繧ｭ繝ｼ繝ｯ繝ｼ繝・(Token::Int 遲・ 縺縺後・
            // 逶ｴ蠕後↓ '(' 縺檎ｶ壹￥蝣ｴ蜷茨ｿｽE螟画鋤蜻ｼ縺ｳ蜃ｺ縺励→縺励※謇ｱ縺・・ｽ・ｽE
            Token::Int | Token::Float | Token::Str => {
                if self.peek() == &Token::LParen {
                    let name = match self.current() {
                        Token::Int => "int".to_string(),
                        Token::Float => "float".to_string(),
                        Token::Str => "str".to_string(),
                        _ => unreachable!(),
                    };
                    self.advance(); // 繧ｭ繝ｼ繝ｯ繝ｼ繝峨ｒ豸郁ｲｻ
                    self.advance(); // '(' 繧呈ｶ郁ｲｻ
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

// 繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ髢｢謨ｰ
#[derive(Clone)]
struct FunctionDef {
    type_params: Vec<String>,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
}

// struct螳夂ｾｩ・ｽE・ｽ繝輔ぅ繝ｼ繝ｫ繝・+ 繝｡繧ｽ繝・・ｽ・ｽ・ｽE・ｽE
#[derive(Clone)]
struct StructDef {
    // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蝙九ヱ繝ｩ繝｡繝ｼ繧ｿ・ｽE・ｽ髱槭ず繧ｧ繝阪Μ繝・・ｽ・ｽ縺ｪ繧臥ｩｺ・ｽE・ｽE
    type_params: Vec<String>,
    // 繝輔ぅ繝ｼ繝ｫ繝・(繝輔ぅ繝ｼ繝ｫ繝牙錐, 蝙句錐) 繧貞ｮ夂ｾｩ鬆・・ｽ・ｽ菫晄戟
    fields: Vec<(String, String)>,
    // 繝｡繧ｽ繝・・ｽ・ｽ蜷・-> 螳夂ｾｩ
    methods: HashMap<String, FunctionDef>,
}

// interface 螳夂ｾｩ: 繧ｸ繧ｧ繝阪Μ繝・け蝙句ｼ墓焚 + 繝｡繧ｽ繝・ラ鄂ｲ蜷阪・髮・粋
#[derive(Clone)]
struct InterfaceDef {
    type_params: Vec<String>,
    methods: Vec<InterfaceMethod>,
}

// 繝励Ο繧ｰ繝ｩ繝蜈ｨ菴薙〒蜈ｱ譛峨☆繧句梛螳夂ｾｩ・ｽE・ｽ隱ｭ縺ｿ蜿悶ｊ蟆ら畑・ｽE・ｽE
struct Defs {
    // struct蜷・-> 螳夂ｾｩ
    structs: HashMap<String, StructDef>,
    // state variant蜷・-> 謇螻槭☆繧・state蜷・
    state_variants: HashMap<String, String>,
    // state蜷・-> variant蜷堺ｸ隕ｧ・ｽE・ｽ邯ｲ鄒・・ｽ・ｽ讀懈渊逕ｨ・ｽE・ｽE
    states: HashMap<String, Vec<String>>,
    // 髢｢謨ｰ蜷・-> 螳夂ｾｩ
    functions: HashMap<String, FunctionDef>,
    // interface蜷・-> 螳夂ｾｩ
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
                    },
                );
            }
            Stmt::Interface {
                name,
                type_params,
                methods,
            } => {
                eprintln!("DEBUG collect interface name={} nmethods={}", name, methods.len());
                defs.interfaces.insert(
                    name.clone(),
                    InterfaceDef {
                        type_params: type_params.clone(),
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
                // type_params 縺ｯ Phase 1 縺ｧ縺ｯ菫晄戟縺ｮ縺ｿ・ｽE・ｽ譛ｪ菴ｿ逕ｨ・ｽE・ｽE
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

// 蝙九ｒ陦ｨ迴ｾ縺吶ｋ縲・ime 縺ｯ證鈴ｻ吝梛螟画鋤繧定ｨｱ縺輔↑縺・・ｽ・ｽ繧√∝宍蟇・・ｽ・ｽ荳閾ｴ繧呈､懈渊縺吶ｋ縲・
// Unknown 縺ｯ縲悟梛縺悟愛譏弱＠縺ｪ縺・・ｽ・ｽ邨・・ｽ・ｽ縺ｿ/StringBuilder遲会ｼ峨阪∪縺滂ｿｽE縲梧､懈渊繧堤ｷｩ蜥後☆繧九咲畑騾斐・
#[derive(Debug, Clone, PartialEq)]
enum Type {
    Int,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Struct(String),
    State(String),
    Interface(String, Vec<Type>),
    List(Box<Type>),
    Option(Box<Type>),
    Unit,
    Unknown,
}

// 螟画焚蜷・-> 蝙・繧堤ｮ｡逅・・ｽ・ｽ繧狗腸蠅・
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

// 蝙句錐譁・・ｽ・ｽ・ｽE -> Type 縺ｸ縺ｮ螟画鋤・ｽE・ｽ螳｣險蝙九→蛟､蝙具ｿｽE豈碑ｼ・・ｽ・ｽ菴ｿ逕ｨ・ｽE・ｽE
fn type_from_str(s: &str, defs: &Defs) -> Type {
    // Option(T) 縺ｾ縺滂ｿｽE T? 險俶ｳ輔ｒ繧ｵ繝晢ｿｽE繝・
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
            // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蝙句盾辣ｧ Base(Arg, ...) 縺ｯ繝呻ｿｽE繧ｹ蜷阪〒辣ｧ蜷・
            let base = match s.find('(') {
                Some(i) => &s[..i],
                None => s,
            };
            if defs.structs.contains_key(base) {
                Type::Struct(base.to_string())
            } else if defs.states.contains_key(base) {
                Type::State(base.to_string())
            } else if defs.interfaces.contains_key(base) {
                // 蝙句ｼ墓焚繧呈歓蜃ｺ: Interface(T) / Interface()
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
                Type::Unknown
            }
        }
    }
}

// Unknown 繧貞性繧豈碑ｼ・・ｽ・ｽ險ｱ螳ｹ縺吶ｋ遲我ｾ｡蛻､螳・
fn type_eq(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Option(inner_a), Type::Option(inner_b)) => type_eq(inner_a, inner_b),
        (Type::List(inner_a), Type::List(inner_b)) => type_eq(inner_a, inner_b),
        (Type::Array(inner_a), Type::Array(inner_b)) => type_eq(inner_a, inner_b),
        _ => a == b,
    }
}

// interface 繝｡繧ｽ繝・ラ鄂ｲ蜷阪→ struct 繝｡繧ｽ繝・ラ鄂ｲ蜷阪′荳閾ｴ縺吶ｋ縺・
// ・亥錐蜑阪・蜷御ｸ縺ｨ莉ｮ螳壹＠縲∝ｼ墓焚謨ｰ繝ｻ蠑墓焚蝙九・謌ｻ繧雁梛繧堤・蜷茨ｼ・
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

// struct 縺・interface 繧呈囓鮟吶↓螳溯｣・＠縺ｦ縺・ｋ縺・
// ・・nterface 縺ｮ蜈ｨ繝｡繧ｽ繝・ラ繧偵∽ｸ閾ｴ縺吶ｋ鄂ｲ蜷阪〒謖√▲縺ｦ縺・ｋ縺薙→・・
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

// 蝙区枚蟄怜・荳ｭ縺ｮ蝙九ヱ繝ｩ繝｡繝ｼ繧ｿ繧・arg 縺ｫ鄂ｮ謠幢ｼ・eneric Operator Interface 縺ｮ螳滉ｽ灘喧・・
fn subst_type(t: &str, type_params: &[String], arg: &str) -> String {
    let mut result = t.to_string();
    for tp in type_params {
        result = result.replace(tp, arg);
    }
    result
}

// struct 縺・interface 繧貞梛蠑墓焚 arg 縺ｧ螳溯｣・＠縺ｦ縺・ｋ縺具ｼ井ｾ・ Add(Point)・・
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

// 莠碁・ｼ皮ｮ怜ｭ舌ｒ Operator Interface 邨檎罰縺ｧ隗｣豎ｺ縺吶ｋ・医Θ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・縺ｿ・・
// 謌ｻ繧雁､: (繝｡繧ｽ繝・ラ蜷・ 邨先棡蝙・縲らｵ・∩霎ｼ縺ｿ蝙九ｄ隗｣豎ｺ荳崎・縺ｪ蝣ｴ蜷医・ None縲・
fn resolve_operator_interface(
    defs: &Defs,
    lt: &Type,
    rt: &Type,
    op: &str,
) -> Option<(String, Type)> {
    // 荳｡霎ｺ縺悟酔荳縺ｮ繝ｦ繝ｼ繧ｶ繝ｼ struct 蝙九・縺ｨ縺阪・縺ｿ Interface 隗｣豎ｺ繧定ｩｦ縺ｿ繧・
    let sname = match (lt, rt) {
        (Type::Struct(a), Type::Struct(b)) if a == b => a.clone(),
        _ => return None,
    };
    // 邨・∩霎ｼ縺ｿ謨ｰ蛟､繝ｻ譁・ｭ怜・縺ｯ蠕捺擂縺ｮ邨・∩霎ｼ縺ｿ貍皮ｮ励ｒ蜆ｪ蜈医☆繧九◆繧√％縺薙〒縺ｯ隗｣豎ｺ縺励↑縺・
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

// TypeChecker 蟆ら畑縺ｮ AST 繝代せ: 蜷・BinOp 縺ｫ隗｣豎ｺ貂医∩貍皮ｮ怜ｭ舌ｒ譖ｸ縺崎ｾｼ繧縲・
// 螳溯｡梧凾・・nterpreter/LLVM Backend・峨・縺薙・諠・ｱ縺縺代ｒ隕九※貍皮ｮ励☆繧九◆繧√・
// Runtime 縺ｧ縺ｮ蝙区､懃ｴ｢繧・Struct 蜷阪°繧峨・ Interface 讀懃ｴ｢縺ｯ荳蛻・｡後ｏ縺ｪ縺・・
//
// 隗｣豎ｺ縺ｯ蝙区､懈渊迺ｰ蠅・ｼ・et 譚溽ｸ帙・蠑墓焚縺ｮ蝙具ｼ峨°繧蛾撕逧・↓陦後≧縲ゅ％縺ｮ迺ｰ蠅・・
// 譛ｬ繝代せ蜀・〒 let / fn 繧定ｵｰ譟ｻ縺励※讒狗ｯ峨☆繧具ｼ・heck_expr 縺ｮ full env 縺ｯ荳崎ｦ・ｼ峨・
fn resolve_operators_stmts(stmts: &mut [Stmt], defs: &Defs) {
    let mut env: HashMap<String, Type> = HashMap::new();
    for s in stmts.iter_mut() {
        resolve_operators_stmt(s, defs, &mut env);
    }
}

fn resolve_operators_stmt(s: &mut Stmt, defs: &Defs, env: &mut HashMap<String, Type>) {
    match s {
        Stmt::Expr(e) => resolve_operators_expr(e, defs, env),
        Stmt::Let { name, value, .. } => {
            if let Ok(t) = infer_type(value, env, defs) {
                env.insert(name.clone(), t);
            }
            resolve_operators_expr(value, defs, env);
        }
        Stmt::Assign { value, .. } => resolve_operators_expr(value, defs, env),
        Stmt::If { cond, then_branch, else_branch } => {
            resolve_operators_expr(cond, defs, env);
            resolve_operators_stmts(then_branch, defs);
            if let Some(b) = else_branch {
                resolve_operators_stmts(b, defs);
            }
        }
        Stmt::While { cond, body } => {
            resolve_operators_expr(cond, defs, env);
            resolve_operators_stmts(body, defs);
        }
        Stmt::For { var, iterable, body } => {
            // iterable 縺ｮ隕∫ｴ蝙九ｒ var 縺ｮ蝙九→縺励※迺ｰ蠅・↓霑ｽ蜉
            if let Ok(it_ty) = infer_type(iterable, env, defs) {
                let elem = match &it_ty {
                    Type::List(e) => (**e).clone(),
                    _ => Type::Unknown,
                };
                env.insert(var.clone(), elem);
            }
            resolve_operators_expr(iterable, defs, env);
            resolve_operators_stmts(body, defs);
        }
        Stmt::Return(Some(e)) => resolve_operators_expr(e, defs, env),
        Stmt::Match { expr, arms, .. } => {
            resolve_operators_expr(expr, defs, env);
            for (_, body) in arms.iter_mut() {
                resolve_operators_stmts(body, defs);
            }
        }
        Stmt::Fn { params, body, .. } => {
            let mut fenv = env.clone();
            for (pname, ptype) in params {
                fenv.insert(pname.clone(), type_from_str(ptype, defs));
            }
            resolve_operators_stmts(body, defs);
            // 豕ｨ: Fn 蜀・・迺ｰ蠅・・蜻ｼ縺ｳ蜃ｺ縺玲ｯ弱↓讒狗ｯ峨＆繧後ｋ縺溘ａ縲∝､門・ env 縺ｫ縺ｯ蜿肴丐縺励↑縺・
            let _ = fenv;
        }
        Stmt::Struct { methods, .. } => {
            resolve_operators_stmts(methods, defs);
        }
        _ => {}
    }
}

// 貍皮ｮ怜ｭ占ｧ｣豎ｺ縺ｮ縺溘ａ縺ｮ霆ｽ驥丞梛謗ｨ隲厄ｼ・nv 縺ｯ let/蠑墓焚縺ｮ蝙九・縺ｿ・峨・
// check_expr 縺ｮ full env 縺ｯ荳崎ｦ√よ綾繧雁､縺悟ｾ励ｉ繧後↑縺・ｴ蜷医・ Unknown 繧定ｿ斐☆縲・
fn infer_type(e: &Expr, env: &HashMap<String, Type>, defs: &Defs) -> Result<Type, String> {
    match e {
        Expr::IntLit(_) => Ok(Type::Int),
        Expr::FloatLit(_) => Ok(Type::Float),
        Expr::StringLit(_) => Ok(Type::String),
        Expr::BoolLit(_) => Ok(Type::Bool),
        Expr::Ident(n) => env
            .get(n)
            .cloned()
            .ok_or_else(|| format!("undefined variable '{}'", n)),
        Expr::Call { func, args } => {
            if defs.structs.contains_key(func) {
                Ok(Type::Struct(func.clone()))
            } else if defs.states.contains_key(func) {
                Ok(Type::Struct(func.clone()))
            } else if func == "Some" && args.len() == 1 {
                Ok(Type::Option(Box::new(infer_type(&args[0], env, defs)?)))
            } else if func == "None" {
                Ok(Type::Option(Box::new(Type::Unknown)))
            } else if let Some(f) = defs.functions.get(func) {
                match &f.return_type {
                    Some(rt) => Ok(type_from_str(rt, defs)),
                    None => Ok(Type::Unit),
                }
            } else {
                Ok(Type::Unknown)
            }
        }
        Expr::MethodCall { object, method, .. } => {
            let ot = infer_type(object, env, defs)?;
            match ot {
                Type::Struct(s) => {
                    if let Some(sd) = defs.structs.get(&s) {
                        if let Some(m) = sd.methods.get(method) {
                            if let Some(rt) = &m.return_type {
                                return Ok(type_from_str(rt, defs));
                            }
                        }
                    }
                    Ok(Type::Unknown)
                }
                _ => Ok(Type::Unknown),
            }
        }
        Expr::UnOp { operand, .. } => infer_type(operand, env, defs),
        Expr::BinOp { left, op, right, .. } => {
            let lt = infer_type(left, env, defs)?;
            let rt = infer_type(right, env, defs)?;
            if let Some((_, t)) = resolve_operator_interface(defs, &lt, &rt, op) {
                Ok(t)
            } else {
                match op.as_str() {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "and" | "or" => Ok(Type::Bool),
                    _ => Ok(lt),
                }
            }
        }
        Expr::FieldAccess { object, field } => {
            let ot = infer_type(object, env, defs)?;
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
                let et = infer_type(first, env, defs)?;
                Ok(Type::List(Box::new(et)))
            } else {
                Ok(Type::List(Box::new(Type::Unknown)))
            }
        }
        Expr::Range { .. } => Ok(Type::List(Box::new(Type::Int))),
        _ => Ok(Type::Unknown),
    }
}

fn resolve_operators_expr(e: &mut Expr, defs: &Defs, env: &HashMap<String, Type>) {
    match e {
        Expr::BinOp { left, right, op, resolved_operator } => {
            // 蟄舌ｒ蜈医↓隗｣豎ｺ・医ロ繧ｹ繝医＠縺・BinOp 繧ょ性繧・・
            resolve_operators_expr(left, defs, env);
            resolve_operators_expr(right, defs, env);
            let lt = infer_type(left, env, defs);
            let rt = infer_type(right, env, defs);
            let res = match (lt, rt) {
                (Ok(lt), Ok(rt)) => {
                    match resolve_operator_interface(defs, &lt, &rt, op) {
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
        Expr::UnOp { operand, .. } => resolve_operators_expr(operand, defs, env),
        Expr::Call { args, .. } => {
            for a in args.iter_mut() {
                resolve_operators_expr(a, defs, env);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            resolve_operators_expr(object, defs, env);
            for a in args.iter_mut() {
                resolve_operators_expr(a, defs, env);
            }
        }
        Expr::FieldAccess { object, .. } => resolve_operators_expr(object, defs, env),
        Expr::Array(items) => {
            for it in items.iter_mut() {
                resolve_operators_expr(it, defs, env);
            }
        }
        Expr::Range { start, end } => {
            resolve_operators_expr(start, defs, env);
            resolve_operators_expr(end, defs, env);
        }
        _ => {}
    }
}


// 證鈴ｻ吝ｮ溯｣・・讀懆ｨｼ: 縺吶∋縺ｦ縺ｮ struct 縺ｫ縺､縺・※縲√Γ繧ｽ繝・ラ蜷阪′ interface 縺ｨ
// 荳閾ｴ縺吶ｋ縺檎ｽｲ蜷阪′逡ｰ縺ｪ繧句ｴ蜷医・繧ｨ繝ｩ繝ｼ・郁ｦｪ蛻・↑險ｺ譁ｭ縺ｮ縺溘ａ・・
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
            // 蜈ｨ繝｡繧ｽ繝・ラ蜷阪′謠・▲縺ｦ縺・ｋ縺ｪ繧峨∫ｽｲ蜷阪ｂ荳閾ｴ縺励※縺・ｋ蠢・ｦ√′縺ゅｋ
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

// struct 縺・interface 繧呈囓鮟吝ｮ溯｣・＠縺ｦ縺・ｋ縺・
fn struct_implements(defs: &Defs, struct_name: &str, iface_name: &str) -> bool {
    struct_satisfies_interface(defs, struct_name, iface_name)
}

// 蝙九・謨ｴ蜷亥愛螳夲ｼ・nterface 縺ｸ縺ｮ struct 莉｣蜈･繝ｻ蠑墓ｸ｡縺励ｒ險ｱ蜿ｯ・・
fn type_matches(defs: &Defs, actual: &Type, expected: &Type) -> bool {
    if let (Type::Struct(sname), Type::Interface(iface, _)) = (actual, expected) {
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
            // Range 縺ｯ int 縺ｮ List 縺ｨ縺励※謇ｱ縺・
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

        Expr::BinOp { left, op, right, .. } => {
            let lt = check_expr(left, env, defs)?;
            let rt = check_expr(right, env, defs)?;

            match op.as_str() {
                // 豈碑ｼ・・ｽ・ｽ邂・ 邨先棡縺ｯ蟶ｸ縺ｫ Bool
                "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                    if let Some((_, result_ty)) =
                        resolve_operator_interface(defs, &lt, &rt, op)
                    {
                        // 繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・ Operator Interface 縺ｧ隗｣豎ｺ
                        Ok(result_ty)
                    } else if !type_eq(&lt, &rt) {
                        return Err(format!(
                            "Type error: cannot compare {:?} with {:?}",
                            lt, rt
                        ));
                    } else {
                        Ok(Type::Bool)
                    }
                }
                // 隲也炊貍皮ｮ・
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
                // 邂苓｡捺ｼ皮ｮ・ 蟾ｦ蜿ｳ蜷悟梛
                "+" | "-" | "*" | "/" | "%" => {
                    if let Some((_, result_ty)) =
                        resolve_operator_interface(defs, &lt, &rt, op)
                    {
                        // 繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・ Operator Interface 縺ｧ隗｣豎ｺ
                        Ok(result_ty)
                    } else if !type_eq(&lt, &rt) {
                        return Err(format!(
                            "Type error: binary '{}' type mismatch (left {:?}, right {:?})",
                            op, lt, rt
                        ));
                    } else {
                        // 譁・・ｽ・ｽ・ｽE騾｣邨舌ｂ + 縺ｧ險ｱ蜿ｯ
                        Ok(lt)
                    }
                }
                other => Err(format!("Type error: unknown binary operator '{}'", other)),
            }
        }

        Expr::Call { func, args } => {
            // 邨・・ｽ・ｽ縺ｿ
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
                    // StringBuilder 縺ｯ蝙九Δ繝・・ｽ・ｽ縺ｫ縺ｪ縺・・ｽ・ｽ繧・Unknown 縺ｧ邱ｩ蜥・
                    Ok(Type::Unknown)
                }
                // 譏守､ｺ蝙句､画鋤 API・ｽE・ｽ證鈴ｻ吝､画鋤遖∵ｭ｢縺ｮ縺溘ａ縺ｮ諢丞峙逧・・ｽ・ｽ謠幢ｿｽE・ｽE
                // bool 螟画鋤縺ｯ遖∵ｭ｢・ｽE・ｽ謨ｰ蛟､ -> bool 荳榊庄・ｽE・ｽE
        "int" | "float" | "str" => {
            if args.len() != 1 {
                return Err(format!(
                    "Type error: {}() takes exactly 1 argument",
                    func
                ));
            }
            // 蠑墓焚縺ｯ莉ｻ諢丞梛繧貞女螳ｹ・ｽE・ｽEnknown 繧ょ性繧・ｽE・ｽE
            check_expr(&args[0], env, defs)?;
            match func.as_str() {
                "int" => Ok(Type::Int),
                "float" => Ok(Type::Float),
                "str" => Ok(Type::String),
                _ => Ok(Type::Unknown),
            }
        }
        // Option 繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ: Some(x) -> Option(T), None -> Option(Unknown)
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
                    // Struct / State 繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ・ｽE・ｽ繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ Base(Arg) 繧ゑｿｽE繝ｼ繧ｹ蜷阪〒辣ｧ蜷茨ｼ・
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

                    // 髢｢謨ｰ蜻ｼ縺ｳ蜃ｺ縺・
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
                // Interface 蝙九∈縺ｮ繝｡繧ｽ繝・・ｽ・ｽ蜻ｼ縺ｳ蜃ｺ縺・ interface 鄂ｲ蜷阪〒讀懈渊縺励∵綾繧雁梛繧定ｿ斐☆
                // ・亥ｮ滄圀縺ｮ繝・ぅ繧ｹ繝代メE縺ｯ螳溯｡梧凾縺ｮ蜈ｷ雎｡ struct 繝｡繧ｽ繝・縺ｧ陦後ｏ繧後ｋ・・
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
                // String 繝｡繧ｽ繝・・ｽ・ｽ・ｽE・ｽ蝙倶ｻ倥″・ｽE・ｽE
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
                // List 繝｡繧ｽ繝・・ｽ・ｽ・ｽE・ｽ蝙倶ｻ倥″・ｽE・ｽE
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
                // StringBuilder / Array / 縺晢ｿｽE莉門梛繝｢繝・・ｽ・ｽ螟・ 邱ｩ蜥・
                Type::Unknown => Ok(Type::Unknown),
                other => Err(format!(
                    "Type error: no method '{}' on type {:?}",
                    method, other
                )),
            }
        }
    }
}

// 譁・・ｽ・ｽ讀懈渊縲Ｆxpected_return 縺ｯ逶ｴ霑托ｿｽE髢｢謨ｰ縺ｮ謌ｻ繧雁梛・ｽE・ｽ謖・ｮ壹↑縺暦ｿｽE None・ｽE・ｽE
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
                // interface 蝙九∈縺ｮ莉｣蜈･: 蛟､縺ｮ蜈ｷ雎｡ struct 縺・interface 繧貞ｮ溯｣・＠縺ｦ縺・ｌ縺ｰ險ｱ蜿ｯ
                if let (Type::Interface(iface, _), Type::Struct(sname)) = (&declared, &v_ty) {
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
            // Iterable 縺ｮ隕∫ｴ蝙九ｒ繝ｫ繝ｼ繝怜､画焚縺ｮ蝙九→縺励※迺ｰ蠅・・ｽ・ｽ豕ｨ蜈･
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

            // 邯ｲ鄒・・ｽ・ｽ讀懈渊・ｽE・ｽEtate 蝙具ｿｽE蝣ｴ蜷茨ｿｽE縺ｿ・ｽE・ｽE
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

                        // 譚溽ｸ帛､画焚繧堤腸蠅・・ｽ・ｽ霑ｽ蜉・ｽE・ｽEariant 縺ｮ繝壹う繝ｭ繝ｼ繝牙梛縺ｯ譛ｪ菫晄戟縺ｮ縺溘ａ Unknown・ｽE・ｽE
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
                // Option 縺ｯ Some / None 縺ｮ荳｡譁ｹ繧堤ｶｲ鄒・・ｽ・ｽE・ｽ・ｽE
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
                // State / Option 蝙倶ｻ･螟厄ｿｽE蜷・・ｽE縺ｮ繝懊ョ繧｣縺ｮ縺ｿ讀懈渊・ｽE・ｽ譚溽ｸ帙↑縺暦ｼ・
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
            // 譌｢蟄伜､画焚縺ｸ縺ｮ莉｣蜈･・ｽE・ｽ譛ｪ螳｣險縺ｪ繧峨お繝ｩ繝ｼ・ｽE・ｽE
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

        // 螳夂ｾｩ邉ｻ縺ｯ collect_defs 縺ｧ逋ｻ骭ｲ貂医∩縲ゅ％縺薙〒縺ｯ譛ｬ譁・・ｽ・ｽ讀懈渊縺吶ｋ縲・
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

// 髢｢謨ｰ譛ｬ譁・・ｽ・ｽ讀懈渊・ｽE・ｽEarams 繧堤腸蠅・・ｽ・ｽ豕ｨ蜈･・ｽE・ｽE
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

// 繝励Ο繧ｰ繝ｩ繝蜈ｨ菴難ｿｽE蝙区､懈渊
fn type_check(stmts: &[Stmt], defs: &Defs) -> Result<(), String> {
    let mut top_env = TypeEnv::new();
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
            } => {
                // 繝｡繧ｽ繝・・ｽ・ｽ讀懈渊: 繝輔ぅ繝ｼ繝ｫ繝峨ｒ迺ｰ蠅・・ｽ・ｽ豕ｨ蜈･
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
                        // 繝輔ぅ繝ｼ繝ｫ繝臥腸蠅・+ 蠑墓焚繧呈ｳｨ蜈･
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
            // 繝医ャ繝励Ξ繝吶Ν縺ｮ螳溯｡梧枚・・ain 縺檎┌縺・・繝ｭ繧ｰ繝ｩ繝逕ｨ・峨ｂ讀懈渊縲・
            // 繝医ャ繝励Ξ繝吶Ν縺ｮ let 蜷悟｣ｫ縺悟盾辣ｧ縺怜粋縺医ｋ繧医≧縲∝・譛・env 繧剃ｽｿ縺・・
            Stmt::Let { name, value, .. } => {
                let _ = check_expr(value, &top_env, defs)?;
                let v_ty = infer_type(value, &top_env.vars, defs);
                let mut env = top_env.clone();
                if let Ok(t) = &v_ty {
                    env.insert(name.clone(), t.clone());
                }
                check_stmt(stmt, &mut env, defs, None)?;
                if let Ok(t) = v_ty {
                    top_env.insert(name.clone(), t);
                }
            }
            Stmt::If { .. }
            | Stmt::Match { .. }
            | Stmt::Return(_)
            | Stmt::Expr(_)
            | Stmt::Assign { .. } => {
                let mut env = top_env.clone();
                check_stmt(stmt, &mut env, defs, None)?;
            }
            _ => {}
        }
    }
    Ok(())
}

// String 縺ｮ繝｡繧ｽ繝・・ｽ・ｽ隧穂ｾ｡・ｽE・ｽElen / .byte_len / .chars / .bytes / .slice・ｽE・ｽE
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
            // 譁・・ｽ・ｽ蜊倅ｽ搾ｿｽE繧､繝ｳ繝・・ｽ・ｽ繧ｯ繧ｹ縺ｧ繧ｹ繝ｩ繧､繧ｹ
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

// List・ｽE・ｽErray 蛟､・ｽE・ｽ・ｽE繝｡繧ｽ繝・・ｽ・ｽ隧穂ｾ｡・ｽE・ｽEadd / .len / .get / .set・ｽE・ｽE
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
        Expr::BinOp { left, op, right, resolved_operator } => {
            let l = eval_expr(left, env, defs)?;
            let r = eval_expr(right, env, defs)?;
            // 解決済み演算子のみを見て実行（Runtime での型検索は行わない）
            match resolved_operator {
                Some(ResolvedOperator::MethodCall { method, op: mop }) => {
                    // 繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙・ 蟾ｦ霎ｺ縺ｮ繝｡繧ｽ繝・ラ繧貞他縺ｳ蜃ｺ縺励｛p 縺ｧ邨先棡繧定ｧ｣驥・
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
                    // 邨・∩霎ｼ縺ｿ蝙九・譌｢蟄俶ｼ皮ｮ暦ｼ・nt/float/str・・
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
                // 譏守､ｺ蝙句､画鋤 API・ｽE・ｽ證鈴ｻ吝､画鋤遖∵ｭ｢・ｽE・ｽE
                // bool 螟画鋤縺ｯ遖∵ｭ｢・ｽE・ｽ謨ｰ蛟､ -> bool 荳榊庄・ｽE・ｽE
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
        // Option 繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ: Some(x) -> Option(Some(x)), None -> Option(None)
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
                    // Constructor蛻､螳・ 1. Struct 竊・2. State Variant 竊・3. Function 竊・4. 繧ｨ繝ｩ繝ｼ
                    // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ Base(Arg) 繧ゑｿｽE繝ｼ繧ｹ蜷阪〒辣ｧ蜷・
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
            // 蠑墓焚繧抵ｿｽE縺ｫ隧穂ｾ｡
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(eval_expr(a, env, defs)?);
            }

            // 螟画焚繧貞ｯｾ雎｡縺ｫ縺励◆蜻ｼ縺ｳ蜃ｺ縺暦ｿｽE譖ｸ縺肴鋤縺医ｒ蜿肴丐縺ｧ縺阪ｋ
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
                        // add/set 縺ｯ蛟､繧呈峩譁ｰ縲√◎繧御ｻ･螟厄ｿｽE荳譎ょ､繧定ｿ斐☆
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
                // 荳譎ょ､縺ｫ蟇ｾ縺吶ｋ隱ｭ縺ｿ蜿悶ｊ蟆ら畑繝｡繧ｽ繝・・ｽ・ｽ
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
                    // 邨らｫｯ繧貞性縺ｾ縺ｪ縺・・ｽ・ｽE譁ｹ蠑擾ｼ・
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
        ExecResult::Continue => {
            // 譛ｫ蟆ｾ縺悟ｼ乗枚縺ｪ繧峨◎縺ｮ蛟､繧呈囓鮟呵ｿ泌唆・亥ｼ乗欠蜷代Γ繧ｽ繝・ラ/髢｢謨ｰ・・
            if let Some(Stmt::Expr(e)) = func.body.last() {
                eval_expr(e, &mut local, defs)
            } else {
                Ok(Value::Int(0))
            }
        }
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

    // 譁ｰ縺励＞繝ｭ繝ｼ繧ｫ繝ｫ迺ｰ蠅・ 繝輔ぅ繝ｼ繝ｫ繝峨ｒ證鈴ｻ呎ｳｨ蜈･・ｽE・ｽEelf/this 縺ｪ縺暦ｼ・
    let mut local: HashMap<String, Value> = HashMap::new();
    for (fname, fval) in fields {
        local.insert(fname.clone(), fval.clone());
    }

    // 蠑墓焚譚溽ｸ幢ｼ医ヵ繧｣繝ｼ繝ｫ繝峨→蜷悟錐縺ｪ繧牙ｼ墓焚縺悟━蜈茨ｼ・
    for ((param_name, _param_type), val) in func.params.iter().zip(args.into_iter()) {
        local.insert(param_name.clone(), val);
    }

    match execute_stmts(&func.body, &mut local, defs)? {
        ExecResult::Return(v) => Ok(v),
        ExecResult::Continue => {
            // 譛ｫ蟆ｾ縺悟ｼ乗枚縺ｪ繧峨◎縺ｮ蛟､繧呈囓鮟呵ｿ泌唆・亥ｼ乗欠蜷代Γ繧ｽ繝・ラ/髢｢謨ｰ・・
            if let Some(Stmt::Expr(e)) = func.body.last() {
                eval_expr(e, &mut local, defs)
            } else {
                Ok(Value::Int(0))
            }
        }
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

        // 髢｢謨ｰ螳夂ｾｩ縺ｯ collect_defs 縺ｧ逋ｻ骭ｲ貂医∩縲ょｮ溯｡梧凾縺ｯ菴輔ｂ縺励↑縺・・ｽ・ｽE
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
                                            // None 縺ｯ譚溽ｸ帙↑縺・
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

        // state 螳｣險縺ｯ蝙句ｮ夂ｾｩ縲ょｮ溯｡梧凾縺ｯ菴輔ｂ縺励↑縺・・ｽ・ｽ諢丞袖莉倥￠縺ｯ谺｡谿ｵ髫趣ｼ・
        Stmt::State { .. } => Ok(ExecResult::Continue),

        _ => Ok(ExecResult::Continue),
    }
}
