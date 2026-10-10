//! F3 Filter expressions per chat panel, in Chatterino's language so filters can be
//! copied over: `author.subbed && message.content contains "hello"`. A panel shows a
//! message only if every filter it uses says true. Also decides which messages go to a
//! routed tab (F4).

mod parse;
mod vars;

use parse::{Expr, Op};
pub use vars::{Facts, Type, Value, Var};

#[derive(Clone, Debug)]
pub struct Filter {
    expr: Expr,
}

impl Filter {
    /// Reads and type-checks a filter. The error says what's wrong, ready to show.
    pub fn parse(text: &str) -> Result<Self, String> {
        parse::parse(text).map(|expr| Self { expr })
    }

    pub fn matches(&self, facts: &Facts<'_>) -> bool {
        matches!(eval(&self.expr, facts), Value::Bool(true))
    }
}

/// The names a filter can use, for the editor's hints.
pub fn names() -> impl Iterator<Item = (&'static str, Type)> {
    Var::ALL.iter().map(|(v, name)| (*name, v.ty()))
}

fn text(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Str(s) => s.clone(),
        Value::List(l) => l.iter().map(text).collect::<Vec<_>>().join(", "),
    }
}

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Str(_), _) | (_, Value::Str(_)) => text(a).to_lowercase() == text(b).to_lowercase(),
        (Value::List(x), Value::List(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b)),
        _ => a == b,
    }
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        Value::Bool(b) => i64::from(*b),
        _ => 0,
    }
}

fn eval(e: &Expr, f: &Facts<'_>) -> Value {
    match e {
        Expr::Int(n) => Value::Int(*n),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Regex(re) => Value::Str(re.as_str().to_owned()),
        Expr::List(items) => Value::List(items.iter().map(|i| eval(i, f)).collect()),
        Expr::Var(v) => f.get(*v),
        Expr::Not(inner) => Value::Bool(!matches!(eval(inner, f), Value::Bool(true))),
        Expr::Bin(op, l, r) => bin(*op, l, r, f),
    }
}

fn bin(op: Op, l: &Expr, r: &Expr, f: &Facts<'_>) -> Value {
    let truthy = |e: &Expr| matches!(eval(e, f), Value::Bool(true));
    match op {
        Op::Or => return Value::Bool(truthy(l) || truthy(r)),
        Op::And => return Value::Bool(truthy(l) && truthy(r)),
        Op::Match => {
            let subject = text(&eval(l, f));
            return match r {
                Expr::Regex(re) => Value::Bool(re.is_match(&subject)),
                Expr::List(items) => match (items.first(), items.get(1).map(|g| eval(g, f))) {
                    (Some(Expr::Regex(re)), Some(Value::Int(group))) => Value::Str(
                        re.captures(&subject)
                            .and_then(|c| c.get(group.max(0) as usize))
                            .map(|m| m.as_str().to_owned())
                            .unwrap_or_default(),
                    ),
                    _ => Value::Str(String::new()),
                },
                _ => Value::Bool(false),
            };
        }
        _ => {}
    }
    let (a, b) = (eval(l, f), eval(r, f));
    match op {
        Op::Eq => Value::Bool(same(&a, &b)),
        Op::Ne => Value::Bool(!same(&a, &b)),
        Op::Lt => Value::Bool(int(&a) < int(&b)),
        Op::Gt => Value::Bool(int(&a) > int(&b)),
        Op::Le => Value::Bool(int(&a) <= int(&b)),
        Op::Ge => Value::Bool(int(&a) >= int(&b)),
        Op::Contains | Op::StartsWith | Op::EndsWith => Value::Bool(match &a {
            Value::List(list) => match op {
                Op::Contains => list.iter().any(|x| same(x, &b)),
                Op::StartsWith => list.first().is_some_and(|x| same(x, &b)),
                _ => list.last().is_some_and(|x| same(x, &b)),
            },
            _ => {
                let (hay, needle) = (text(&a).to_lowercase(), text(&b).to_lowercase());
                match op {
                    Op::Contains => hay.contains(&needle),
                    Op::StartsWith => hay.starts_with(&needle),
                    _ => hay.ends_with(&needle),
                }
            }
        }),
        Op::Add => match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x.saturating_add(*y)),
            _ => Value::Str(text(&a) + &text(&b)),
        },
        Op::Sub => Value::Int(int(&a).saturating_sub(int(&b))),
        Op::Mul => Value::Int(int(&a).saturating_mul(int(&b))),
        Op::Div => Value::Int(int(&a).checked_div(int(&b)).unwrap_or(0)),
        Op::Mod => Value::Int(int(&a).checked_rem(int(&b)).unwrap_or(0)),
        Op::Or | Op::And | Op::Match => unreachable!("handled above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{Ctx, build};
    use crate::irc::Line;
    use crate::model::ChatMessage;

    fn msg(raw: &str) -> ChatMessage {
        let mut colors = Default::default();
        let mut ctx = Ctx::new(None, &mut colors, 0);
        match build(&Line::parse(raw).unwrap(), &mut ctx) {
            Some(crate::build::Incoming::Message(m)) => m,
            _ => panic!(),
        }
    }

    fn check(filter: &str, m: &ChatMessage) -> bool {
        Filter::parse(filter).unwrap().matches(&Facts {
            msg: m,
            channel: "forsen",
            live: true,
        })
    }

    #[test]
    fn filters_match() {
        let m = msg(
            "@badge-info=subscriber/14;badges=subscriber/12,vip/1;display-name=Bob;id=1;user-id=7;bits=100 :bob!bob@bob PRIVMSG #forsen :Hello cheer100 World",
        );
        assert!(check("author.subbed && author.sub_length >= 12", &m));
        assert!(check(r#"message.content contains "hello""#, &m));
        assert!(check(
            r#"author.badges contains "VIP" && channel.name == "FORSEN""#,
            &m
        ));
        assert!(check(r#"message.content match r"cheer\d+""#, &m));
        assert!(check(
            r#"(message.content match {ri"(world)", 1}) == "world""#,
            &m
        ));
        assert!(check(
            "bits.amount == 100 && flags.cheer_message && !flags.reply",
            &m
        ));
        assert!(!check(r#"author.name startswith "x""#, &m));
        assert!(check(r#""a" + 1 == "a1""#, &m));
        assert!(check("1 + 2 * 3 == 9", &m), "math runs left to right");
    }

    #[test]
    fn errors_are_caught_up_front() {
        assert!(Filter::parse("").is_err());
        assert!(Filter::parse("author.nope").is_err());
        assert!(
            Filter::parse("message.content")
                .unwrap_err()
                .contains("true or false")
        );
        assert!(Filter::parse("author.subbed &&").is_err());
        assert!(Filter::parse(r#"message.length contains r"x""#).is_err());
        assert!(Filter::parse(r#"message.content match r"(""#).is_err());
        assert!(Filter::parse("(flags.reply").is_err());
    }
}
