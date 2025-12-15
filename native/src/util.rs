use napi_derive::napi;
use serde_json::Value;
use std::collections::HashMap;

/// Convert camelCase string to snake_case (JavaScript camelCase utility)
#[napi]
pub fn camel_case(input: String) -> String {
    let mut result = String::new();
    let mut first = true;
    
    for ch in input.chars() {
        if ch == '_' {
            first = false;
            continue;
        }
        
        if first {
            result.push(ch);
            first = false;
        } else if ch == '_' {
            // Skip underscore, next char will be uppercased
        } else {
            result.push(ch);
        }
    }
    
    // Convert snake_case to camelCase
    let bytes = input.as_bytes();
    result.clear();
    let mut i = 0;
    let mut capitalize_next = false;
    
    while i < bytes.len() {
        if bytes[i] == b'_' {
            capitalize_next = true;
        } else {
            if i == 0 {
                result.push(bytes[i] as char);
            } else if capitalize_next {
                result.push((bytes[i] as char).to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(bytes[i] as char);
            }
        }
        i += 1;
    }
    
    result
}

/// Capitalize first character of a string
#[napi]
pub fn uc_first(input: String) -> String {
    let mut chars = input.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Check if a name is a reserved JavaScript keyword
#[napi]
pub fn is_reserved(name: String) -> bool {
    matches!(
        name.as_str(),
        "do" | "if" | "in" | "for" | "let" | "new" | "try" | "var" | "case" | "else" | "enum" |
        "eval" | "false" | "null" | "this" | "true" | "void" | "with" | "break" | "catch" |
        "class" | "const" | "super" | "throw" | "while" | "yield" | "delete" | "export" |
        "import" | "public" | "return" | "static" | "switch" | "typeof" | "default" | "extends" |
        "finally" | "package" | "private" | "continue" | "debugger" | "function" | "arguments" |
        "interface" | "protected" | "implements" | "instanceof"
    )
}

/// Create a safe property accessor for the specified property name
#[napi]
pub fn safe_prop(prop: String) -> String {
    let is_safe = prop.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && !is_reserved(prop.clone());
    
    if is_safe && !prop.is_empty() {
        format!(".{}", prop)
    } else {
        format!("[\"{}\"]", prop.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// Convert object values to an array
#[napi]
pub fn to_array(obj: HashMap<String, Value>) -> Vec<Value> {
    obj.into_values().collect()
}

/// Convert array of alternating keys and values to an object, omitting undefined
#[napi]
pub fn to_object(array: Vec<Value>) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    let mut i = 0;
    
    while i + 1 < array.len() {
        if let Some(key) = array[i].as_str() {
            if !array[i + 1].is_null() {
                result.insert(key.to_string(), array[i + 1].clone());
            }
        }
        i += 2;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uc_first() {
        assert_eq!(uc_first("hello".to_string()), "Hello");
        assert_eq!(uc_first("".to_string()), "");
    }

    #[test]
    fn test_is_reserved() {
        assert!(is_reserved("if".to_string()));
        assert!(is_reserved("class".to_string()));
        assert!(!is_reserved("myVar".to_string()));
    }

    #[test]
    fn test_safe_prop() {
        assert_eq!(safe_prop("myProp".to_string()), ".myProp");
        assert_eq!(safe_prop("class".to_string()), "[\"class\"]");
    }
}
