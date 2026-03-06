// Expression parser for dynamic layout values.
//
// Handles screenWidth/screenHeight variables, basic arithmetic, and
// a few math functions (min, max, floor, ceil, round, abs, roundEven).
// Used by modes/stretch config for stuff like "roundEven(sw * 0.95)".

// --- Tokenizer ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Number,
    Identifier,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
    End,
    Invalid,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    text: String,
    val: f64,
}

impl Token {
    fn new(kind: TokenKind, text: impl Into<String>, val: f64) -> Self {
        Self { kind, text: text.into(), val }
    }

    fn end() -> Self {
        Self::new(TokenKind::End, "", 0.0)
    }
}

struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    fn new(input: &str) -> Self {
        Self { chars: input.chars().collect(), pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_ws();
        if self.pos >= self.chars.len() {
            return Token::end();
        }

        let ch = self.chars[self.pos];

        // single-char operators
        let op = match ch {
            '+' => Some(TokenKind::Plus),
            '-' => Some(TokenKind::Minus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            ',' => Some(TokenKind::Comma),
            _ => None,
        };
        if let Some(kind) = op {
            self.pos += 1;
            return Token::new(kind, ch.to_string(), 0.0);
        }

        // numbers (with optional decimal point)
        if ch.is_ascii_digit() || ch == '.' {
            let start = self.pos;
            let mut has_dot = false;
            while self.pos < self.chars.len()
                && (self.chars[self.pos].is_ascii_digit() || self.chars[self.pos] == '.')
            {
                if self.chars[self.pos] == '.' {
                    if has_dot { break; } // second dot -> stop
                    has_dot = true;
                }
                self.pos += 1;
            }
            let text: String = self.chars[start..self.pos].iter().collect();
            let num: f64 = text.parse().unwrap_or(0.0);
            return Token::new(TokenKind::Number, text, num);
        }

        // identifiers
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = self.pos;
            while self.pos < self.chars.len()
                && (self.chars[self.pos].is_ascii_alphanumeric() || self.chars[self.pos] == '_')
            {
                self.pos += 1;
            }
            let text: String = self.chars[start..self.pos].iter().collect();
            return Token::new(TokenKind::Identifier, text, 0.0);
        }

        // garbage
        self.pos += 1;
        Token::new(TokenKind::Invalid, ch.to_string(), 0.0)
    }
}

// --- Parser ---
//
// Recursive descent with standard precedence:
//   expr   = term (('+' | '-') term)*
//   term   = unary (('*' | '/') unary)*
//   unary  = ('-' | '+')? primary
//   primary = number | ident | fn_call | '(' expr ')'

struct Parser {
    tok: Tokenizer,
    cur: Token,
    sw: i32,
    sh: i32,
}

impl Parser {
    fn new(expr: &str, sw: i32, sh: i32) -> Self {
        let mut tok = Tokenizer::new(expr);
        let cur = tok.next_token();
        Self { tok, cur, sw, sh }
    }

    fn advance(&mut self) {
        self.cur = self.tok.next_token();
    }

    fn expect(&mut self, kind: TokenKind, msg: &str) -> Result<(), String> {
        if self.cur.kind != kind {
            return Err(msg.to_string());
        }
        self.advance();
        Ok(())
    }

    fn parse(&mut self) -> Result<f64, String> {
        let val = self.parse_expr()?;
        if self.cur.kind != TokenKind::End {
            return Err(format!("Unexpected token at end: {}", self.cur.text));
        }
        Ok(val)
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut lhs = self.parse_term()?;
        while self.cur.kind == TokenKind::Plus || self.cur.kind == TokenKind::Minus {
            let op = self.cur.kind;
            self.advance();
            let rhs = self.parse_term()?;
            lhs = if op == TokenKind::Plus { lhs + rhs } else { lhs - rhs };
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut lhs = self.parse_unary()?;
        while self.cur.kind == TokenKind::Star || self.cur.kind == TokenKind::Slash {
            let op = self.cur.kind;
            self.advance();
            let rhs = self.parse_unary()?;
            if op == TokenKind::Star {
                lhs *= rhs;
            } else {
                if rhs == 0.0 {
                    return Err("Division by zero".into());
                }
                lhs /= rhs;
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if self.cur.kind == TokenKind::Minus {
            self.advance();
            return Ok(-self.parse_unary()?);
        }
        if self.cur.kind == TokenKind::Plus {
            self.advance();
            return self.parse_unary();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.cur.kind {
            TokenKind::Number => {
                let v = self.cur.val;
                self.advance();
                Ok(v)
            }
            TokenKind::Identifier => {
                let name = self.cur.text.clone();
                self.advance();

                // might be a function call
                if self.cur.kind == TokenKind::LParen {
                    return self.parse_fn_call(&name);
                }
                self.resolve_var(&name)
            }
            TokenKind::LParen => {
                self.advance();
                let v = self.parse_expr()?;
                self.expect(TokenKind::RParen, "Expected ')'")?;
                Ok(v)
            }
            _ => Err(format!("Unexpected token: {}", self.cur.text)),
        }
    }

    fn parse_fn_call(&mut self, name: &str) -> Result<f64, String> {
        self.expect(TokenKind::LParen, "Expected '(' after function name")?;

        let mut args = Vec::new();
        if self.cur.kind != TokenKind::RParen {
            args.push(self.parse_expr()?);
            while self.cur.kind == TokenKind::Comma {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }
        self.expect(TokenKind::RParen, "Expected ')' after function arguments")?;

        self.eval_fn(name, &args)
    }

    fn resolve_var(&self, name: &str) -> Result<f64, String> {
        match name {
            "screenWidth" | "sw" => Ok(self.sw as f64),
            "screenHeight" | "sh" => Ok(self.sh as f64),
            _ => Err(format!("Unknown variable: {name}")),
        }
    }

    fn eval_fn(&self, name: &str, args: &[f64]) -> Result<f64, String> {
        match name {
            "min" => {
                if args.len() != 2 { return Err("min() requires 2 arguments".into()); }
                Ok(args[0].min(args[1]))
            }
            "max" => {
                if args.len() != 2 { return Err("max() requires 2 arguments".into()); }
                Ok(args[0].max(args[1]))
            }
            "floor" => {
                if args.len() != 1 { return Err("floor() requires 1 argument".into()); }
                Ok(args[0].floor())
            }
            "ceil" => {
                if args.len() != 1 { return Err("ceil() requires 1 argument".into()); }
                Ok(args[0].ceil())
            }
            "round" => {
                if args.len() != 1 { return Err("round() requires 1 argument".into()); }
                Ok(args[0].round())
            }
            "abs" => {
                if args.len() != 1 { return Err("abs() requires 1 argument".into()); }
                Ok(args[0].abs())
            }
            "roundEven" => {
                // ceil(x/2) * 2 -> nearest even integer (always rounds up)
                if args.len() != 1 { return Err("roundEven() requires 1 argument".into()); }
                Ok((args[0] / 2.0).ceil() * 2.0)
            }
            _ => Err(format!("Unknown function: {name}")),
        }
    }
}

// --- Public API ---

// Evaluate an expression string, flooring the result to i32.
pub fn evaluate_expression(expr: &str, screen_width: i32, screen_height: i32) -> Result<i32, String> {
    let s = expr.trim();
    if s.is_empty() {
        return Err("Expression cannot be empty".into());
    }
    let mut p = Parser::new(s, screen_width, screen_height);
    let val = p.parse()?;
    Ok(val.floor() as i32)
}

// Returns true when the string contains something beyond a plain integer
// (i.e. it actually needs the expression parser, not just i32::parse).
pub fn is_expression(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() { return false; }

    let start = if s.starts_with('-') { 1 } else { 0 };
    if start >= s.len() {
        return true; // bare minus sign, sure why not
    }

    s[start..].chars().any(|c| !c.is_ascii_digit())
}

// Quick check that an expression parses (uses dummy 1920x1080 resolution).
pub fn validate_expression(expr: &str) -> Result<(), String> {
    let s = expr.trim();
    if s.is_empty() {
        return Err("Expression cannot be empty".into());
    }
    let mut p = Parser::new(s, 1920, 1080);
    p.parse().map(|_| ())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_number() {
        assert_eq!(evaluate_expression("42", 1920, 1080), Ok(42));
    }

    #[test]
    fn simple_addition() {
        assert_eq!(evaluate_expression("10 + 20", 1920, 1080), Ok(30));
    }

    #[test]
    fn subtraction() {
        assert_eq!(evaluate_expression("100 - 30", 1920, 1080), Ok(70));
    }

    #[test]
    fn multiplication() {
        assert_eq!(evaluate_expression("6 * 7", 1920, 1080), Ok(42));
    }

    #[test]
    fn division() {
        assert_eq!(evaluate_expression("84 / 2", 1920, 1080), Ok(42));
    }

    #[test]
    fn division_by_zero() {
        assert!(evaluate_expression("1 / 0", 1920, 1080).is_err());
    }

    #[test]
    fn operator_precedence() {
        // 2 + 3 * 4 = 14, not 20
        assert_eq!(evaluate_expression("2 + 3 * 4", 1920, 1080), Ok(14));
    }

    #[test]
    fn parentheses() {
        assert_eq!(evaluate_expression("(2 + 3) * 4", 1920, 1080), Ok(20));
    }

    #[test]
    fn unary_minus() {
        assert_eq!(evaluate_expression("-5", 1920, 1080), Ok(-5));
        assert_eq!(evaluate_expression("10 + -3", 1920, 1080), Ok(7));
    }

    #[test]
    fn unary_plus() {
        assert_eq!(evaluate_expression("+5", 1920, 1080), Ok(5));
    }

    #[test]
    fn screen_width_variable() {
        assert_eq!(evaluate_expression("screenWidth", 1920, 1080), Ok(1920));
    }

    #[test]
    fn screen_height_variable() {
        assert_eq!(evaluate_expression("screenHeight", 1920, 1080), Ok(1080));
    }

    #[test]
    fn screen_width_arithmetic() {
        assert_eq!(
            evaluate_expression("screenWidth / 2", 1920, 1080),
            Ok(960)
        );
    }

    #[test]
    fn complex_expression() {
        assert_eq!(
            evaluate_expression("(screenWidth - 100) / 2", 1920, 1080),
            Ok(910)
        );
    }

    #[test]
    fn function_min() {
        assert_eq!(
            evaluate_expression("min(screenWidth, 1000)", 1920, 1080),
            Ok(1000)
        );
    }

    #[test]
    fn function_max() {
        assert_eq!(
            evaluate_expression("max(500, screenHeight)", 1920, 1080),
            Ok(1080)
        );
    }

    #[test]
    fn function_floor() {
        assert_eq!(evaluate_expression("floor(3.7)", 1920, 1080), Ok(3));
    }

    #[test]
    fn function_ceil() {
        assert_eq!(evaluate_expression("ceil(3.2)", 1920, 1080), Ok(4));
    }

    #[test]
    fn function_round() {
        assert_eq!(evaluate_expression("round(3.5)", 1920, 1080), Ok(4));
        assert_eq!(evaluate_expression("round(3.4)", 1920, 1080), Ok(3));
    }

    #[test]
    fn function_abs() {
        assert_eq!(evaluate_expression("abs(-42)", 1920, 1080), Ok(42));
        assert_eq!(evaluate_expression("abs(42)", 1920, 1080), Ok(42));
    }

    #[test]
    fn function_round_even() {
        // ceil(5/2)*2 = ceil(2.5)*2 = 3*2 = 6
        assert_eq!(evaluate_expression("roundEven(5)", 1920, 1080), Ok(6));
        // ceil(4/2)*2 = ceil(2)*2 = 2*2 = 4
        assert_eq!(evaluate_expression("roundEven(4)", 1920, 1080), Ok(4));
    }

    #[test]
    fn nested_function_calls() {
        assert_eq!(
            evaluate_expression("min(max(100, 200), 150)", 1920, 1080),
            Ok(150)
        );
    }

    #[test]
    fn decimal_arithmetic() {
        // 1920 * 0.9 = 1728
        assert_eq!(
            evaluate_expression("screenWidth * 0.9", 1920, 1080),
            Ok(1728)
        );
    }

    #[test]
    fn whitespace_handling() {
        assert_eq!(evaluate_expression("  42  ", 1920, 1080), Ok(42));
        assert_eq!(evaluate_expression("  1 + 2  ", 1920, 1080), Ok(3));
    }

    #[test]
    fn empty_expression_is_error() {
        assert!(evaluate_expression("", 1920, 1080).is_err());
        assert!(evaluate_expression("   ", 1920, 1080).is_err());
    }

    #[test]
    fn unknown_variable_is_error() {
        assert!(evaluate_expression("fooBar", 1920, 1080).is_err());
    }

    #[test]
    fn unknown_function_is_error() {
        assert!(evaluate_expression("sqrt(4)", 1920, 1080).is_err());
    }

    #[test]
    fn wrong_arg_count_is_error() {
        assert!(evaluate_expression("min(1)", 1920, 1080).is_err());
        assert!(evaluate_expression("floor(1, 2)", 1920, 1080).is_err());
    }

    #[test]
    fn is_expression_pure_integers() {
        assert!(!is_expression("42"));
        assert!(!is_expression("-42"));
        assert!(!is_expression("0"));
    }

    #[test]
    fn is_expression_complex() {
        assert!(is_expression("screenWidth"));
        assert!(is_expression("10 + 20"));
        assert!(is_expression("min(1,2)"));
        assert!(is_expression("3.14"));
    }

    #[test]
    fn validate_expression_ok() {
        assert!(validate_expression("screenWidth / 2").is_ok());
        assert!(validate_expression("min(100, 200)").is_ok());
    }

    #[test]
    fn validate_expression_err() {
        assert!(validate_expression("").is_err());
        assert!(validate_expression("unknownVar").is_err());
    }

    #[test]
    fn realistic_mode_expression() {
        // "screenWidth - 300" on a 2560-wide display
        assert_eq!(
            evaluate_expression("screenWidth - 300", 2560, 1440),
            Ok(2260)
        );
    }

    #[test]
    fn realistic_stretch_expression() {
        // centering calc
        assert_eq!(
            evaluate_expression("(screenWidth - 300) / 2", 2560, 1440),
            Ok(1130)
        );
    }
}
