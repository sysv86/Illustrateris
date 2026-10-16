//! Minimal, dependency-free JSON parser and serializer.
//!
//! Objects are stored in a `BTreeMap`, so serialization is canonical (sorted keys).
//! That property is used to derive a deterministic seed from the input context.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(BTreeMap<String, Value>),
}

#[derive(Debug, Clone)]
pub struct JsonError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON error at {}:{}: {}", self.line, self.col, self.msg)
    }
}

impl std::error::Error for JsonError {}

const MAX_DEPTH: usize = 128;

pub fn parse(src: &str) -> Result<Value, JsonError> {
    let mut p = Parser { s: src.as_bytes(), pos: 0 };
    p.skip_ws();
    let v = p.value(0)?;
    p.skip_ws();
    if p.pos != p.s.len() {
        return Err(p.err("trailing characters after JSON value"));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> JsonError {
        let mut line = 1;
        let mut col = 1;
        let end = self.pos.min(self.s.len());
        for &b in &self.s[..end] {
            if b == b'\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        JsonError { msg: msg.to_string(), line, col }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), JsonError> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{}'", b as char)))
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, JsonError> {
        if self.s[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(v)
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.err("nesting too deep"));
        }
        match self.peek() {
            None => Err(self.err("unexpected end of input")),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b) if b == b'-' || b.is_ascii_digit() => self.number(),
            Some(_) => Err(self.err("unexpected character")),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, JsonError> {
        self.expect(b'[')?;
        self.skip_ws();
        let mut out = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Arr(out));
        }
        loop {
            self.skip_ws();
            out.push(self.value(depth + 1)?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Arr(out));
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, JsonError> {
        self.expect(b'{')?;
        self.skip_ws();
        let mut out = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Obj(out));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected string key"));
            }
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let v = self.value(depth + 1)?;
            out.insert(key, v);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Obj(out));
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut v: u32 = 0;
        for _ in 0..4 {
            let b = match self.peek() {
                Some(b) => b,
                None => return Err(self.err("short \\u escape")),
            };
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.err("invalid hex digit")),
            };
            v = v * 16 + d as u32;
            self.pos += 1;
        }
        Ok(v)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut buf: Vec<u8> = Vec::new();
        loop {
            let b = match self.peek() {
                Some(b) => b,
                None => return Err(self.err("unterminated string")),
            };
            self.pos += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let e = match self.peek() {
                        Some(e) => e,
                        None => return Err(self.err("unterminated escape")),
                    };
                    self.pos += 1;
                    match e {
                        b'"' => buf.push(b'"'),
                        b'\\' => buf.push(b'\\'),
                        b'/' => buf.push(b'/'),
                        b'b' => buf.push(8),
                        b'f' => buf.push(12),
                        b'n' => buf.push(b'\n'),
                        b'r' => buf.push(b'\r'),
                        b't' => buf.push(b'\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp: u32 = if (0xD800..0xDC00).contains(&hi) {
                                if self.peek() == Some(b'\\') && self.s.get(self.pos + 1) == Some(&b'u') {
                                    self.pos += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return Err(self.err("invalid low surrogate"));
                                    }
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                } else {
                                    return Err(self.err("lone high surrogate"));
                                }
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return Err(self.err("lone low surrogate"));
                            } else {
                                hi
                            };
                            let ch = match char::from_u32(cp) {
                                Some(c) => c,
                                None => return Err(self.err("invalid code point")),
                            };
                            let mut tmp = [0u8; 4];
                            buf.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                }
                0..=0x1F => return Err(self.err("control character in string")),
                _ => buf.push(b),
            }
        }
        match String::from_utf8(buf) {
            Ok(s) => Ok(s),
            Err(_) => Err(self.err("invalid UTF-8")),
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
            }
            Some(b) if b.is_ascii_digit() => {
                self.digits();
            }
            _ => return Err(self.err("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.err("digits expected after '.'"));
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(self.err("digits expected in exponent"));
            }
        }
        let text = match std::str::from_utf8(&self.s[start..self.pos]) {
            Ok(t) => t,
            Err(_) => return Err(self.err("invalid number")),
        };
        let n: f64 = match text.parse() {
            Ok(n) => n,
            Err(_) => return Err(self.err("invalid number")),
        };
        if !n.is_finite() {
            return Err(self.err("number out of range"));
        }
        Ok(Value::Num(n))
    }
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Obj(m) => Some(m),
            _ => None,
        }
    }

    /// Compact serialization.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, None, 0);
        out
    }

    /// Pretty serialization with two-space indentation.
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, Some(2), 0);
        out
    }

    fn write(&self, out: &mut String, indent: Option<usize>, level: usize) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Num(n) => write_num(out, *n),
            Value::Str(s) => write_str(out, s),
            Value::Arr(a) => {
                if a.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent, level + 1);
                    v.write(out, indent, level + 1);
                }
                newline(out, indent, level);
                out.push(']');
            }
            Value::Obj(m) => {
                if m.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, indent, level + 1);
                    write_str(out, k);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, level + 1);
                }
                newline(out, indent, level);
                out.push('}');
            }
        }
    }
}

fn newline(out: &mut String, indent: Option<usize>, level: usize) {
    if let Some(n) = indent {
        out.push('\n');
        for _ in 0..(n * level) {
            out.push(' ');
        }
    }
}

fn write_num(out: &mut String, n: f64) {
    if !n.is_finite() {
        out.push_str("null");
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        out.push_str(&format!("{}", n as i64));
    } else {
        out.push_str(&format!("{}", n));
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---- builders -------------------------------------------------------------

pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    Value::Obj(m)
}

pub fn arr(items: Vec<Value>) -> Value {
    Value::Arr(items)
}

pub fn string<S: Into<String>>(s: S) -> Value {
    Value::Str(s.into())
}

pub fn boolean(b: bool) -> Value {
    Value::Bool(b)
}

/// Number rounded to 4 decimals (keeps reports stable and compact).
pub fn num(x: f64) -> Value {
    Value::Num((x * 10_000.0).round() / 10_000.0)
}

/// Exact integer-valued number.
pub fn int(x: usize) -> Value {
    Value::Num(x as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_basic() {
        let src = r#"{"a":[1,2.5,-3e2,true,false,null],"b":{"c":"x\ny\u00e9\ud83d\ude00"}}"#;
        let v = parse(src).unwrap();
        let again = parse(&v.to_json()).unwrap();
        assert_eq!(v, again);
        assert_eq!(v.get("b").unwrap().get("c").unwrap().as_str().unwrap(), "x\nyé😀");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("{").is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\":1} x").is_err());
        assert!(parse("\"\\ud800\"").is_err());
        assert!(parse("01").is_err());
    }

    #[test]
    fn pretty_is_parseable() {
        let v = parse(r#"{"k":[{"z":1},{"y":[]}]}"#).unwrap();
        assert_eq!(parse(&v.to_json_pretty()).unwrap(), v);
    }
}
