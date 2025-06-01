use crate::style::style_vars::StyleVars;

enum StyleExprPart {
    String(String),
    Var(String),
}

pub struct StyleExpr {
    parts: Vec<StyleExprPart>,
}

impl StyleExpr {
    pub fn parse(value: &str) -> Option<StyleExpr> {
        let mut parts = Vec::new();
        let mut has_var = false;
        let mut v = value;
        loop {
            let (part, consumed) = consume_expr_part(v)?;
            if let StyleExprPart::Var(_) = &part {
                has_var = true;
            }
            parts.push(part);
            if consumed >= v.len() {
                break;
            }
            v = &value[consumed..];
        }
        if has_var {
            Some(StyleExpr { parts })
        } else {
            None
        }
    }

    pub fn resolve(&self, vars: &StyleVars) -> Option<String> {
        let mut result = String::new();
        for part in &self.parts {
            match part {
                StyleExprPart::Var(k) => {
                    result.push_str(vars.get(k)?);
                }
                StyleExprPart::String(v) => {
                    result.push_str(v);
                }
            }
        }
        Some(result)
    }
}

fn consume_expr_part(value: &str) -> Option<(StyleExprPart, usize)> {
    if value.starts_with("var(") {
        //TODO support nested var
        let end = value.find(")")?;
        let key = value[4..end].trim();
        if !key.starts_with("--") {
            return None;
        }
        let key = key[2..].to_string();
        Some((StyleExprPart::Var(key), end + 1))
    } else {
        let pos = value.find("var(").unwrap_or(value.len());
        Some((StyleExprPart::String(String::from(&value[..pos])), pos))
    }
}

#[cfg(test)]
mod tests {
    use crate::style::style_vars::StyleVars;
    use crate::style::var_expr::StyleExpr;

    #[test]
    fn test_var_expr() {
        assert!(StyleExpr::parse("#FFF").is_none());
        let mut vars = StyleVars::new();
        vars.set("color", "#abc");
        vars.set("highlight-border-color", "#123456");

        let style_expr = StyleExpr::parse("var(--color)").unwrap();
        assert_eq!("#abc", style_expr.resolve(&vars).unwrap().as_str());

        let border_expr = StyleExpr::parse("1px var(--highlight-border-color)").unwrap();
        assert_eq!("1px #123456", border_expr.resolve(&vars).unwrap().as_str());
    }
}
