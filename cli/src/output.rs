//! Output formatting for CLI commands.
//!
//! Ports the semantics of the Python `output.py`: values are emitted as
//! pretty JSON by default (key order preserved via `serde_json`'s
//! `preserve_order` feature) or as human-friendly aligned/tabular text when
//! `--human` is requested. A `Null` value is emitted as nothing at all.

use serde_json::Value;

/// Print `value` to stdout. `Null` prints nothing (like skipping `None`).
pub fn emit(value: &Value, human: bool) {
    if value.is_null() {
        return;
    }
    if human {
        println!("{}", to_human(value));
    } else {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    }
}

/// Render a value as human-friendly text.
pub fn to_human(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return "(no results)".to_string();
            }
            if items.iter().all(Value::is_object) {
                let mut columns: Vec<String> = Vec::new();
                for item in items {
                    if let Value::Object(map) = item {
                        for key in map.keys() {
                            if !columns.iter().any(|c| c == key) {
                                columns.push(key.clone());
                            }
                        }
                    }
                }
                let mut lines = vec![columns.join("\t")];
                for item in items {
                    let cells: Vec<String> = columns
                        .iter()
                        .map(|col| scalar(item.get(col).unwrap_or(&Value::Null)))
                        .collect();
                    lines.push(cells.join("\t"));
                }
                lines.join("\n")
            } else {
                items.iter().map(scalar).collect::<Vec<_>>().join("\n")
            }
        }
        Value::Object(map) => {
            let width = map.keys().map(|k| k.len() + 1).max().unwrap_or(0);
            map.iter()
                .map(|(k, v)| format!("{:<width$} {}", format!("{k}:"), scalar(v), width = width))
                .collect::<Vec<_>>()
                .join("\n")
        }
        other => scalar(other),
    }
}

/// Render a single value as a one-line cell.
fn scalar(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(v).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn array_of_objects_renders_table() {
        let v = json!([
            {"id": 1, "name": "a"},
            {"id": 2, "name": "b"},
        ]);
        assert_eq!(to_human(&v), "id\tname\n1\ta\n2\tb");
    }

    #[test]
    fn empty_array_renders_no_results() {
        assert_eq!(to_human(&json!([])), "(no results)");
    }

    #[test]
    fn object_aligns_by_keys() {
        let v = json!({"id": 1, "longer": "x"});
        assert_eq!(to_human(&v), "id:     1\nlonger: x");
    }

    #[test]
    fn nested_object_field_is_compact_json() {
        let v = json!([{"id": 1, "meta": {"k": "v"}}]);
        assert_eq!(to_human(&v), "id\tmeta\n1\t{\"k\":\"v\"}");
    }

    #[test]
    fn null_emits_nothing() {
        // `emit` prints nothing for Null; `scalar` of Null is empty.
        assert_eq!(scalar(&Value::Null), "");
    }
}
