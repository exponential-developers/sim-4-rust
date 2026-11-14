/*!
 * Strategy conditions from data.json, e.g. `(Idle || Semi-Idle) && rho < 210 || rho < 25`.
 *
 * Port of the expression handling in sim-3's `strats.ts`, which turns each condition into a
 * JavaScript function. Here conditions are parsed once into a small expression tree.
 *
 * Supported: the play styles (`Very-Active`, `Active`, `Semi-Idle`, `Idle`), `rho`, `tau`,
 * `lastStrat`, `true`, `false`, numbers, quoted strings, `!`, `&&`, `||`, `<`, `<=`, `>`, `>=`,
 * `==`/`===`, `!=`/`!==` and parentheses, with JavaScript precedence.
 *
 * Identifiers are case-insensitive and `-`/`_` are interchangeable, like in sim-3.
 * Unlike sim-3, quoted strings keep their case: sim-3 lowercases the whole expression, so
 * `lastStrat !== "T4C3"` compares against "t4c3" and can never match a real strat name.
 */

use serde::{Deserialize, Deserializer, de};
use std::cmp::Ordering;

/** Values a condition is evaluated with */
#[derive(Debug, Clone)]
pub struct ConditionArgs<'a> {
    pub very_active: bool,
    pub active: bool,
    pub semi_idle: bool,
    pub idle: bool,
    pub tau: f64,
    pub rho: f64,
    pub last_strat: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Var {
    VeryActive,
    Active,
    SemiIdle,
    Idle,
    Tau,
    Rho,
    LastStrat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Bool(bool),
    Num(f64),
    Str(String),
    Var(Var),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Cmp(CmpOp, Box<Expr>, Box<Expr>),
}

/** A parsed strategy condition. An empty condition is always true (sim-3 behavior). */
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    expr: Expr,
}

impl Default for Condition {
    fn default() -> Self {
        Condition {
            expr: Expr::Bool(true),
        }
    }
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Condition::parse(&s).map_err(de::Error::custom)
    }
}

/** Runtime value, with JavaScript-like conversions */
enum Val<'a> {
    Bool(bool),
    Num(f64),
    Str(&'a str),
}

impl Val<'_> {
    fn truthy(&self) -> bool {
        match self {
            Val::Bool(b) => *b,
            Val::Num(n) => *n != 0. && !n.is_nan(),
            Val::Str(s) => !s.is_empty(),
        }
    }
    fn to_num(&self) -> f64 {
        match self {
            Val::Bool(b) => {
                if *b {
                    1.
                } else {
                    0.
                }
            }
            Val::Num(n) => *n,
            Val::Str(s) => s.trim().parse().unwrap_or(f64::NAN),
        }
    }
}

impl Condition {
    pub fn parse(src: &str) -> Result<Condition, String> {
        if src.trim().is_empty() {
            return Ok(Condition::default());
        }
        let tokens = tokenize(src)?;
        let mut p = Parser {
            tokens: &tokens,
            pos: 0,
        };
        let expr = p.or()?;
        if p.pos != tokens.len() {
            return Err(format!(
                "Unexpected {:?} in condition \"{src}\"",
                tokens[p.pos]
            ));
        }
        Ok(Condition { expr })
    }

    pub fn eval(&self, args: &ConditionArgs) -> bool {
        eval(&self.expr, args).truthy()
    }
}

fn eval<'a>(e: &'a Expr, a: &ConditionArgs<'a>) -> Val<'a> {
    match e {
        Expr::Bool(b) => Val::Bool(*b),
        Expr::Num(n) => Val::Num(*n),
        Expr::Str(s) => Val::Str(s),
        Expr::Var(v) => match v {
            Var::VeryActive => Val::Bool(a.very_active),
            Var::Active => Val::Bool(a.active),
            Var::SemiIdle => Val::Bool(a.semi_idle),
            Var::Idle => Val::Bool(a.idle),
            Var::Tau => Val::Num(a.tau),
            Var::Rho => Val::Num(a.rho),
            Var::LastStrat => Val::Str(a.last_strat),
        },
        Expr::Not(x) => Val::Bool(!eval(x, a).truthy()),
        Expr::And(l, r) => Val::Bool(eval(l, a).truthy() && eval(r, a).truthy()),
        Expr::Or(l, r) => Val::Bool(eval(l, a).truthy() || eval(r, a).truthy()),
        Expr::Cmp(op, l, r) => {
            let (l, r) = (eval(l, a), eval(r, a));
            let ord = match (&l, &r) {
                (Val::Str(x), Val::Str(y)) => Some(x.cmp(y)),
                _ => l.to_num().partial_cmp(&r.to_num()),
            };
            // Equality is strict: values of different types are never equal
            let same_type = matches!(
                (&l, &r),
                (Val::Str(_), Val::Str(_))
                    | (Val::Num(_), Val::Num(_))
                    | (Val::Bool(_), Val::Bool(_))
            );
            let eq = same_type && ord == Some(Ordering::Equal);
            Val::Bool(match op {
                CmpOp::Lt => ord == Some(Ordering::Less),
                CmpOp::Le => matches!(ord, Some(Ordering::Less | Ordering::Equal)),
                CmpOp::Gt => ord == Some(Ordering::Greater),
                CmpOp::Ge => matches!(ord, Some(Ordering::Greater | Ordering::Equal)),
                CmpOp::Eq => eq,
                CmpOp::Ne => !eq,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Num(f64),
    Str(String),
    Op(&'static str),
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    const OPS: [&str; 13] = [
        "===", "!==", "&&", "||", "==", "!=", "<=", ">=", "<", ">", "!", "(", ")",
    ];
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    'outer: while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-')
            {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| {
                format!("Invalid number \"{s}\" in condition \"{src}\"")
            })?));
        } else if c == '"' || c == '\'' {
            let start = i + 1;
            i += 1;
            while i < chars.len() && chars[i] != c {
                i += 1;
            }
            if i >= chars.len() {
                return Err(format!("Unterminated string in condition \"{src}\""));
            }
            out.push(Tok::Str(chars[start..i].iter().collect()));
            i += 1;
        } else {
            for op in OPS {
                let len = op.chars().count();
                if chars[i..].iter().take(len).copied().eq(op.chars()) {
                    out.push(Tok::Op(op));
                    i += len;
                    continue 'outer;
                }
            }
            return Err(format!("Unexpected character '{c}' in condition \"{src}\""));
        }
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: &'a [Tok],
    pos: usize,
}

impl Parser<'_> {
    fn peek_op(&self) -> Option<&'static str> {
        match self.tokens.get(self.pos) {
            Some(Tok::Op(o)) => Some(o),
            _ => None,
        }
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut l = self.and()?;
        while self.peek_op() == Some("||") {
            self.pos += 1;
            l = Expr::Or(Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut l = self.equality()?;
        while self.peek_op() == Some("&&") {
            self.pos += 1;
            l = Expr::And(Box::new(l), Box::new(self.equality()?));
        }
        Ok(l)
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut l = self.relational()?;
        while let Some(op) = self.peek_op().and_then(|o| match o {
            "==" | "===" => Some(CmpOp::Eq),
            "!=" | "!==" => Some(CmpOp::Ne),
            _ => None,
        }) {
            self.pos += 1;
            l = Expr::Cmp(op, Box::new(l), Box::new(self.relational()?));
        }
        Ok(l)
    }

    fn relational(&mut self) -> Result<Expr, String> {
        let mut l = self.unary()?;
        while let Some(op) = self.peek_op().and_then(|o| match o {
            "<" => Some(CmpOp::Lt),
            "<=" => Some(CmpOp::Le),
            ">" => Some(CmpOp::Gt),
            ">=" => Some(CmpOp::Ge),
            _ => None,
        }) {
            self.pos += 1;
            l = Expr::Cmp(op, Box::new(l), Box::new(self.unary()?));
        }
        Ok(l)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.peek_op() == Some("!") {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let tok = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or("Unexpected end of condition")?;
        self.pos += 1;
        match tok {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Op("(") => {
                let e = self.or()?;
                if self.peek_op() != Some(")") {
                    return Err("Missing ')' in condition".to_owned());
                }
                self.pos += 1;
                Ok(e)
            }
            Tok::Ident(name) => {
                let norm = name.to_lowercase().replace('-', "_");
                Ok(match norm.as_str() {
                    "true" => Expr::Bool(true),
                    "false" => Expr::Bool(false),
                    "very_active" => Expr::Var(Var::VeryActive),
                    "active" => Expr::Var(Var::Active),
                    "semi_idle" => Expr::Var(Var::SemiIdle),
                    "idle" => Expr::Var(Var::Idle),
                    "tau" => Expr::Var(Var::Tau),
                    "rho" => Expr::Var(Var::Rho),
                    "laststrat" => Expr::Var(Var::LastStrat),
                    _ => return Err(format!("Unknown name \"{name}\" in condition")),
                })
            }
            Tok::Op(o) => Err(format!("Unexpected '{o}' in condition")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args<'a>(style: &str, rho: f64, last: &'a str) -> ConditionArgs<'a> {
        ConditionArgs {
            very_active: style == "va",
            active: style == "a",
            semi_idle: style == "si",
            idle: style == "i",
            tau: rho,
            rho,
            last_strat: last,
        }
    }

    #[test]
    fn precedence_and_styles() {
        let c = Condition::parse("(Idle || Semi-Idle) && rho < 210 || rho < 25").unwrap();
        assert!(c.eval(&args("i", 100., "")));
        assert!(!c.eval(&args("i", 300., "")));
        assert!(!c.eval(&args("a", 100., "")));
        assert!(c.eval(&args("a", 20., "")));

        let c = Condition::parse("Semi-Idle && rho < 180 || !Idle && rho < 25").unwrap();
        assert!(c.eval(&args("a", 20., "")));
        assert!(!c.eval(&args("i", 20., "")));
        assert!(c.eval(&args("si", 100., "")));
    }

    #[test]
    fn strings_and_literals() {
        let c = Condition::parse("(Idle || Semi-Idle) && lastStrat !== \"T4C3\"").unwrap();
        assert!(c.eval(&args("i", 0., "T4C12")));
        assert!(!c.eval(&args("i", 0., "T4C3")));
        assert!(!Condition::parse("false").unwrap().eval(&args("i", 0., "")));
        assert!(Condition::parse("").unwrap().eval(&args("i", 0., "")));
        assert!(
            Condition::parse("tau >= 1e2")
                .unwrap()
                .eval(&args("i", 100., ""))
        );
    }

    #[test]
    fn errors() {
        assert!(Condition::parse("rho <").is_err());
        assert!(Condition::parse("foo > 1").is_err());
        assert!(Condition::parse("(rho > 1").is_err());
        let err = |src: &str| Condition::parse(src).unwrap_err();
        assert!(err("lastStrat == \"T1").contains("Unterminated string"));
        assert!(err("rho > 1.2.3").contains("Invalid number"));
        assert!(err("rho # 3").contains("Unexpected character '#'"));
        assert!(err("rho > 1 )").contains("Unexpected"));
        assert!(err("&& rho").contains("Unexpected '&&'"));
    }

    #[test]
    fn javascript_value_conversions() {
        let a = |rho: f64, last: &'static str| ConditionArgs {
            very_active: false,
            active: true,
            semi_idle: false,
            idle: false,
            tau: rho,
            rho,
            last_strat: last,
        };
        let eval = |src: &str, args: &ConditionArgs| Condition::parse(src).unwrap().eval(args);
        // a number on its own is true unless it is 0 or NaN
        assert!(eval("rho", &a(5., "")));
        assert!(!eval("rho", &a(0., "")));
        assert!(!eval("rho", &a(f64::NAN, "")));
        // booleans compare as 0/1, strings as numbers when compared with a number
        assert!(eval("Active > 0", &a(0., "")));
        assert!(!eval("Idle > 0", &a(0., "")));
        assert!(eval("lastStrat < 5", &a(0., "3")));
        assert!(!eval("lastStrat < 5", &a(0., "abc"))); // NaN
        // <= and ==, strict: different types are never equal
        assert!(eval("rho <= 5", &a(5., "")) && !eval("rho <= 5", &a(6., "")));
        assert!(eval("rho >= 5", &a(5., "")));
        assert!(eval("rho == 5", &a(5., "")));
        assert!(!eval("lastStrat == 5", &a(0., "5")));
        assert!(eval("lastStrat === \"T1\"", &a(0., "T1")));
        assert!(eval("rho > 1e+2", &a(150., "")));
        assert!(eval("rho > .5", &a(1., "")));
    }

    /** Every condition in both sims' data.json must parse */
    #[test]
    fn all_data_json_conditions_parse() {
        for json in [include_str!("../../../src/Data/data.json")] {
            let v: serde_json::Value = serde_json::from_str(json).unwrap();
            for (theory, data) in v["theories"].as_object().unwrap() {
                let Some(strats) = data["strats"].as_object() else {
                    continue;
                };
                for (strat, s) in strats {
                    for key in ["stratFilterCondition", "forcedCondition"] {
                        if let Some(src) = s[key].as_str() {
                            Condition::parse(src)
                                .unwrap_or_else(|e| panic!("{theory} {strat} {key}: {e}"));
                        }
                    }
                }
            }
        }
    }
}
