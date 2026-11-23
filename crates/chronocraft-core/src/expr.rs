//! Flow-rate expression evaluator.
//!
//! Grammar (whitespace-insensitive):
//! ```text
//! expr    := term (("+" | "-") term)*
//! term    := factor (("*" | "/") factor)*
//! factor  := "-" factor | primary
//! primary := number | variable | call | "(" expr ")"
//! call    := "min" "(" expr "," expr ")"
//!          | "max" "(" expr "," expr ")"
//!          | "clamp" "(" expr "," expr "," expr ")"
//!          | "step" "(" expr "," expr ")"
//!          | "pulse" "(" expr "," expr "," expr "," expr ")"
//! ```
//! Variables are stock ids, parameter names, or `t` (model time).
//! `step(x, edge)` is 1 when `x >= edge`, else 0.
//! `pulse(x, start, width, height)` is `height` while
//! `start <= x < start + width`, else 0.

use crate::error::ChronoError;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<Token>, ChronoError> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            let mut dots = 0;
            while i < bytes.len() {
                let d = bytes[i] as char;
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' {
                    dots += 1;
                    i += 1;
                } else {
                    break;
                }
            }
            let text = src.get(start..i).unwrap_or("");
            if dots > 1 || text == "." {
                return Err(ChronoError::Validation(format!(
                    "Bad number literal '{text}' in rate expression"
                )));
            }
            match text.parse::<f64>() {
                Ok(v) => out.push(Token::Num(v)),
                Err(_) => {
                    return Err(ChronoError::Validation(format!(
                        "Bad number literal '{text}' in rate expression"
                    )));
                }
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len() {
                let d = bytes[i] as char;
                if d.is_ascii_alphanumeric() || d == '_' {
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(Token::Ident(src.get(start..i).unwrap_or("").to_string()));
        } else {
            let tok = match c {
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '(' => Token::LParen,
                ')' => Token::RParen,
                ',' => Token::Comma,
                _ => {
                    return Err(ChronoError::Validation(format!(
                        "Unexpected character '{c}' in rate expression"
                    )));
                }
            };
            out.push(tok);
            i += 1;
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_expr(&mut self, vars: &HashMap<String, f64>) -> Result<f64, ChronoError> {
        let mut acc = self.parse_term(vars)?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.next();
                    acc += self.parse_term(vars)?;
                }
                Some(Token::Minus) => {
                    self.next();
                    acc -= self.parse_term(vars)?;
                }
                _ => return Ok(acc),
            }
        }
    }

    fn parse_term(&mut self, vars: &HashMap<String, f64>) -> Result<f64, ChronoError> {
        let mut acc = self.parse_factor(vars)?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.next();
                    acc *= self.parse_factor(vars)?;
                }
                Some(Token::Slash) => {
                    self.next();
                    let denom = self.parse_factor(vars)?;
                    if denom.abs() < 1e-12 {
                        return Err(ChronoError::Validation(
                            "Division by zero in rate expression".to_string(),
                        ));
                    }
                    acc /= denom;
                }
                _ => return Ok(acc),
            }
        }
    }

    fn parse_factor(&mut self, vars: &HashMap<String, f64>) -> Result<f64, ChronoError> {
        match self.peek() {
            Some(Token::Minus) => {
                self.next();
                Ok(-self.parse_factor(vars)?)
            }
            _ => self.parse_primary(vars),
        }
    }

    fn parse_primary(&mut self, vars: &HashMap<String, f64>) -> Result<f64, ChronoError> {
        match self.next() {
            Some(Token::Num(v)) => Ok(v),
            Some(Token::Ident(name)) => {
                if self.peek() == Some(&Token::LParen) {
                    self.next();
                    self.parse_call(&name, vars)
                } else {
                    vars.get(&name).copied().ok_or_else(|| {
                        ChronoError::Validation(format!(
                            "Unknown variable '{name}' in rate expression"
                        ))
                    })
                }
            }
            Some(Token::LParen) => {
                let v = self.parse_expr(vars)?;
                match self.next() {
                    Some(Token::RParen) => Ok(v),
                    _ => Err(ChronoError::Validation(
                        "Missing closing parenthesis in rate expression".to_string(),
                    )),
                }
            }
            _ => Err(ChronoError::Validation(
                "Unexpected end of rate expression".to_string(),
            )),
        }
    }

    fn expect_comma(&mut self) -> Result<(), ChronoError> {
        match self.next() {
            Some(Token::Comma) => Ok(()),
            _ => Err(ChronoError::Validation(
                "Expected comma between function arguments".to_string(),
            )),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), ChronoError> {
        match self.next() {
            Some(Token::RParen) => Ok(()),
            _ => Err(ChronoError::Validation(
                "Expected closing parenthesis after function arguments".to_string(),
            )),
        }
    }

    fn parse_call(&mut self, name: &str, vars: &HashMap<String, f64>) -> Result<f64, ChronoError> {
        match name {
            "min" => {
                let a = self.parse_expr(vars)?;
                self.expect_comma()?;
                let b = self.parse_expr(vars)?;
                self.expect_rparen()?;
                Ok(a.min(b))
            }
            "max" => {
                let a = self.parse_expr(vars)?;
                self.expect_comma()?;
                let b = self.parse_expr(vars)?;
                self.expect_rparen()?;
                Ok(a.max(b))
            }
            "clamp" => {
                let x = self.parse_expr(vars)?;
                self.expect_comma()?;
                let lo = self.parse_expr(vars)?;
                self.expect_comma()?;
                let hi = self.parse_expr(vars)?;
                self.expect_rparen()?;
                if lo > hi {
                    return Err(ChronoError::Validation(
                        "clamp needs lo <= hi in rate expression".to_string(),
                    ));
                }
                Ok(x.clamp(lo, hi))
            }
            "step" => {
                let x = self.parse_expr(vars)?;
                self.expect_comma()?;
                let edge = self.parse_expr(vars)?;
                self.expect_rparen()?;
                Ok(if x >= edge { 1.0 } else { 0.0 })
            }
            "pulse" => {
                let x = self.parse_expr(vars)?;
                self.expect_comma()?;
                let start = self.parse_expr(vars)?;
                self.expect_comma()?;
                let width = self.parse_expr(vars)?;
                self.expect_comma()?;
                let height = self.parse_expr(vars)?;
                self.expect_rparen()?;
                Ok(if x >= start && x < start + width {
                    height
                } else {
                    0.0
                })
            }
            _ => Err(ChronoError::Validation(format!(
                "Unknown function '{name}' in rate expression (expected min, max, clamp, step, or pulse)"
            ))),
        }
    }
}

/// Evaluate a rate expression against stock values, parameters, and time `t`.
pub fn eval_expr(expr: &str, vars: &HashMap<String, f64>) -> Result<f64, ChronoError> {
    if expr.trim().is_empty() {
        return Err(ChronoError::Validation(
            "Rate expression cannot be empty".to_string(),
        ));
    }
    let tokens = tokenize(expr)?;
    if tokens.is_empty() {
        return Err(ChronoError::Validation(
            "Rate expression cannot be empty".to_string(),
        ));
    }
    let mut parser = Parser { tokens, pos: 0 };
    let value = parser.parse_expr(vars)?;
    if parser.pos != parser.tokens.len() {
        return Err(ChronoError::Validation(
            "Trailing characters after rate expression".to_string(),
        ));
    }
    if !value.is_finite() {
        return Err(ChronoError::Validation(
            "Rate expression evaluated to a non-finite value".to_string(),
        ));
    }
    Ok(value)
}

/// Check that an expression parses; used when adding flows.
pub fn check_expr(expr: &str) -> Result<(), ChronoError> {
    let tokens = tokenize(expr)?;
    if tokens.is_empty() {
        return Err(ChronoError::Validation(
            "Rate expression cannot be empty".to_string(),
        ));
    }
    let mut probe: HashMap<String, f64> = HashMap::new();
    for tok in &tokens {
        if let Token::Ident(name) = tok {
            probe.entry(name.clone()).or_insert(0.0);
        }
    }
    let mut parser = Parser { tokens, pos: 0 };
    let _ = parser.parse_expr(&probe)?;
    if parser.pos != parser.tokens.len() {
        return Err(ChronoError::Validation(
            "Trailing characters after rate expression".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn arithmetic_and_variables_evaluate() {
        let v = vars(&[("pop", 100.0), ("growth", 0.1), ("t", 3.0)]);
        let got = eval_expr("growth * pop * (1 - pop / 1000) + t", &v);
        assert!(got.is_ok());
        if let Ok(val) = got {
            assert!((val - 12.0).abs() < 1e-9);
        }
    }

    #[test]
    fn helpers_min_max_clamp_step_pulse() {
        let v = vars(&[("t", 5.0)]);
        for (expr, expected) in [
            ("min(3, 7)", 3.0),
            ("max(3, 7)", 7.0),
            ("clamp(9, 0, 5)", 5.0),
            ("step(t, 5)", 1.0),
            ("step(t, 6)", 0.0),
            ("pulse(t, 4, 2, 10)", 10.0),
            ("pulse(t, 8, 2, 10)", 0.0),
            ("-3 + 2 * 4", 5.0),
        ] {
            let got = eval_expr(expr, &v);
            assert!(got.is_ok());
            if let Ok(val) = got {
                assert!((val - expected).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn bad_expressions_return_errors() {
        let v = vars(&[("a", 1.0)]);
        assert!(eval_expr("", &v).is_err());
        assert!(eval_expr("a + ", &v).is_err());
        assert!(eval_expr("nope + 1", &v).is_err());
        assert!(eval_expr("1 / 0", &v).is_err());
        assert!(eval_expr("bogus(1)", &v).is_err());
        assert!(check_expr("").is_err());
        assert!(check_expr("growth * pop + min(a, 2)").is_ok());
    }
}
