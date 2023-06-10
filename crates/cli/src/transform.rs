use ast_grep_core::meta_var::{MetaVarEnv, MetaVariable};
use ast_grep_core::{Language, StrDoc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubString {
  source: String,
  start_char: Option<i32>,
  end_char: Option<i32>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Replace {
  source: String,
  replace: String,
  by: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Transformation {
  SubString(SubString),
  Replace(Replace),
}

impl Transformation {
  fn insert<L: Language>(&self, key: &str, lang: &L, env: &mut MetaVarEnv<StrDoc<L>>) {
    // avoid cyclic
    env.insert_transformation(key.to_string(), vec![]);
    let Some(s) = self.compute(lang, env) else {
      return;
    };
    env.insert_transformation(key.to_string(), s.into_bytes());
  }
  fn compute<L: Language>(&self, lang: &L, env: &mut MetaVarEnv<StrDoc<L>>) -> Option<String> {
    use Transformation as T;
    match self {
      T::Replace(r) => {
        let source = lang.pre_process_pattern(&r.source);
        let node = match lang.extract_meta_var(&source)? {
          MetaVariable::Named(n, _) => env.get_match(&n)?,
          _ => return None,
        };
        let text = node.text();
        let re = Regex::new(&r.replace).unwrap();
        Some(re.replace_all(&text, &r.by).into_owned())
      }
      T::SubString(s) => {
        let source = lang.pre_process_pattern(&s.source);
        let node = match lang.extract_meta_var(&source)? {
          MetaVariable::Named(n, _) => env.get_match(&n)?,
          _ => return None,
        };
        let text = node.text();
        let chars: Vec<_> = text.chars().collect();
        let len = chars.len() as i32;
        let start = resolve_char(&s.start_char, 0, len);
        let end = resolve_char(&s.end_char, len, len);
        if start > end || start >= len as usize || end > len as usize {
          return Some(String::new());
        }
        Some(chars[start..end].iter().collect())
      }
    }
  }
}

fn resolve_char(opt: &Option<i32>, dft: i32, len: i32) -> usize {
  let c = *opt.as_ref().unwrap_or(&dft);
  if c < 0 {
    if len + c < 0 {
      0
    } else {
      (len + c) as usize
    }
  } else if c >= len {
    len as usize
  } else {
    c as usize
  }
}

pub fn apply_env_transform<L: Language>(
  transforms: &HashMap<String, serde_yaml::Value>,
  lang: &L,
  env: &mut MetaVarEnv<StrDoc<L>>,
) {
  for (key, val) in transforms {
    let tr: Transformation = serde_yaml::from_value(val.clone()).unwrap();
    tr.insert(key, lang, env);
  }
}
