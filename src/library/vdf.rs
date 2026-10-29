//! Tiny reader for Valve's KeyValues text format (`.vdf`, `.acf`).

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Obj(Obj),
}

/// Keys are lowercased: Valve isn't consistent about case (`LastPlayed` vs `lastplayed`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Obj(pub BTreeMap<String, Value>);

impl Obj {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(&key.to_ascii_lowercase())
    }
    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Str(s) => Some(s),
            Value::Obj(_) => None,
        }
    }
    pub fn obj(&self, key: &str) -> Option<&Obj> {
        match self.get(key)? {
            Value::Obj(o) => Some(o),
            Value::Str(_) => None,
        }
    }
}

enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(src: &str) -> Vec<Token> {
    let mut out = vec![];
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Token::Open),
            '}' => out.push(Token::Close),
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => {}
                        },
                        _ => s.push(c),
                    }
                }
                out.push(Token::Str(s));
            }
            c if c.is_whitespace() => {}
            c => {
                // Unquoted token, e.g. conditionals like [$WIN32]. Read to whitespace.
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || n == '{' || n == '}' || n == '"' {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                if !s.starts_with('[') {
                    out.push(Token::Str(s));
                }
            }
        }
    }
    out
}

fn parse_obj(tokens: &mut std::iter::Peekable<std::vec::IntoIter<Token>>) -> Obj {
    let mut obj = Obj::default();
    while let Some(tok) = tokens.next() {
        let key = match tok {
            Token::Str(k) => k.to_ascii_lowercase(),
            Token::Close => break,
            Token::Open => continue,
        };
        match tokens.next() {
            Some(Token::Str(v)) => {
                obj.0.insert(key, Value::Str(v));
            }
            Some(Token::Open) => {
                obj.0.insert(key, Value::Obj(parse_obj(tokens)));
            }
            Some(Token::Close) | None => break,
        }
    }
    obj
}

/// Parses a whole file. The root is an object holding the top-level key(s).
pub fn parse(src: &str) -> Obj {
    let mut tokens = tokenize(src).into_iter().peekable();
    parse_obj(&mut tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_appmanifest() {
        let src = r#"
"AppState"
{
	"appid"		"620"
	"name"		"Portal 2"
	"installdir"		"Portal 2"
	"LastPlayed"		"1729000000"
	"UserConfig" { "language" "english" }
}"#;
        let root = parse(src);
        let app = root.obj("AppState").unwrap();
        assert_eq!(app.str("name"), Some("Portal 2"));
        assert_eq!(app.str("lastplayed"), Some("1729000000"));
        assert_eq!(app.obj("userconfig").unwrap().str("language"), Some("english"));
    }

    #[test]
    fn reads_libraryfolders_with_escapes_and_comments() {
        let src = r#"
// written by steam
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps" { "228980" "1" "620" "123" }
	}
	"1" { "path" "D:\\Games\\Steam" }
}"#;
        let root = parse(src);
        let libs = root.obj("libraryfolders").unwrap();
        assert_eq!(libs.obj("0").unwrap().str("path"), Some(r"C:\Program Files (x86)\Steam"));
        assert_eq!(libs.obj("1").unwrap().str("path"), Some(r"D:\Games\Steam"));
        assert_eq!(libs.obj("0").unwrap().obj("apps").unwrap().0.len(), 2);
    }
}
