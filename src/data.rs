use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
};

use serde_derive::Deserialize;

#[derive(Clone, Debug)]
pub struct FormatString {
    pub args: Vec<FormatArg>,
    pub rest: String,
}

fn write_str<W: core::fmt::Write>(output: &mut W, str: &str) -> std::io::Result<()> {
    output
        .write_str(str)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, "fmt error"))
}

impl FormatString {
    pub fn eval<W: core::fmt::Write, S: Borrow<str> + Eq + Hash, R: AsRef<str>>(
        &self,
        default: &str,
        keys: &HashMap<S, R>,
        mut output: W,
    ) -> std::io::Result<()> {
        for arg in &self.args {
            write_str(&mut output, &arg.leading_text)?;
            match &arg.fmt {
                FormatSpec::EscapeLeftBrace => write_str(&mut output, "{")?,
                FormatSpec::EscapeRightBrace => write_str(&mut output, "}")?,
                FormatSpec::Default => write_str(&mut output, default)?,
                FormatSpec::Keyed(key) => {
                    let val = keys.get(&**key).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Unknown key {key}"),
                        )
                    })?;

                    write_str(&mut output, val.as_ref())?;
                }
            }
        }

        write_str(&mut output, &self.rest)
    }
}

#[derive(Clone, Debug)]
pub struct FormatArg {
    pub leading_text: String,
    pub fmt: FormatSpec,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum FormatSpec {
    EscapeLeftBrace,  // {{
    EscapeRightBrace, // }}
    Default,          // {}
    Keyed(String),    // {*str*}
}

fn parse_fmt_str<'a, E: serde::de::Error>(x: &'a str) -> Result<(Option<FormatArg>, &'a str), E> {
    if let Some((l, r)) = x.split_once('{') {
        if r.starts_with('{') {
            let r = &r[1..];
            Ok((
                Some(FormatArg {
                    leading_text: l.to_string(),
                    fmt: FormatSpec::EscapeLeftBrace,
                }),
                r,
            ))
        } else if let Some((arg, r)) = r.split_once('}') {
            if arg.is_empty() {
                Ok((
                    Some(FormatArg {
                        leading_text: l.to_string(),
                        fmt: FormatSpec::Default,
                    }),
                    r,
                ))
            } else {
                Ok((
                    Some(FormatArg {
                        leading_text: l.to_string(),
                        fmt: FormatSpec::Keyed(arg.to_string()),
                    }),
                    r,
                ))
            }
        } else {
            Err(E::invalid_value(
                serde::de::Unexpected::Str(x),
                &FormatStringVisitor,
            ))
        }
    } else if let Some((l, r)) = x.split_once('}') {
        if r.starts_with('}') {
            let r = &r[1..];
            Ok((
                Some(FormatArg {
                    leading_text: l.to_string(),
                    fmt: FormatSpec::EscapeRightBrace,
                }),
                r,
            ))
        } else {
            Err(E::invalid_value(
                serde::de::Unexpected::Str(x),
                &FormatStringVisitor,
            ))
        }
    } else {
        Ok((None, x))
    }
}

struct FormatStringVisitor;

impl<'de> serde::de::Visitor<'de> for FormatStringVisitor {
    type Value = FormatString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a format string containing plain text, format keys like {} or {foo}, and escaped {{ and }} sequences (interpreted as literal { and } respectively)")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, mut v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut args = Vec::new();

        loop {
            let (arg, rest) = parse_fmt_str(v)?;
            v = rest;
            match arg {
                Some(arg) => args.push(arg),
                None => break,
            }
        }

        Ok(FormatString {
            args,
            rest: v.to_string(),
        })
    }
}

impl<'de> serde::Deserialize<'de> for FormatString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(FormatStringVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct LabelSpec {
    pub repos: Option<HashSet<String>>,
    pub description: Option<FormatString>,
    pub colour: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]

pub struct LabelGroupSpec {
    #[serde(flatten)]
    pub label_spec: LabelSpec,
    #[serde(default)]
    pub repeatable: bool,
    pub pattern: FormatString,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(flatten)]
    pub elaborated_labels: HashMap<String, LabelSpec>,
    #[serde(default)]
    pub subgroups: HashMap<String, LabelGroupSpec>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LabelsFile {
    pub groups: HashMap<String, LabelGroupSpec>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReposFile {
    pub repos: HashMap<String, String>,
    #[serde(rename = "allowed-labels")]
    pub allowed_labels: AllowedLabels,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct AllowedLabels {
    pub all: HashSet<String>,
    #[serde(flatten)]
    pub by_repo: HashMap<String, HashSet<String>>,
}
