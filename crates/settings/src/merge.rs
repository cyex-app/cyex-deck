use serde_json::Value;

/// JSON merge patch (RFC 7386): objects merge recursively, `null` removes a key,
/// anything else replaces.
pub(crate) fn merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(patch_map) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Default::default());
    }
    let target_map = target.as_object_mut().expect("just made an object");
    for (key, value) in patch_map {
        if value.is_null() {
            target_map.remove(key);
        } else {
            merge_patch(target_map.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_nested_and_removes_nulls() {
        let mut doc = json!({"a": 1, "b": {"c": 2, "d": 3}});
        merge_patch(&mut doc, &json!({"b": {"c": null, "e": 4}, "f": [1]}));
        assert_eq!(doc, json!({"a": 1, "b": {"d": 3, "e": 4}, "f": [1]}));
    }

    #[test]
    fn non_object_patch_replaces() {
        let mut doc = json!({"a": 1});
        merge_patch(&mut doc, &json!(5));
        assert_eq!(doc, json!(5));
    }
}
