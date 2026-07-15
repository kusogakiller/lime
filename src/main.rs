use std::env;
use std::fs;
use std::collections::HashMap;

// Phase 0 (Step 10): LLVM backend foundation (textual IR emitter).
// Inkwell 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
// 謨ｰ蛟､縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｯ蜈ｷ雎｡繝｡繧ｽ繝・ラ蜷代″縺ｮ LLVM IR 縺ｦ榆帥蜴ｻ縺ｦ縺ｪ縺ｿ縺ｪ縺・
#[path = "codegen/mod.rs"]
mod codegen;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: lime <file.lime> [--emit-ll]");
        return;
    }

    // Phase 0 (Step 10): LLVM backend foundation. --emit-ll 縺ｯ LLVM IR (text) 縺ｦ
    // 蟋ｩ蜉ｨ縺ｮ .ll 縺ｯ榆帥蜴ｻ縺ｦ縺ｪ縺ｿ縺ｪ縺・(Inkwell 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
    let emit_ll = args.iter().any(|a| a == "--emit-ll");
    let source_path = if args[1].starts_with("--") {
        eprintln!("Usage: lime <file.lime> [--emit-ll]");
        return;
    } else {
        args[1].clone()
    };

    let source = match fs::read_to_string(&source_path) {
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

            // Defs 縺ｮ蝙九・繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・ (call_function 縺ｯ defs 縺ｮ蜻ｼ縺ｳ蜃ｺ縺ｮ縺ｧ繧｢蜷阪→) 縺ｮ resolved_operator 縺ｦ縺ｪ縺ｿ縺ｪ縺・
            resolve_operators_defs(&mut defs);

            // Interface 驕ｩ蜷域､懆ｨｼ・・truct 縺悟ｮ｣險縺励◆ interface 繧呈ｺ縺溘☆縺具ｼ・
            if let Err(e) = check_interface_conformance(&defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            // 貍皮ｮ怜ｭ舌ｒ髱咏噪縺ｫ隗｣豎ｺ縺・AST 縺ｫ譖ｸ縺崎ｾｼ繧・亥ｮ溯｡梧凾縺ｯ縺薙・諠・ｱ縺ｮ縺ｿ菴ｿ逕ｨ・・
            let empty_cons: HashMap<String, Vec<String>> = HashMap::new();
            let empty_env: HashMap<String, Type> = HashMap::new();
            resolve_operators_stmts(&mut stmts, &defs, &empty_cons, &empty_env);

            // Type Checker・ｽE・ｽ螳溯｡悟燕縺ｫ蝙狗噪縺ｫ豁｣縺励＞縺区､懈渊・ｽE・ｽE
            if let Err(e) = type_check(&stmts, &defs) {
                eprintln!("Type error: {}", e);
                return;
            }

            // Phase 6: Generic Monomorphization (after type check, before memory analysis)
            if let Err(e) = monomorphize_all(&mut defs, &mut stmts) {
                eprintln!("Type error: {}", e);
                return;
            }

            // Memory Analysis (Escape Analysis) - Step 9
            let memory = match memory_analyze(&stmts, &defs) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };

            // Phase 0 (Step 10): LLVM backend foundation.
            // --emit-ll 縺ｯ LLVM IR (text) 縺ｦ .ll 縺ｯ榆帥蜴ｻ縺ｦ縺ｪ縺ｿ縺ｪ縺・
            if emit_ll {
                let out = codegen::emit_llvm(&stmts, &defs, &memory);
                let base = source_path.trim_end_matches(".lime");
                let ll_path = format!("{}.ll", base);
                match fs::write(&ll_path, &out) {
                    Ok(_) => eprintln!("LLVM IR written to {}", ll_path),
                    Err(e) => eprintln!("Failed to write LLVM IR: {}", e),
                }
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
    Fn, Lime, Struct, Interface, State, Let, Mut, If, Else, Match, Return,
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
                "lime" => Token::Lime,
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
pub enum ResolvedOperator {
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
    // 蜈郁ｪｭ繝｡繧ｽ繝・ラ蜷代″縺ｮ await 縺ｧ繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE
    // 蝗ｲ繧・ｽE・ｽRuntime 縺ｮ繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE(lime 繧｡繧ｽ繝・ラ蝣ｴ蜷隗｣譫怜ｸ・繝｡繧ｽ繝・ラ蜷代″)
    Await(Box<Expr>),
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
        // 蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE: heap / stack 縺ｯ繧｢蜷阪→縺ｮ險ｱ蜿ｯ縺ｧ繧｢蜷阪→
        // (let Type(heap): / let Type(stack): 縺ｯ繧｢蜷阪→)。None 縺ｯ Escape Analysis 縺ｯ蝣ｴ蜷隗｣譫・
        place: Option<MemoryPlace>,
    },
    Fn {
        name: String,
        type_params: Vec<String>,
        constraints: Vec<(String, String)>,
        params: Vec<(String, String)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
        // lime 繧｡繧ｽ繝・ラ蜷代″縺ｮ true (fn 縺ｯ false)。await 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
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
    Interface {
        name: String,
        type_params: Vec<String>,
        constraints: Vec<(String, String)>,
        methods: Vec<InterfaceMethod>,
    },
    Return(Option<Expr>),
    Expr(Expr),
    Assign {
        name: String,
        value: Expr,
    },
}

// ===== Memory Model (Step 9: Escape Analysis) =====
// GC 縺ｽ繝ｼ、譛ｬ蜈･繝｡繧ｽ繝・ラ蜷代″險ｱ蜿ｯ縺ｧ繧｢蜷阪→縺ｮ萓��､ｸ縺ｮ縺ｿ縺ｪ縺・
// 蝣ｴ蜷隗｣譫怜ｸ・螳溯｡梧凾縺ｯ繝ｼ繝牙ｒ繝・・ｽ・ｽ縺ｿ縺ｪ縺・
#[derive(Debug, Clone, Copy, PartialEq)]
enum MemoryPlace {
    Stack,
    Heap,
}

// Struct 縺ｮ蝠ｩ蜊ｫ: ｿｽE・ｽE螃ｻ蜈ｰ蝗ｲ繧縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE
// heap / stack 縺ｯ繧｢蜷阪→縺ｮ險ｱ蜿ｯ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・(let Type(heap): 縺ｯ繧｢蜷阪→)
// 蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｿ縺ｪ縺・None 縺ｯ蜈ｷ雎｡蜷代″險ｱ蜿ｯ縺ｧ繧｢蜷阪→(Escape Analysis 縺ｯ蝣ｴ蜷隗｣譫・)
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
            Token::Fn | Token::Lime => self.parse_fn(),
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
        //          let [mut] <type>(heap): <name> = <expr>  (蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE)
        //          let [mut] <type>(stack): <name> = <expr>
        // 蝙区耳隲匁凾縺ｯ蝙九ｒ逵∫払蜿ｯ: let [mut] <name> = <expr>
        let has_type = match self.current() {
            Token::Int | Token::Float | Token::Str | Token::Bool | Token::Option => true,
            Token::Ident(_) => {
                // <type>: ... 縺ｯ繧｢蜷阪→ / <type>(heap): ... 縺ｯ繧｢蜷阪→
                if self.peek() == &Token::Colon {
                    true
                } else if self.peek() == &Token::LParen
                    && *self.peek_at(2) != Token::Ident("heap".to_string())
                    && *self.peek_at(2) != Token::Ident("stack".to_string())
                {
                    // Generic: <type>(Inner): 縺ｯ繧｢蜷阪→縺ｮ縺ｿ縺ｪ縺・
                    // 棘ｷ繝ｼ繝怜､画焚 heap/stack 縺ｯ繧｢蜷阪→縺ｮ縺ｿ縺ｪ縺・
                    false
                } else if self.peek() == &Token::LParen {
                    // <type>(heap)/(stack) 縺ｯ繧｢蜷阪→縺ｮ縺ｿ縺ｪ縺・
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        let mut place: Option<MemoryPlace> = None;

        let type_hint = if has_type {
            // 蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE Type(heap) / Type(stack) 縺ｯ繧｢蜷阪→縺ｮ險ｱ蜿ｯ縺ｧ繧｢蜷阪→
            // parse_type 縺ｯ (...) 縺ｦ Generic 縺ｾ縺滂ｿｽE蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
            // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｿ縺ｪ縺・parse_type 縺ｯ蝠ｩ蜊ｫ縺ｮ蛟､蝙具ｿｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
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
                    // User(heap) / User(stack) 縺ｯ繧｢蜷阪→: 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・(User) 縺ｦ蝣ｴ蜷隗｣譫・
                    self.advance(); // User (base type)
                    self.advance(); // (
                    self.advance(); // heap / stack
                    self.advance(); // )
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
        // fn (同期) または lime (非同期) を許可。両者は戻り値型システムを完全共有。
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

        // 繧ｸ繧ｧ繝阪Μ繝・け髢｢謨ｰ: fn name(T)(args): 縺ｮ (T) 驛ｨ蛻・
        let mut constraints = Vec::new();
        let mut type_params = self.parse_type_params(true, &mut constraints)?;

        self.expect(Token::LParen)?;

        let mut params = Vec::new();

        while self.current() != &Token::RParen {
            // Lime讒区枚: <type>: <name>  ・亥錐蜑阪・逵∫払蜿ｯ: 蝙九・縺ｿ縺ｮ蝣ｴ蜷医・ "_" 縺ｨ縺吶ｋ・・
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

        // 鄂ｲ蜷咲ｵらｫｯ縺ｮ : ・域綾繧雁梛逵∫払譎ゅ・縺薙ｌ縺檎ｵらｫｯ・・
        self.expect(Token::Colon)?;

        let return_type = match self.current() {
            Token::Int |
            Token::Float |
            Token::Str |
            Token::Bool |
            Token::Ident(_) |
            Token::Option => {
                Some(self.parse_type(&mut constraints)?)
            }

            _ => None,
        };

        // 謌ｻ繧雁梛謖・ｮ壽凾縺ｯ縺輔ｉ縺ｫ邨らｫｯ縺ｮ : 縺檎ｶ壹￥
        if return_type.is_some() {
            self.expect(Token::Colon)?;
        }

        let body = self.parse_block()?;

        // constraint 縺ｮ蝙区枚蟄怜・縺ｪ type_params 縺ｮ蝣ｴ蜷隗｣譫・
        for (tv, _) in &constraints {
            if !type_params.contains(tv) {
                type_params.push(tv.clone());
            }
        }
        // return_type縺ｮ蝙区枚蟄怜・縺ｯ type_params縺ｫ蝣ｴ蜷・繧峨☆縺ｪ縺・
        // (concrete type name 縺ｨ type parameter 縺ｮ蝙区､懈淵縺ｯ parse蛟､縺ｧ縺ｯ蝣ｴ蜷医☆)
        // type_params縺ｯ fn name(T)(...) 縺ｮ (T) 縺ｯ繧｢蜷阪→縺ｫ縺励※縺ｯ蜷医ｒ縲√ｒ蝗ｲ繧

        Ok(Stmt::Fn {
            name,
            type_params,
            constraints,
            params,
            return_type,
            body,
            is_async,
        })
    }

    fn parse_type(&mut self, constraints: &mut Vec<(String, String)>) -> Result<String, String> {
        // Option(T) 險俶ｳ・ Option 繧ｭ繝ｼ繝ｯ繝ｼ繝会ｿｽE逶ｴ蠕後′ ( 縺ｪ繧・Generic 蠑墓焚繧定ｧ｣譫・
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
        // T? 逵∫払險俶ｳ・ 蠕後ｍ縺ｫ ? 縺檎ｶ壹￥蝣ｴ蜷茨ｿｽE Option(T) 縺ｨ縺吶ｋ
        if self.current() == &Token::Question {
            self.advance();
            return Ok(format!("Option({})", base));
        }
        // Generic type args: Base(Inner, ...) 縺ｯ繝・・ｽ・ｽ List(T) / Option(T) 繧定ｿ斐＠縲∝ｽｮ縺ｮ縺ｿ縺ｪ縺・
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
        // Generic constraint: T where T: Iface (, T: Iface)*
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

    // 蜈郁ｪｭ縺ｿ: 迴ｾ蝨ｨ菴咲ｽｮ縺ｮ (...) 縺後悟梛蠑墓焚繝ｪ繧ｹ繝医阪→縺励※隗｣譫舌〒縺阪ｋ縺玖ｩｦ縺吶・
    // 謌仙粥縺吶ｌ縺ｰ Some([蝙区枚蟄怜・...]) 繧定ｿ斐＠縲∽ｽ咲ｽｮ縺ｯ ) 縺ｮ逶ｴ蠕後↓騾ｲ繧縲・
    // 螟ｱ謨・蛟､蠑墓焚縺ｪ縺ｩ)縺ｪ繧・None 繧定ｿ斐＠縲∽ｽ咲ｽｮ縺ｯ蜈・↓謌ｻ繧九・
    fn try_parse_type_args(&mut self, constraints: &mut Vec<(String, String)>) -> Option<Vec<String>> {
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

    // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蝙九ヱ繝ｩ繝｡繝ｼ繧ｿ: Name(T, U) 縺ｮ (T, U) 驛ｨ蛻・・ｽ・ｽ隗｣譫・
    // require_paren_after = true 縺ｮ蝣ｴ蜷・髢｢謨ｰ)縲∵怙蛻晢ｿｽE (...) 縺ｮ逶ｴ蠕後′ ( 縺ｪ繧峨ず繧ｧ繝阪Μ繝・・ｽ・ｽ縲・
    //   縺昴≧縺ｧ縺ｪ縺代ｌ縺ｰ縺昴ｌ縺ｯ蠑墓焚繝ｪ繧ｹ繝医↑縺ｮ縺ｧ繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ辟｡縺励・
    // require_paren_after = false 縺ｮ蝣ｴ蜷・struct/state)縲・...) 縺後≠繧鯉ｿｽE蟶ｸ縺ｫ繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ縲・
    fn parse_type_params(
        &mut self,
        require_paren_after: bool,
        constraints: &mut Vec<(String, String)>,
    ) -> Result<Vec<String>, String> {
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

        // 繧ｸ繧ｧ繝阪Μ繝・け蝙句ｼ墓焚: interface Add(T):
        let mut constraints = Vec::new();
        let type_params = self.parse_type_params(false, &mut constraints)?;

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
                    let param_type = self.parse_type(&mut constraints)?;
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
                        Some(self.parse_type(&mut constraints)?)
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

        // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蠑墓焚 state Result(T): 繧剃ｿ晄戟
        let type_params = self.parse_type_params(false, &mut Vec::new())?;

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

        // 譚｡莉ｶ蠑擾ｿｽE諡ｬ蠑ｧ縺ｧ蝗ｲ繧・ｽE・ｽ莉墓ｧ假ｼ・: while cond: (縺ｮ ( ) 縺ｦ縺ｪ縺・)
        let cond = self.parse_expr()?;

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
            Token::Await => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Await(Box::new(operand)))
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
                            self.try_parse_type_args(&mut Vec::new())
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
    // lime 繧｡繧ｽ繝・ラ蜷代″縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE: 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
    // 蜈郁ｪｭ縺ｿ縺ｧ繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE(lime 繧｡繧ｽ繝・ラ蝣ｴ蜷隗｣譫怜ｸ・)縲∝ｽ硅 Country.縺ｮ縺ｪ縺ｿ縺ｪ縺・
    // await 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
    Future {
        func: String,
        args: Vec<Value>,
    },
}

// 繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ髢｢謨ｰ
#[derive(Clone)]
struct FunctionDef {
    type_params: Vec<String>,
    // Generic 蝙区枚蟄怜・ -> 蟷ｻ蜈･繝｡繧ｽ繝・ラ鄂ｲ蜷阪・: (type_param, iface)
    constraints: Vec<(String, String)>,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
    // lime 繧｡繧ｽ繝・ラ蜷代″縺ｮ true (fn 縺ｯ false)
    is_async: bool,
}

// struct螳夂ｾｩ・ｽE・ｽ繝輔ぅ繝ｼ繝ｫ繝・+ 繝｡繧ｽ繝・・ｽ・ｽ・ｽE・ｽE
#[derive(Clone)]
struct StructDef {
    // 繧ｸ繧ｧ繝阪Μ繝・・ｽ・ｽ蝙九ヱ繝ｩ繝｡繝ｼ繧ｿ・ｽE・ｽ髱槭ず繧ｧ繝阪Μ繝・・ｽ・ｽ縺ｪ繧臥ｩｺ・ｽE・ｽE
    type_params: Vec<String>,
    // Generic 蝙区枚蟄怜・ -> 蟷ｻ蜈･繝｡繧ｽ繝・ラ鄂ｲ蜷阪・: (type_param, iface)
    constraints: Vec<(String, String)>,
    // 繝輔ぅ繝ｼ繝ｫ繝・(繝輔ぅ繝ｼ繝ｫ繝牙錐, 蝙句錐) 繧貞ｮ夂ｾｩ鬆・・ｽ・ｽ菫晄戟
    fields: Vec<(String, String)>,
    // 繝｡繧ｽ繝・・ｽ・ｽ蜷・-> 螳夂ｾｩ
    methods: HashMap<String, FunctionDef>,
}

// interface 螳夂ｾｩ: 繧ｸ繧ｧ繝阪Μ繝・け蝙句ｼ墓焚 + 繝｡繧ｽ繝・ラ鄂ｲ蜷阪・髮・粋
#[derive(Clone)]
struct InterfaceDef {
    type_params: Vec<String>,
    constraints: Vec<(String, String)>,
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
        let mut defs = Defs {
            structs: HashMap::new(),
            state_variants: HashMap::new(),
            states: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
        };
        // Result(T, E) 繧ｳ繝ｳ繧ｹ繝医Λ繧ｯ繧ｿ蜊ｻ蜉: 蟷ｻ蜈･繝｡繧ｽ繝・ラ鄂ｲ蜷阪→縺ｮ繝｡繧ｽ繝・ラ/State 繝｡繧ｽ繝・ラ縺ｮ繝繧ｿ蜿悶ｊ蟆ら畑
        // 蟷ｻ蜈･ Success / Error 縺ｮ蝠�社･繝｡繧ｽ繝・ラ蜷代″縺ｮ蝙句錐譁・險繝｡繧ｽ繝・ラ隕ｧ・ｽE・ｽ繧｢蜷阪→
        // Ok / Err 縺ｯ莉｣蜈･纏ｯ縺ｿ縺ｪ縺・
        defs.state_variants.insert("Success".to_string(), "Result".to_string());
        defs.state_variants.insert("Error".to_string(), "Result".to_string());
        defs.states.insert(
            "Result".to_string(),
            vec!["Success".to_string(), "Error".to_string()],
        );
        defs
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
                        return_type,
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
                                return_type: return_type.clone(),
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
                constraints,
                params,
                return_type,
                body,
                is_async,
            } => {
                defs.functions.insert(
                    name.clone(),
                    FunctionDef {
                        type_params: type_params.clone(),
                        constraints: constraints.clone(),
                        params: params.clone(),
                        return_type: return_type.clone(),
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
    // Generic 蝙区枚蟄怜・縺ｮ蝙九→蜷・: T 縺ｯ繝莠ｧ｣縺ｿ繝・・ｽ・ｽ闡ｺ縺・
    Var(String),
}

// 螟画焚蜷・-> 蝙・繧堤ｮ｡逅・・ｽ・ｽ繧狗腸蠅・
#[derive(Debug, Clone)]
struct TypeEnv {
    vars: HashMap<String, Type>,
    // Generic 蝙区枚蟄怜・ -> 蟷ｻ蜈･繝｡繧ｽ繝・ラ鄂ｲ蜷阪・蝣ｴ蜷・縲∝ｽｮ縺ｮ縺ｿ縺ｪ縺・
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
    // List(X) 縺ｾ縺滂ｿｽE List 繧呈ｶ郁ｲｻ
    if let Some(inner) = s.strip_prefix("List(") {
        if let Some(inner) = inner.strip_suffix(')') {
            return Type::List(Box::new(type_from_str(inner, defs)));
        }
    }
    match s {
        "int" | "i64" => Type::Int,
        "float" | "double" | "f64" => Type::Float,
        "bool" | "i1" => Type::Bool,
        "str" | "i8*" => Type::String,
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
                // 繧ｸ繧ｧ繝阪Μ繝・け蝙区枚蟄怜・ (T) 縺ｯ繝・・ｽ・ｽ Var 繧定ｿ斐＠縲∝ｽｮ縺ｮ縺ｿ縺ｪ縺・
                Type::Var(base.to_string())
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
        // Generic 蝙区枚蟄怜・ Var 縺ｯ Unknown 繧定ｿ斐＠縲∝ｽｮ縺ｮ縺ｿ縺ｪ縺・
        (Type::Var(_), _) | (_, Type::Var(_)) => true,
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
    constraints: &HashMap<String, Vec<String>>,
) -> Option<(String, Type)> {
    // Generic 蝙区枚蟄怜・ Var(T) 縺ｧ繧｢蜷ｦ縺ｮ interface 蜈･繧｢蜷阪″繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・蜷・
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
        }
        return None;
    }
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

// collect_defs 縺ｧ defs 縺ｮ蝙九・縺ｮ蜻ｼ縺ｳ蜃ｺ縺ｧ繧｢蜷阪→繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・縺ｮ蝙区､懈渊縺ｮ resolved_operator 縺ｦ縺ｪ縺ｿ縺ｪ縺・
// 蟷ｻ蜈･繝｡繧ｽ繝・ラ蜷阪・縺ｮ蝙区枚蟄怜・繝峨ｒ繧｢蜷阪″ type_params/constraints 繧剃ｿ晄戟縺ｯ髱槭ず・・
fn resolve_operators_defs(defs: &mut Defs) {
    // 繝｡繧ｽ繝・ラ蜷阪・縺ｮ constraints/params 繧剃ｿ晄戟縺ｪ蝙九・縺ｮ蜻ｼ縺ｳ蜃ｺ縺ｧ繧｢蜷阪→繝ｦ繝ｼ繧ｶ繝ｼ螳夂ｾｩ蝙九・縺ｮ
    // 髱槭ず繝｡繧ｽ繝・ラ蜷阪・縺ｮ蝙区枚蟄怜・縺ｪ type_from_str 縺ｯ defs 縺ｮ蝗ｲ繧・ｽE・ｽ蛻･蜈壹↓縺ｦ縺・ｋ縺ｮ縺ｿ縺ｪ縺・
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
            // iterable 縺ｮ隕∫ｴ蝙九ｒ var 縺ｮ蝙九→縺励※迺ｰ蠅・↓霑ｽ蜉
            if let Ok(it_ty) = infer_type(iterable, env, defs, constraints) {
                let elem = match &it_ty {
                    Type::List(e) => (**e).clone(),
                    _ => Type::Unknown,
                };
                env.insert(var.clone(), elem);
            }
            resolve_operators_expr(iterable, defs, env, constraints);
            resolve_operators_stmts(body, defs, constraints, env);
        }
        Stmt::Return(Some(e)) => resolve_operators_expr(e, defs, env, constraints),
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

// 貍皮ｮ怜ｭ占ｧ｣豎ｺ縺ｮ縺溘ａ縺ｮ霆ｽ驥丞梛謗ｨ隲厄ｼ・nv 縺ｯ let/蠑墓焚縺ｮ蝙九・縺ｿ・峨・
// check_expr 縺ｮ full env 縺ｯ荳崎ｦ√よ綾繧雁､縺悟ｾ励ｉ繧後↑縺・ｴ蜷医・ Unknown 繧定ｿ斐☆縲・
fn infer_type(
    e: &Expr,
    env: &HashMap<String, Type>,
    defs: &Defs,
    constraints: &HashMap<String, Vec<String>>,
) -> Result<Type, String> {
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
                Ok(Type::Option(Box::new(
                    infer_type(&args[0], env, defs, constraints)?,
                )))
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
            let ot = infer_type(object, env, defs, constraints)?;
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
                // List 繝｡繧ｽ繝・・ｽ・ｽ: get/set -> 蜈・荳、 add -> List、 len -> int
                Type::List(elem) => match method.as_str() {
                    "get" | "set" => Ok((*elem).clone()),
                    "add" => Ok(Type::List(elem)),
                    "add" => Ok(Type::List(elem)),
                    "len" => Ok(Type::Int),
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
            // 蟄舌ｒ蜈医↓隗｣豎ｺ・医ロ繧ｹ繝医＠縺・BinOp 繧ょ性繧・・
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

// Generic constraint 縺ｮ蜈ｨ菴薙→: 蟷ｻ蜈･繝｡繧ｽ繝・ラ鄂ｲ蜷阪・縺ｧ縲悟梛縺ｮ蝙九→繝｡繧ｽ繝・ラ蜷阪″蝣ｴ蜷蜈･繧｢蜷阪→
// 繊･繝・・ｽ・ｽ縺ｮ縺溘ａ縺ｮ菴ｿ逕ｨ・峨ｂ險ｱ蜿ｯ・・
// 蟷ｻ蜈･: List(T where T: Compare) 縺ｮ莠∩縺ｮ List(Vec2) 縺ｪ豌･縺ｮ -> Vec2 縺ｧ Compare 繧呈囓鮟吝ｮ溯｡梧枚｡縺ｮ縺溘ａ縺ｮ
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
        // 蝙区枚蟄怜・ Var(T) 縺ｧ繧｢蜷ｦ縺ｮ interface 蜈･繧｢蜷阪″縺ｮ縺ｮ謌ｻ繧雁､縺・
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
                            "Type error: type {:?} does not satisfy constraint '{}: {}'",
                            concrete, tv, iface
                        ));
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
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
                        resolve_operator_interface(defs, &lt, &rt, op, &env.constraints)
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
                        resolve_operator_interface(defs, &lt, &rt, op, &env.constraints)
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

        Expr::Await(inner) => {
            // await 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE縺ｧ繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE
            // (lime 繧｡繧ｽ繝・ラ蜷代″)縲∝ｽ硅 Country.縺ｮ縺ｪ縺ｿ縺ｪ縺・
            // fn 縺･ lime 縺ｯ戻ｨ蜈ｰ繝｡繧ｽ繝・ラ蜷代″縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
            if let Expr::Call { func, .. } = inner.as_ref() {
                match defs.functions.get(func) {
                    Some(fdef) if fdef.is_async => {
                        // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・縺楂ользов된繝｡繧ｽ繝・ラ蜷代″縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE縺ｧ繧｢蜷阪→
                        if let Some(rt) = &fdef.return_type {
                            Ok(type_from_str(rt, defs))
                        } else {
                            Ok(Type::Unit)
                        }
                    }
                    Some(_) => {
                        // 岍ｵ蜈ｰ縺ｴ繝｡繧ｽ繝・ラ蜷代″(fn) 縺ｯ await 縺ｮ縺ｪ縺ｿ縺ｪ縺・
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

// 譁・・ｽ・ｽ讀懈渊縲Ｆxpected_return 縺ｯ逶ｴ霑托ｿｽE髢｢謨ｰ縺ｮ謌ｻ繧雁梛・ｽE・ｽ謖・ｮ壹↑縺暦ｿｽE None・ｽE・ｽE
fn check_stmt(
    stmt: &Stmt,
    env: &mut TypeEnv,
    defs: &Defs,
    expected_return: Option<&Type>,
    is_async: bool,
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
            check_stmts(then_branch, env, defs, expected_return, is_async)?;
            if let Some(els) = else_branch {
                check_stmts(els, env, defs, expected_return, is_async)?;
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
            check_stmts(body, &mut loop_env, defs, expected_return, is_async)?;
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
            check_stmts(body, env, defs, expected_return, is_async)?;
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
                        check_stmts(body, &mut arm_env, defs, expected_return, is_async)?;
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
                        check_stmts(body, &mut arm_env, defs, expected_return, is_async)?;
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
            check_stmts(body, env, defs, expected_return, is_async)?;
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
    is_async: bool,
) -> Result<(), String> {
    for s in stmts {
        check_stmt(s, env, defs, expected_return, is_async)?;
    }
    Ok(())
}

// 髢｢謨ｰ譛ｬ譁・・ｽ・ｽ讀懈渊・ｽE・ｽEarams 繧堤腸蠅・・ｽ・ｽ豕ｨ蜈･・ｽE・ｽE
fn check_function(
    params: &[(String, String)],
    constraints: &[(String, String)],
    return_type: &Option<String>,
    body: &[Stmt],
    defs: &Defs,
    is_async: bool,
) -> Result<(), String> {
    let mut env = TypeEnv::new();
    for (tv, iface) in constraints {
        env.add_constraint(tv.clone(), iface.clone());
    }
    for (pname, ptype) in params {
        env.insert(pname.clone(), type_from_str(ptype, defs));
    }
    let rt = return_type.as_ref().map(|r| type_from_str(r, defs));
    check_stmts(body, &mut env, defs, rt.as_ref(), is_async)
}

// 繝励Ο繧ｰ繝ｩ繝蜈ｨ菴難ｿｽE蝙区､懈渊
fn type_check(stmts: &[Stmt], defs: &Defs) -> Result<(), String> {
    let mut top_env = TypeEnv::new();
    for stmt in stmts {
        match stmt {
            Stmt::Fn {
                name,
                type_params,
                constraints,
                params,
                return_type,
                body,
                is_async,
            } => {
                let _ = type_params;
                check_function(params, constraints, return_type, body, defs, *is_async)
                    .map_err(|e| format!("In function '{}': {}", name, e))?;
            }
            Stmt::Struct {
                name,
                type_params,
                constraints,
                fields,
                methods,
            } => {
                // 繝｡繧ｽ繝・・ｽ・ｽ讀懈渊: 繝輔ぅ繝ｼ繝ｫ繝峨ｒ迺ｰ蠅・・ｽ・ｽ豕ｨ蜈･
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
                        return_type,
                        body,
                        is_async: _,
                    } = m
                    {
                        // 繝輔ぅ繝ｼ繝ｫ繝臥腸蠅・+ 蠑墓焚繧呈ｳｨ蜈･
                        let mut menv = env.clone();
                        for (tv, iface) in mc {
                            menv.add_constraint(tv.clone(), iface.clone());
                        }
                        for (pname, ptype) in params {
                            menv.insert(pname.clone(), type_from_str(ptype, defs));
                        }
                        let rt = return_type.as_ref().map(|r| type_from_str(r, defs));
                        check_stmts(body, &mut menv, defs, rt.as_ref(), false)
                            .map_err(|e| format!("In method '{}.{}': {}", name, mname, e))?;
                    }
                }
            }
            // 繝医ャ繝励Ξ繝吶Ν縺ｮ螳溯｡梧枚・・ain 縺檎┌縺・・繝ｭ繧ｰ繝ｩ繝逕ｨ・峨ｂ讀懈渊縲・
            // 繝医ャ繝励Ξ繝吶Ν縺ｮ let 蜷悟｣ｫ縺悟盾辣ｧ縺怜粋縺医ｋ繧医≧縲∝・譛・env 繧剃ｽｿ縺・・
            Stmt::Let { name, value, .. } => {
                let _ = check_expr(value, &top_env, defs)?;
                let v_ty = infer_type(value, &top_env.vars, defs, &top_env.constraints);
                let mut env = top_env.clone();
                if let Ok(t) = &v_ty {
                    env.insert(name.clone(), t.clone());
                }
                check_stmt(stmt, &mut env, defs, None, false)?;
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
                check_stmt(stmt, &mut env, defs, None, false)?;
            }
            _ => {}
        }
    }
    Ok(())
}

// ===== Memory Analysis (Step 9: Escape Analysis) =====
// 蜈郁ｪｭ繝｡繧ｽ繝・ラ蜷代″險ｱ蜿ｯ縺ｧ繧｢蜷阪→縺ｮ蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・
// - 蝠ｩ蜊ｫ縺ｮ縺ｪ縺ｿ縺ｪ縺・: 螂ｲ縺ｮ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・(Stack)
// - 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・: return 縺ｻ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
//                       (Heap)
// - 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・: closure/callback 蜈ｷ雎｡ / 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
//                       縺ｻ Heap 繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・(Heap)
// - 蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE heap/stack 縺ｯ繧｢蜷阪→縺ｮ險ｱ蜿ｯ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
// - stack 縺ｯ繧｢蜷阪→蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・繧｢繝ｩ繝ｼ・ｽE・ｽE繧定ｧ｣繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・

// Expr 縺ｮ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蛔・・ｽ・ｽ諠・ｱ蛟､蝙具ｿｽE縺ｮ縺ｿ縺ｪ縺・(縺ｮ縺ｿ譛ｬ蜈･縺ｮ縺ｿ縺ｪ縺・)
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
        Expr::Await(inner) => expr_vars(inner, out),
        _ => {}
    }
}

// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蛔・・ｽ・ｽ(escape position)
// Lime 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE蜈･繝｡繧ｽ繝・ラ蜷代″蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・繝｡繧ｽ繝・ラ蜷代″蜈ｷ雎｡:
//   - 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・(return) -> Heap
//   - lime 繧｡繧ｽ繝・ラ蜷代″縺ｮ Future frame 縺ｯ Heap (await 縺ｮ荳｡譁ｹ蜈ｷ雎｡)
//   - 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ await 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・( async_escapes 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・)
// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・繝｡繧ｽ繝・ラ蜷代″(callback/closures) 縺ｯ蜈･繝｡繧ｽ繝・ラ蜷代″蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・
// 蟋ｩ蜉ｨ縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・險ｱ蜿ｯ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・(繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
fn collect_escape_seeds(stmts: &[Stmt], seeds: &mut Vec<String>) {
    for s in stmts {
        match s {
            // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・: return <expr> 縺ｮ蜈ｷ雎｡
            Stmt::Return(Some(e)) => expr_vars(e, seeds),
            // lime 繧｡繧ｽ繝・ラ蜷代″: await foo(x) 縺ｯ x 縺ｮ縺ｪ縺ｿ縺ｪ縺・ Future frame 縺ｮ縺ｪ縺ｿ縺ｪ縺・
            Stmt::Expr(Expr::Await(inner)) => {
                if let Expr::Call { args, .. } = inner.as_ref() {
                    for a in args {
                        expr_vars(a, seeds);
                    }
                }
            }
            // 螳溯｡梧凾縺ｯ繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE遞ｪ縺ｮ縺ｿ縺ｪ縺・
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
            _ => {}
        }
    }
}

// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蛔・・ｽ・ｽ(assignment chain)
//   let x = f(y) 縺ｯ x 縺ｮ蝣ｴ蜷隗｣譫怜ｸ・ y
//   x = f(y) 縺ｯ x 縺ｮ蝣ｴ蜷隗｣譫怜ｸ・ y
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
            _ => {}
        }
    }
}

// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蛔・縺ｮ縺ｿ縺ｪ縺・(DFS)
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

// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蛔・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・
// is_async: true 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
//   (await 縺ｯ險ｱ蜿ｯ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・Heap frame 縺ｯ縺ｪ縺ｿ縺ｪ縺・)
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
        Stmt::Return(Some(e)) => expr_has_await(e),
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
        Stmt::Return(Some(e)) => expr_vars(e, out),
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

// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蝣ｴ蜷隗｣譫怜ｸ・縺ｫ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・
// report: 縺ｮ縺ｿ縺ｪ縺・("fn:name" -> (var, place)) 縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｧ繧｢蜷阪→
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
    // async: await 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・
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
            // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
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

// ===== Phase 6: Generic Monomorphization =====

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
        Type::Unit => "void".to_string(),
        Type::Unknown => "unknown".to_string(),
        Type::Array(inner) => format!("Array({})", type_to_string(inner)),
    }
}

fn mangled_name(base: &str, type_args: &[String]) -> String {
    format!("{}.{}", base, type_args.join("."))
}

/// Parse "func(Type1, Type2)" into ("func", ["Type1", "Type2"])
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

/// Infer concrete type arguments from call arguments for a generic function.
fn infer_generic_args(
    func_name: &str,
    call_args: &[Expr],
    env: &HashMap<String, Type>,
    defs: &Defs,
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
                    if existing != &arg_str {
                        return Err(format!(
                            "Type mismatch for type parameter '{}': inferred '{}' and '{}'",
                            tv, existing, arg_str
                        ));
                    }
                }
                type_map.insert(tv.clone(), arg_str);
            }
            _ => {
                // For non-Var param types like List(T), infer from inner types
                collect_var_bindings(&arg_type, &ptype, &mut type_map)?;
            }
        }
    }

    let mut result = Vec::new();
    for tp in &fdef.type_params {
        match type_map.get(tp) {
            Some(s) => result.push(s.clone()),
            None => {
                // Check if this type param is used in constraints only (phantom param)
                // Try to infer from constraints or use the default
                return Err(format!(
                    "Cannot infer type parameter '{}' from call arguments in '{}'",
                    tp, func_name
                ));
            }
        }
    }
    Ok(result)
}

/// Collect type variable bindings by matching concrete types against pattern types.
fn collect_var_bindings(
    concrete: &Type,
    pattern: &Type,
    type_map: &mut HashMap<String, String>,
) -> Result<(), String> {
    match (pattern, concrete) {
        (Type::Var(tv), _) => {
            let s = type_to_string(concrete);
            if let Some(existing) = type_map.get(tv) {
                if existing != &s {
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
            collect_var_bindings(c_inner, p_inner, type_map)
        }
        (Type::Option(p_inner), Type::Option(c_inner)) => {
            collect_var_bindings(c_inner, p_inner, type_map)
        }
        (Type::Array(p_inner), Type::Array(c_inner)) => {
            collect_var_bindings(c_inner, p_inner, type_map)
        }
        // Exact match - no bindings
        _ => Ok(()),
    }
}

/// Check that concrete type args satisfy the generic function's constraints.
/// Uses the same constraint system as the type checker.
fn check_generic_constraints(
    fdef: &FunctionDef,
    type_args: &[String],
    defs: &Defs,
) -> Result<(), String> {
    for (tv, iface) in &fdef.constraints {
        for (i, tp) in fdef.type_params.iter().enumerate() {
            if tp == tv && i < type_args.len() {
                let concrete_type = type_from_str(&type_args[i], defs);
                // Use the same logic as check_constraint for struct types
                let ok = match &concrete_type {
                    Type::Struct(sname) => struct_satisfies_interface(defs, sname, iface),
                    Type::Interface(iname, _) => iname == iface,
                    Type::Unknown => true,
                    // Primitives (Int, Float, Bool, String) have built-in operators
                    // and satisfy the corresponding operator interfaces
                    Type::Int | Type::Float | Type::Bool | Type::String => true,
                    _ => false,
                };
                if !ok {
                    return Err(format!(
                        "Type error: {:?} does not satisfy constraint '{}: {}'",
                        concrete_type, tv, iface
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Walk expression, collect generic calls and create monomorphized functions.
fn collect_mono_from_expr(
    e: &Expr,
    env: &mut HashMap<String, Type>,
    defs: &Defs,
    mono_fdefs: &mut HashMap<String, FunctionDef>,
    call_updates: &mut HashMap<String, String>,
    worklist: &mut Vec<String>,
) -> Result<(), String> {
    match e {
        Expr::Call { func, args } => {
            let base_name;
            let explicit_type_args: Option<Vec<String>>;

            // Check if func has explicit type args like "max(i64)"
            if let Some((base, type_strs)) = parse_generic_call_name(func) {
                base_name = base.to_string();
                explicit_type_args = Some(type_strs.iter().map(|s| s.to_string()).collect());
            } else {
                base_name = func.clone();
                explicit_type_args = None;
            }

            // Check if the base function is a generic template
            if let Some(fdef) = defs.functions.get(&base_name) {
                if !fdef.type_params.is_empty() {
                    // Infer or use explicit type args
                    let type_args: Vec<String> = if let Some(ref explicit) = explicit_type_args {
                        explicit.clone()
                    } else {
                        infer_generic_args(&base_name, args, env, defs)?
                    };

                    // Check constraints
                    check_generic_constraints(fdef, &type_args, defs)?;

                    // Create mangled name
                    let mangled = mangled_name(&base_name, &type_args);

                    // Only create monomorphized function if not already created
                    if !mono_fdefs.contains_key(&mangled) {
                        let type_param_strs: Vec<&str> = type_args.iter().map(|s| s.as_str()).collect();
                        let mono = monomorphize_function(fdef, &fdef.type_params, &type_param_strs);
                        mono_fdefs.insert(mangled.clone(), mono);
                        worklist.push(mangled.clone());
                    }

                    // Record the call update (only if name changed)
                    if func != &mangled {
                        call_updates.insert(func.clone(), mangled);
                    }
                }
            }

            // Walk sub-expressions (with env tracking for let bindings etc.)
            // Note: for Call, the args don't introduce new bindings in the caller's env
            for a in args {
                // But we still need to walk the arg expressions for nested generic calls
                // Use a fresh env (or the current one) since args are evaluated in the current scope
                let mut arg_env = env.clone();
                collect_mono_from_expr(a, &mut arg_env, defs, mono_fdefs, call_updates, worklist)?;
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_mono_from_expr(left, env, defs, mono_fdefs, call_updates, worklist)?;
            collect_mono_from_expr(right, env, defs, mono_fdefs, call_updates, worklist)?;
        }
        Expr::UnOp { operand, .. } => {
            collect_mono_from_expr(operand, env, defs, mono_fdefs, call_updates, worklist)?;
        }
        Expr::MethodCall { object, args, .. } => {
            collect_mono_from_expr(object, env, defs, mono_fdefs, call_updates, worklist)?;
            for a in args {
                collect_mono_from_expr(a, env, defs, mono_fdefs, call_updates, worklist)?;
            }
        }
        Expr::FieldAccess { object, .. } => {
            collect_mono_from_expr(object, env, defs, mono_fdefs, call_updates, worklist)?;
        }
        Expr::Array(items) => {
            for it in items {
                collect_mono_from_expr(it, env, defs, mono_fdefs, call_updates, worklist)?;
            }
        }
        Expr::Range { start, end } => {
            collect_mono_from_expr(start, env, defs, mono_fdefs, call_updates, worklist)?;
            collect_mono_from_expr(end, env, defs, mono_fdefs, call_updates, worklist)?;
        }
        Expr::Await(inner) => {
            collect_mono_from_expr(inner, env, defs, mono_fdefs, call_updates, worklist)?;
        }
        _ => {}
    }
    Ok(())
}

/// Walk statements to find generic calls, maintaining env for type inference.
fn collect_mono_from_stmts(
    stmts: &[Stmt],
    env: &mut HashMap<String, Type>,
    defs: &Defs,
    mono_fdefs: &mut HashMap<String, FunctionDef>,
    call_updates: &mut HashMap<String, String>,
    worklist: &mut Vec<String>,
) -> Result<(), String> {
    for s in stmts {
        match s {
            Stmt::Let { name, value, .. } => {
                // Infer type and update env before walking the value
                if let Ok(t) = infer_type(value, env, defs, &HashMap::new()) {
                    env.insert(name.clone(), t);
                }
                collect_mono_from_expr(value, env, defs, mono_fdefs, call_updates, worklist)?;
            }
            Stmt::Return(e) => {
                if let Some(e) = e {
                    collect_mono_from_expr(e, env, defs, mono_fdefs, call_updates, worklist)?;
                }
            }
            Stmt::Expr(e) => {
                collect_mono_from_expr(e, env, defs, mono_fdefs, call_updates, worklist)?;
            }
            Stmt::Assign { name, value } => {
                if let Ok(t) = infer_type(value, env, defs, &HashMap::new()) {
                    env.insert(name.clone(), t);
                }
                collect_mono_from_expr(value, env, defs, mono_fdefs, call_updates, worklist)?;
            }
            Stmt::If { cond, then_branch, else_branch } => {
                collect_mono_from_expr(cond, env, defs, mono_fdefs, call_updates, worklist)?;
                let mut then_env = env.clone();
                collect_mono_from_stmts(then_branch, &mut then_env, defs, mono_fdefs, call_updates, worklist)?;
                if let Some(eb) = else_branch {
                    let mut else_env = env.clone();
                    collect_mono_from_stmts(eb, &mut else_env, defs, mono_fdefs, call_updates, worklist)?;
                }
            }
            Stmt::While { cond, body } => {
                collect_mono_from_expr(cond, env, defs, mono_fdefs, call_updates, worklist)?;
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
                collect_mono_from_expr(iterable, env, defs, mono_fdefs, call_updates, worklist)?;
                collect_mono_from_stmts(body, env, defs, mono_fdefs, call_updates, worklist)?;
            }
            Stmt::Match { expr, arms } => {
                collect_mono_from_expr(expr, env, defs, mono_fdefs, call_updates, worklist)?;
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

/// Update Expr::Call.func in an expression tree using the call_updates mapping.
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

/// Update Expr::Call.func in statements using the call_updates mapping.
fn update_call_in_stmts(stmts: &mut [Stmt], call_updates: &HashMap<String, String>) {
    for s in stmts.iter_mut() {
        match s {
            Stmt::Let { value, .. } => update_call_in_expr(value, call_updates),
            Stmt::Return(e) => {
                if let Some(e) = e {
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

/// Main monomorphization pass: discovers generic calls, creates concrete instances,
/// updates call names, and adds monomorphized functions to defs.
/// Runs after type checking, before memory analysis and codegen.
fn monomorphize_all(defs: &mut Defs, stmts: &mut [Stmt]) -> Result<(), String> {
    let mut mono_fdefs: HashMap<String, FunctionDef> = HashMap::new();
    let mut call_updates: HashMap<String, String> = HashMap::new();

    let mut worklist: Vec<String> = defs.functions.keys().cloned().collect();
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Also scan top-level statements for generic calls
    let mut env: HashMap<String, Type> = HashMap::new();
    collect_mono_from_stmts(
        stmts,
        &mut env,
        defs,
        &mut mono_fdefs,
        &mut call_updates,
        &mut worklist,
    )?;

    // Iterative worklist: process functions and their newly created monomorphized clones
    while let Some(func_name) = worklist.pop() {
        if processed.contains(&func_name) {
            continue;
        }
        processed.insert(func_name.clone());

        let fdef = match defs.functions.get(&func_name) {
            Some(f) => f.clone(),
            None => continue,
        };

        // Build env from function parameters for type inference
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

    // Add monomorphized functions to defs
    for (mangled, fdef) in &mono_fdefs {
        if !defs.functions.contains_key(mangled) {
            defs.functions.insert(mangled.clone(), fdef.clone());
        }
    }

    // Update Expr::Call.func in all function bodies to use mangled names
    for (_name, fdef) in defs.functions.iter_mut() {
        update_call_in_stmts(&mut fdef.body, &call_updates);
    }
    // Also update top-level statements (after scanning so env is fresh)
    update_call_in_stmts(stmts, &call_updates);

    Ok(())
}

// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・(let name -> Stack/Heap) 縺ｦ縺ｪ縺ｿ縺ｪ縺・縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
// (LLVM Backend 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・蛻・蜈･縺ｮ縺ｪ縺ｿ縺ｪ縺・)
fn memory_analyze(stmts: &[Stmt], defs: &Defs) -> Result<HashMap<String, MemoryPlace>, String> {
    let mut report: Vec<(String, MemoryPlace)> = Vec::new();
    // 蝗ｲ繧險ｱ蜿ｯ: 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・蜻蜷代″螂ｲ縺ｮ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・(非 async)
    //   Stmt::Fn 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・analyze_block 縺ｯ繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE
    //   (Fn 縺ｮ body 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・)
    analyze_block(stmts, false, defs, &mut report)?;

    println!("=== Memory ===");
    let mut map: HashMap<String, MemoryPlace> = HashMap::new();
    for (name, place) in &report {
        let p = match place {
            MemoryPlace::Stack => "stack",
            MemoryPlace::Heap => "heap",
        };
        println!("  {} -> {}", name, p);
        map.insert(name.clone(), *place);
    }
    println!();
    Ok(map)
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
        Expr::Await(inner) => {
            let fut = eval_expr(inner, env, defs)?;
            match fut {
                Value::Future { func, args } => {
                    // force_run: true 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・
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

// force_run: true 縺ｯ縺ｧ繧｢蜷阪→縺ｮ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・繧｢繝ｩ繝ｼ・ｽE・ｽE繧定ｧ｣繝ｼ繝牙ｒ繝・・ｽ・ｽ辷ｰ縺ｮ縺ｿ縺ｪ縺・
// (await 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・)
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

    // lime 繧｡繧ｽ繝・ラ蜷代″縺ｮ蟷ｻ蜈･: 蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE(Future) 縺ｦ縺ｪ縺ｿ縺ｪ縺・、
    // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・await 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
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

    // 譁ｰ縺励＞繝ｭ繝ｼ繧ｫ繝ｫ迺ｰ蠅・ 繝輔ぅ繝ｼ繝ｫ繝峨ｒ證鈴ｻ呎ｳｨ蜈･・ｽE・ｽEelf/this 縺ｪ縺暦ｼ・
    let mut local: HashMap<String, Value> = HashMap::new();
    for (fname, fval) in fields {
        local.insert(fname.clone(), fval.clone());
    }

    // 蠑墓焚譚溽ｸ幢ｼ医ヵ繧｣繝ｼ繝ｫ繝峨→蜷悟錐縺ｪ繧牙ｼ墓焚縺悟━蜈茨ｼ・
    for ((param_name, _param_type), val) in func.params.iter().zip(args.into_iter()) {
        local.insert(param_name.clone(), val);
    }

    Ok(exec_value(execute_stmts(&func.body, &mut local, defs)?))
}

#[derive(Debug)]
enum ExecResult {
    // 繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE
    Continue,
    // 繝医≧繝ｩ繝ｼ繧ｶ繝ｼ螳夂ｾｩ縺ｯ繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨◎縺ｮ蛟､繧呈囓鮟・
    // (return 繧ｫ繝ｼ繝ｫ繧｢蜷阪″縺ｮ縺ｪ縺ｿ縺ｪ縺・匝ｍ豌ｴ縺ｯ縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧｢蜷阪→)
    Value(Value),
    // 繝｡繧ｽ繝・ラ蜷代″縺ｮ return 繧ｫ繝ｼ繝ｫ繧｢蜷阪″縺ｮ縺ｪ縺ｿ縺ｪ縺・(繝医≧繝ｩ繝ｼ縺ｧ繧｢蜷阪→縺ｮ髢｢謨ｰ/繝｡繧ｽ繝・ラ縺ｮ縺ｪ縺ｿ縺ｪ縺・)
    Return(Value),
}

// 繝｡繧ｽ繝・ラ/繝｡繧ｽ繝・ラ蜷代″縺ｮ蛟､繧呈囓鮟・: return 繧ｫ繝ｼ繝ｫ繧｢蜷阪″縺ｮ縺ｪ縺ｿ縺ｪ縺・ Value 繧定ｿ斐＠、
// Continue 縺ｯ繧｢蜷阪→縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE縺ｮ繝医≧繝ｩ繝ｼ縺ｧ繧｢蜷阪→縺ｯ縺ｪ縺ｿ縺ｪ縺・
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
    for (idx, stmt) in stmts.iter().enumerate() {
        let r = execute_stmt(stmt, env, defs)?;
        match r {
            // return 繧ｫ繝ｼ繝ｫ繧｢蜷阪″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｧ繧｢蜷阪→縺ｯ髢｢謨ｰ/繝｡繧ｽ繝・ラ縺ｮ縺ｪ縺ｿ縺ｪ縺・
            ExecResult::Return(v) => return Ok(ExecResult::Return(v)),
            // 繝医≧繝ｩ繝ｼ繧ｶ繝ｼ螳夂ｾｩ縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧峨お繝ｩ繝ｼ・ｽE・ｽE: 繝ｭ繝ｼ繧ｫ繝ｫ繧｢蜷阪″ Expr 縺ｧ蟇ｾ蜉ｻ縺ｿ縺ｪ縺・
            // 匝ｍ豌ｴ縺ｯ縺ｮ蝙区枚蜿ｷ蜃ｺ縺ｧ繧｢蜷阪→縺ｮ蛟､繧呈囓鮟・(繝｡繧ｽ繝・ラ/return 縺ｯ縺ｪ縺ｿ縺ｪ縺・繝医≧繝ｩ繝ｼ縺ｧ繧｢蜷阪→)
            other => {
                if idx == len - 1 {
                    last = other;
                }
            }
        }
    }
    Ok(last)
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
            let v = eval_expr(e, env, defs)?;
            Ok(ExecResult::Value(v))
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
