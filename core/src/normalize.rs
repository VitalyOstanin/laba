//! Flatten HAL (`_links` / `_embedded`) OpenProject payloads into flat objects.
//!
//! Each normalizer returns a [`serde_json::Value::Object`] whose key order is
//! significant: it defines the column order of the `--human` output. Object key
//! order is preserved via the `preserve_order` feature of `serde_json`.

use serde_json::{Map, Value};

use crate::duration::iso8601_to_hours;

/// Clone `p[k]`, or [`Value::Null`] if absent.
pub fn get(p: &Value, k: &str) -> Value {
    p.get(k).cloned().unwrap_or(Value::Null)
}

/// Title of a HAL link: `p["_links"][name]["title"]`, or [`Value::Null`].
pub fn link_title(p: &Value, name: &str) -> Value {
    p.get("_links")
        .and_then(|l| l.get(name))
        .and_then(|link| link.get("title"))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Numeric id from a HAL link href tail: the segment after the last `/` of
/// `p["_links"][name]["href"]`, parsed as `i64`. Non-numeric tail → [`Value::Null`].
pub fn link_id(p: &Value, name: &str) -> Value {
    p.get("_links")
        .and_then(|l| l.get(name))
        .and_then(|link| link.get("href"))
        .and_then(|href| href.as_str())
        .and_then(|href| href.rsplit('/').next())
        .and_then(|tail| tail.parse::<i64>().ok())
        .map(Value::from)
        .unwrap_or(Value::Null)
}

/// Unwrap a formattable text value: `{ "raw": ... }` → `raw`, else clone as-is.
pub fn text(v: &Value) -> Value {
    match v.get("raw") {
        Some(raw) => raw.clone(),
        None => v.clone(),
    }
}

/// Collect [`text`] of each object element of an array. Empty or non-array →
/// [`Value::Null`]; otherwise a [`Value::Array`].
pub fn details(v: &Value) -> Value {
    match v.as_array() {
        Some(items) => {
            let collected: Vec<Value> = items.iter().filter(|i| i.is_object()).map(text).collect();
            if collected.is_empty() {
                Value::Null
            } else {
                Value::Array(collected)
            }
        }
        None => Value::Null,
    }
}

/// Cloned elements of a HAL collection: `p["_embedded"]["elements"]`, or empty.
pub fn collection(p: &Value) -> Vec<Value> {
    p.get("_embedded")
        .and_then(|e| e.get("elements"))
        .and_then(|els| els.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Build an object from ordered (key, value) pairs, preserving insertion order.
fn object(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Object(map)
}

/// Flatten a work package resource.
pub fn work_package(p: &Value) -> Value {
    object(vec![
        ("id", get(p, "id")),
        ("subject", get(p, "subject")),
        ("type", link_title(p, "type")),
        ("status", link_title(p, "status")),
        ("priority", link_title(p, "priority")),
        ("project", link_title(p, "project")),
        ("projectId", link_id(p, "project")),
        ("author", link_title(p, "author")),
        ("assignee", link_title(p, "assignee")),
        ("percentageDone", get(p, "percentageDone")),
        ("startDate", get(p, "startDate")),
        ("dueDate", get(p, "dueDate")),
        ("createdAt", get(p, "createdAt")),
        ("updatedAt", get(p, "updatedAt")),
        ("lockVersion", get(p, "lockVersion")),
        ("description", text(&get(p, "description"))),
    ])
}

/// Flatten a time entry resource.
pub fn time_entry(p: &Value) -> Value {
    let hours = match get(p, "hours") {
        Value::String(s) => iso8601_to_hours(&s).map(Value::from).unwrap_or(Value::Null),
        _ => Value::Null,
    };
    object(vec![
        ("id", get(p, "id")),
        ("hours", hours),
        ("spentOn", get(p, "spentOn")),
        ("comment", text(&get(p, "comment"))),
        ("user", link_title(p, "user")),
        ("userId", link_id(p, "user")),
        ("workPackage", link_title(p, "workPackage")),
        ("workPackageId", link_id(p, "workPackage")),
        ("project", link_title(p, "project")),
        ("activity", link_title(p, "activity")),
        ("createdAt", get(p, "createdAt")),
        ("updatedAt", get(p, "updatedAt")),
    ])
}

/// Flatten a notification resource.
pub fn notification(p: &Value) -> Value {
    let read = p.get("readIAN").and_then(|v| v.as_bool()).unwrap_or(false);
    let activity_href = p
        .get("_links")
        .and_then(|l| l.get("activity"))
        .and_then(|a| a.get("href"))
        .cloned()
        .unwrap_or(Value::Null);
    object(vec![
        ("id", get(p, "id")),
        ("reason", get(p, "reason")),
        ("read", Value::Bool(read)),
        ("wpId", link_id(p, "resource")),
        ("wpTitle", link_title(p, "resource")),
        ("project", link_title(p, "project")),
        ("actor", link_title(p, "actor")),
        ("activityHref", activity_href),
        ("createdAt", get(p, "createdAt")),
    ])
}

/// Flatten a comment (activity) resource.
pub fn comment(p: &Value) -> Value {
    object(vec![
        ("id", get(p, "id")),
        ("type", get(p, "_type")),
        ("comment", text(&get(p, "comment"))),
        ("details", details(&get(p, "details"))),
        ("user", link_title(p, "user")),
        ("userId", link_id(p, "user")),
        ("createdAt", get(p, "createdAt")),
        ("version", get(p, "version")),
    ])
}

/// Flatten an attachment resource.
pub fn attachment(p: &Value) -> Value {
    let download_url = p
        .get("_links")
        .and_then(|l| l.get("downloadLocation"))
        .and_then(|d| d.get("href"))
        .cloned()
        .unwrap_or(Value::Null);
    object(vec![
        ("id", get(p, "id")),
        ("fileName", get(p, "fileName")),
        ("fileSize", get(p, "fileSize")),
        ("contentType", get(p, "contentType")),
        ("description", text(&get(p, "description"))),
        ("author", link_title(p, "author")),
        ("createdAt", get(p, "createdAt")),
        ("downloadUrl", download_url),
    ])
}

/// Flatten a relation resource.
pub fn relation(p: &Value) -> Value {
    object(vec![
        ("id", get(p, "id")),
        ("type", get(p, "type")),
        ("reverseType", get(p, "reverseType")),
        ("description", get(p, "description")),
        ("from", link_id(p, "from")),
        ("to", link_id(p, "to")),
        ("lockVersion", get(p, "lockVersion")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(v: &Value) -> Vec<String> {
        v.as_object().unwrap().keys().cloned().collect()
    }

    #[test]
    fn link_id_parses_numeric_tail() {
        let p = json!({ "_links": { "project": { "href": "/api/v3/projects/7" } } });
        assert_eq!(link_id(&p, "project"), Value::from(7i64));
    }

    #[test]
    fn link_id_null_for_identifier_tail() {
        let p = json!({ "_links": { "project": { "href": "/api/v3/projects/my-project" } } });
        assert_eq!(link_id(&p, "project"), Value::Null);
    }

    #[test]
    fn text_unwraps_raw_and_passes_string() {
        assert_eq!(text(&json!({ "raw": "x" })), json!("x"));
        assert_eq!(text(&json!("plain")), json!("plain"));
    }

    #[test]
    fn details_collects_raw_and_rejects_non_array() {
        assert_eq!(
            details(&json!([{ "raw": "a" }, { "raw": "b" }])),
            json!(["a", "b"])
        );
        assert_eq!(details(&json!("nope")), Value::Null);
        assert_eq!(details(&json!([])), Value::Null);
    }

    #[test]
    fn work_package_order_and_values() {
        let p = json!({
            "id": 1,
            "subject": "S",
            "percentageDone": 50,
            "startDate": "2026-01-01",
            "dueDate": "2026-01-02",
            "createdAt": "c",
            "updatedAt": "u",
            "lockVersion": 3,
            "description": { "raw": "D" },
            "_links": {
                "type": { "title": "Task" },
                "status": { "title": "New" },
                "priority": { "title": "Normal" },
                "project": { "href": "/api/v3/projects/7", "title": "P" },
                "author": { "title": "A" },
                "assignee": { "title": "B" }
            }
        });
        let v = work_package(&p);
        assert_eq!(
            keys(&v),
            vec![
                "id",
                "subject",
                "type",
                "status",
                "priority",
                "project",
                "projectId",
                "author",
                "assignee",
                "percentageDone",
                "startDate",
                "dueDate",
                "createdAt",
                "updatedAt",
                "lockVersion",
                "description"
            ]
        );
        assert_eq!(v["type"], json!("Task"));
        assert_eq!(v["projectId"], json!(7));
        assert_eq!(v["description"], json!("D"));
    }

    #[test]
    fn time_entry_order_and_hours() {
        let p = json!({
            "id": 2,
            "hours": "PT1H30M",
            "spentOn": "2026-01-01",
            "comment": { "raw": "note" },
            "createdAt": "c",
            "updatedAt": "u",
            "_links": {
                "user": { "href": "/api/v3/users/5", "title": "U" },
                "workPackage": { "href": "/api/v3/work_packages/9", "title": "WP" },
                "project": { "title": "P" },
                "activity": { "title": "Dev" }
            }
        });
        let v = time_entry(&p);
        assert_eq!(
            keys(&v),
            vec![
                "id",
                "hours",
                "spentOn",
                "comment",
                "user",
                "userId",
                "workPackage",
                "workPackageId",
                "project",
                "activity",
                "createdAt",
                "updatedAt"
            ]
        );
        assert_eq!(v["hours"], json!(1.5));
        assert_eq!(v["userId"], json!(5));
        assert_eq!(v["workPackageId"], json!(9));
    }

    #[test]
    fn notification_order_and_read() {
        let p = json!({
            "id": 3,
            "reason": "mentioned",
            "readIAN": true,
            "createdAt": "c",
            "_links": {
                "resource": { "href": "/api/v3/work_packages/11", "title": "WP11" },
                "project": { "title": "P" },
                "actor": { "title": "Actor" },
                "activity": { "href": "/api/v3/activities/1" }
            }
        });
        let v = notification(&p);
        assert_eq!(
            keys(&v),
            vec![
                "id",
                "reason",
                "read",
                "wpId",
                "wpTitle",
                "project",
                "actor",
                "activityHref",
                "createdAt"
            ]
        );
        assert_eq!(v["read"], json!(true));
        assert_eq!(v["wpId"], json!(11));
    }

    #[test]
    fn comment_order_and_type() {
        let p = json!({
            "id": 4,
            "_type": "Activity::Comment",
            "comment": { "raw": "hi" },
            "details": [{ "raw": "d1" }],
            "createdAt": "c",
            "version": 2,
            "_links": { "user": { "href": "/api/v3/users/8", "title": "U" } }
        });
        let v = comment(&p);
        assert_eq!(
            keys(&v),
            vec![
                "id",
                "type",
                "comment",
                "details",
                "user",
                "userId",
                "createdAt",
                "version"
            ]
        );
        assert_eq!(v["type"], json!("Activity::Comment"));
        assert_eq!(v["details"], json!(["d1"]));
    }

    #[test]
    fn attachment_order_and_download_url() {
        let p = json!({
            "id": 5,
            "fileName": "f.pdf",
            "fileSize": 100,
            "contentType": "application/pdf",
            "description": { "raw": "desc" },
            "createdAt": "c",
            "_links": {
                "author": { "title": "A" },
                "downloadLocation": { "href": "https://op/dl/5" }
            }
        });
        let v = attachment(&p);
        assert_eq!(
            keys(&v),
            vec![
                "id",
                "fileName",
                "fileSize",
                "contentType",
                "description",
                "author",
                "createdAt",
                "downloadUrl"
            ]
        );
        assert_eq!(v["downloadUrl"], json!("https://op/dl/5"));
    }

    #[test]
    fn relation_order_and_ids() {
        let p = json!({
            "id": 6,
            "type": "relates",
            "reverseType": "relates",
            "description": "r",
            "lockVersion": 0,
            "_links": {
                "from": { "href": "/api/v3/work_packages/1" },
                "to": { "href": "/api/v3/work_packages/2" }
            }
        });
        let v = relation(&p);
        assert_eq!(
            keys(&v),
            vec![
                "id",
                "type",
                "reverseType",
                "description",
                "from",
                "to",
                "lockVersion"
            ]
        );
        assert_eq!(v["from"], json!(1));
        assert_eq!(v["to"], json!(2));
    }
}
