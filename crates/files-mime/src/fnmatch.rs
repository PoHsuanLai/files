//! A small `fnmatch`: `*`, `?`, `[...]` (with `!`/`^` negation and ranges) and `\` escapes.

/// One piece of a compiled pattern.
enum Token {
    Star,
    Any,
    Lit(char),
    Class(Class),
}

/// Whether a class lists the characters it takes or the ones it refuses.
enum Polarity {
    Plain,
    Negated,
}

enum ClassItem {
    One(char),
    Range(char, char),
}

struct Class {
    polarity: Polarity,
    items: Vec<ClassItem>,
}

impl Class {
    fn contains(&self, c: char) -> bool {
        let hit = self.items.iter().any(|item| match item {
            ClassItem::One(one) => *one == c,
            ClassItem::Range(low, high) => *low <= c && c <= *high,
        });
        match self.polarity {
            Polarity::Plain => hit,
            Polarity::Negated => !hit,
        }
    }
}

impl Token {
    fn accepts(&self, c: char) -> bool {
        match self {
            Token::Star => false,
            Token::Any => true,
            Token::Lit(lit) => *lit == c,
            Token::Class(class) => class.contains(c),
        }
    }
}

/// The class that starts at `start` (just after the `[`), and where it ends; `None` when it is
/// never closed, so the `[` is an ordinary character.
fn class(chars: &[char], start: usize) -> Option<(Class, usize)> {
    let mut at = start;
    let polarity = match chars.get(at).copied() {
        Some('!' | '^') => {
            at += 1;
            Polarity::Negated
        }
        Some(_) | None => Polarity::Plain,
    };
    let mut items = Vec::new();
    let mut first = true;
    loop {
        let c = *chars.get(at)?;
        at += 1;
        if c == ']' && !first {
            return Some((Class { polarity, items }, at));
        }
        first = false;
        let ranged = chars.get(at) == Some(&'-') && chars.get(at + 1).is_some_and(|n| *n != ']');
        if ranged {
            let high = *chars.get(at + 1)?;
            items.push(ClassItem::Range(c, high));
            at += 2;
        } else {
            items.push(ClassItem::One(c));
        }
    }
}

fn compile(pattern: &str) -> Vec<Token> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(&c) = chars.get(at) {
        at += 1;
        match c {
            '*' => tokens.push(Token::Star),
            '?' => tokens.push(Token::Any),
            '\\' => {
                if let Some(&next) = chars.get(at) {
                    tokens.push(Token::Lit(next));
                    at += 1;
                } else {
                    tokens.push(Token::Lit('\\'));
                }
            }
            '[' => match class(&chars, at) {
                Some((class, end)) => {
                    tokens.push(Token::Class(class));
                    at = end;
                }
                None => tokens.push(Token::Lit('[')),
            },
            other => tokens.push(Token::Lit(other)),
        }
    }
    tokens
}

/// Whether the whole of `text` matches `pattern`.
pub(crate) fn matches(pattern: &str, text: &str) -> bool {
    let tokens = compile(pattern);
    let text: Vec<char> = text.chars().collect();
    let (mut t, mut s) = (0, 0);
    // Where the last `*` was and the text position it currently swallows up to.
    let mut star: Option<(usize, usize)> = None;
    while let Some(&c) = text.get(s) {
        match tokens.get(t) {
            Some(Token::Star) => {
                star = Some((t, s));
                t += 1;
                continue;
            }
            Some(token) if token.accepts(c) => {
                t += 1;
                s += 1;
                continue;
            }
            Some(_) | None => {}
        }
        match star {
            Some((star_t, star_s)) => {
                t = star_t + 1;
                s = star_s + 1;
                star = Some((star_t, star_s + 1));
            }
            None => return false,
        }
    }
    tokens
        .get(t..)
        .unwrap_or_default()
        .iter()
        .all(|token| matches!(token, Token::Star))
}
