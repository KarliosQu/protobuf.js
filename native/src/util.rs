use napi_derive::napi;
use serde_json::Value;
use std::collections::HashMap;

/// Convert snake_case string to camelCase (JavaScript utility)
/// Mimics: str.substring(0, 1) + str.substring(1).replace(/_([a-z])/g, (_, $1) => $1.toUpperCase())
#[napi]
pub fn camel_case(input: String) -> String {
    if input.is_empty() {
        return input;
    }
    
    let bytes = input.as_bytes();
    let mut result = String::new();
    
    // Keep first character as-is
    result.push(bytes[0] as char);
    
    // Process remaining characters
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'_' && i + 1 < bytes.len() {
            let next_char = bytes[i + 1] as char;
            if next_char.is_ascii_lowercase() {
                // Skip underscore and capitalize next lowercase letter
                result.push(next_char.to_ascii_uppercase());
                i += 2;
                continue;
            }
        }
        result.push(bytes[i] as char);
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
    is_reserved_str(name.as_str())
}

/// Internal function to check reserved keywords (takes string slice)
fn is_reserved_str(name: &str) -> bool {
    matches!(
        name,
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
        && !is_reserved_str(&prop);
    
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

/// Convert array of alternating keys and values to an object, omitting undefined and null
#[napi]
pub fn to_object(array: Vec<Value>) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    let mut i = 0;
    
    while i + 1 < array.len() {
        if let Some(key) = array[i].as_str() {
            // Only insert if value is not null or undefined
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
