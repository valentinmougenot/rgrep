use crate::ast::Ast;

pub(crate) fn required_literal(ast: &Ast) -> Option<String> {
    let mut best_run = None;
    let mut current_run: Option<String> = None;

    match ast {
        Ast::Literal(c) => return Some(c.to_string()),
        Ast::Group(inner) => return required_literal(inner),
        Ast::Concat(items) => {
            for item in items {
                if let Some(result) = exact_literal(item) {
                    if let Some(ref mut current) = current_run {
                        current.push_str(&result);
                    } else {
                        current_run = Some(result);
                    }
                } else {
                    best_run = longer(best_run, current_run.take());

                    let literal_run = required_literal(item);
                    best_run = longer(best_run, literal_run);
                }
            }
        }
        Ast::Repetition { kind: _, inner: _ }
        | Ast::CharClass {
            negative: _,
            items: _,
        }
        | Ast::Dot
        | Ast::Alternation(_) => return None,
        Ast::StartAnchor | Ast::EndAnchor | Ast::WordBoundary | Ast::WordStart | Ast::WordEnd => {
            return Some(String::new());
        }
    }

    longer(best_run, current_run)
}

fn exact_literal(ast: &Ast) -> Option<String> {
    let mut current_run = String::new();

    match ast {
        Ast::Literal(c) => return Some(c.to_string()),
        Ast::Group(inner) => return exact_literal(inner),
        Ast::Concat(items) => {
            for item in items {
                let result = exact_literal(item);

                if let Some(value) = result {
                    current_run.push_str(&value);
                } else {
                    return None;
                }
            }

            Some(current_run)
        }
        Ast::Repetition { kind: _, inner: _ }
        | Ast::CharClass {
            negative: _,
            items: _,
        }
        | Ast::Dot
        | Ast::Alternation(_) => return None,
        Ast::StartAnchor | Ast::EndAnchor | Ast::WordBoundary | Ast::WordStart | Ast::WordEnd => {
            return Some(String::new());
        }
    }
}

fn longer(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) if a.len() >= b.len() => Some(a),
        (Some(_), Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}
