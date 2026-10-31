use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Ref(String),
    Neg(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Value(f64),
    Stale,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Eval {
    Value(f64),
    Stale,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '+' => {
                out.push(Tok::Op('+'));
                i += 1
            }
            '-' | '−' => {
                out.push(Tok::Op('-'));
                i += 1
            }
            '*' | '×' => {
                out.push(Tok::Op('*'));
                i += 1
            }
            '/' | '÷' => {
                out.push(Tok::Op('/'));
                i += 1
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n = s.parse::<f64>().map_err(|_| format!("invalid number `{s}`"))?;
                out.push(Tok::Num(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Ident(chars[start..i].iter().collect::<String>().to_ascii_lowercase()));
            }
            other => return Err(format!("unexpected character `{other}`")),
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let mut lhs = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.term()?;
            let op = if c == '+' { Op::Add } else { Op::Sub };
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        while let Some(Tok::Op(c @ ('*' | '/'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.unary()?;
            let op = if c == '*' { Op::Mul } else { Op::Div };
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Ident(s)) => Ok(Expr::Ref(s)),
            Some(Tok::LParen) => {
                let e = self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err("missing `)`".into()),
                }
            }
            Some(Tok::RParen) => Err("unexpected `)`".into()),
            Some(Tok::Op(c)) => Err(format!("unexpected `{c}`")),
            None => Err("formula ends too early".into()),
        }
    }
}

pub fn parse(src: &str) -> Result<Expr, String> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err("formula is empty".into());
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(match &p.toks[p.pos] {
            Tok::RParen => "unexpected `)`".into(),
            _ => "unexpected text after the formula".into(),
        });
    }
    Ok(e)
}

pub fn refs(e: &Expr) -> Vec<String> {
    fn walk(e: &Expr, out: &mut BTreeSet<String>) {
        match e {
            Expr::Num(_) => {}
            Expr::Ref(s) => {
                out.insert(s.clone());
            }
            Expr::Neg(a) => walk(a, out),
            Expr::Bin(_, a, b) => {
                walk(a, out);
                walk(b, out);
            }
        }
    }
    let mut set = BTreeSet::new();
    walk(e, &mut set);
    set.into_iter().collect()
}

fn compute(e: &Expr, vals: &HashMap<String, f64>) -> Result<f64, String> {
    Ok(match e {
        Expr::Num(n) => *n,
        Expr::Ref(s) => vals[s],
        Expr::Neg(a) => -compute(a, vals)?,
        Expr::Bin(op, a, b) => {
            let (x, y) = (compute(a, vals)?, compute(b, vals)?);
            match op {
                Op::Add => x + y,
                Op::Sub => x - y,
                Op::Mul => x * y,
                Op::Div => {
                    if y == 0.0 {
                        return Err("division by zero".into());
                    }
                    x / y
                }
            }
        }
    })
}

pub fn eval(e: &Expr, lookup: &dyn Fn(&str) -> Option<Input>) -> Eval {
    let mut vals = HashMap::new();
    let mut stale = false;
    for r in refs(e) {
        match lookup(&r) {
            None => return Eval::Error(format!("unknown entry `{r}`")),
            Some(Input::Error) => return Eval::Error(format!("input `{r}` has an error")),
            Some(Input::Stale) => stale = true,
            Some(Input::Value(v)) => {
                vals.insert(r, v);
            }
        }
    }
    if stale {
        return Eval::Stale;
    }
    match compute(e, &vals) {
        Ok(v) if v.is_finite() => Eval::Value(v),
        Ok(_) => Eval::Error("result is not a finite number".into()),
        Err(m) => Eval::Error(m),
    }
}

/// Returns one dependency cycle (as a path of slugs) if the graph has any.
pub fn find_cycle(graph: &HashMap<String, Vec<String>>) -> Option<Vec<String>> {
    fn dfs(
        n: &str,
        graph: &HashMap<String, Vec<String>>,
        color: &mut HashMap<String, u8>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        color.insert(n.to_string(), 1);
        stack.push(n.to_string());
        for m in graph.get(n).into_iter().flatten() {
            match color.get(m.as_str()).copied().unwrap_or(0) {
                1 => {
                    let start = stack.iter().position(|s| s == m).unwrap();
                    let mut cyc = stack[start..].to_vec();
                    cyc.push(m.clone());
                    return Some(cyc);
                }
                0 => {
                    if let Some(c) = dfs(m, graph, color, stack) {
                        return Some(c);
                    }
                }
                _ => {}
            }
        }
        stack.pop();
        color.insert(n.to_string(), 2);
        None
    }
    let mut color = HashMap::new();
    let mut keys: Vec<&String> = graph.keys().collect();
    keys.sort();
    for k in keys {
        if color.get(k.as_str()).copied().unwrap_or(0) == 0 {
            let mut stack = Vec::new();
            if let Some(c) = dfs(k, graph, &mut color, &mut stack) {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(n: f64) -> Option<Input> {
        Some(Input::Value(n))
    }

    #[test]
    fn precedence() {
        let e = parse("2 + 3 * (4 - 1)").unwrap();
        assert_eq!(eval(&e, &|_| None), Eval::Value(11.0));
        assert_eq!(eval(&parse("-2 * -3").unwrap(), &|_| None), Eval::Value(6.0));
        assert_eq!(eval(&parse("10 - 4 - 3").unwrap(), &|_| None), Eval::Value(3.0));
        assert_eq!(eval(&parse("8 / 4 / 2").unwrap(), &|_| None), Eval::Value(1.0));
    }

    #[test]
    fn refs_and_div() {
        let e = parse("revenue / orders").unwrap();
        assert_eq!(refs(&e), vec!["orders", "revenue"]);
        let r = eval(&e, &|s| match s {
            "revenue" => v(2940.0),
            "orders" => v(37.0),
            _ => None,
        });
        assert!(matches!(r, Eval::Value(x) if (x - 79.459).abs() < 0.01));
    }

    #[test]
    fn div_zero() {
        let e = parse("a / b").unwrap();
        let r = eval(&e, &|s| if s == "a" { v(1.0) } else { v(0.0) });
        assert_eq!(r, Eval::Error("division by zero".into()));
    }

    #[test]
    fn overflow_is_error() {
        let e = parse("a * a").unwrap();
        assert!(matches!(eval(&e, &|_| v(1e200)), Eval::Error(_)));
    }

    #[test]
    fn stale_propagates() {
        let e = parse("a + b").unwrap();
        let r = eval(&e, &|s| if s == "a" { v(1.0) } else { Some(Input::Stale) });
        assert_eq!(r, Eval::Stale);
    }

    #[test]
    fn error_beats_stale() {
        let e = parse("a + b").unwrap();
        let r = eval(&e, &|s| if s == "a" { Some(Input::Stale) } else { Some(Input::Error) });
        assert!(matches!(r, Eval::Error(_)));
    }

    #[test]
    fn unknown_ref() {
        assert_eq!(eval(&parse("x + 1").unwrap(), &|_| None), Eval::Error("unknown entry `x`".into()));
    }

    #[test]
    fn syntax_errors() {
        for s in ["a + * b", "(a", "a)", "", "   ", "a b", "1..2", "a $ b"] {
            assert!(parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn unicode_ops() {
        assert_eq!(eval(&parse("6 ÷ 2 × 3").unwrap(), &|_| None), Eval::Value(9.0));
        assert_eq!(eval(&parse("5 − 2").unwrap(), &|_| None), Eval::Value(3.0));
    }

    #[test]
    fn cycle() {
        let g = HashMap::from([
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["a".to_string()]),
            ("c".to_string(), vec![]),
        ]);
        assert!(find_cycle(&g).is_some());
        let ok = HashMap::from([("a".to_string(), vec!["b".to_string()]), ("b".to_string(), vec![])]);
        assert!(find_cycle(&ok).is_none());
        let selfref = HashMap::from([("a".to_string(), vec!["a".to_string()])]);
        assert_eq!(find_cycle(&selfref), Some(vec!["a".to_string(), "a".to_string()]));
    }
}
