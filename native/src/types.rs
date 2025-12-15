use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a protobuf field
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub id: u32,
    pub field_type: String,
    pub rule: Option<String>,
    pub extend: Option<String>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Represents a protobuf type (message)
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Type {
    pub name: String,
    pub fields: HashMap<String, Field>,
    pub oneofs: Option<HashMap<String, Vec<String>>>,
    pub extensions: Option<Vec<(u32, u32)>>,
    pub reserved: Option<Vec<String>>,
    pub nested: Option<HashMap<String, Type>>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

impl Type {
    pub fn new(name: String) -> Self {
        Type {
            name,
            fields: HashMap::new(),
            oneofs: None,
            extensions: None,
            reserved: None,
            nested: None,
            options: None,
        }
    }
}

/// Represents a protobuf service method
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    pub name: String,
    pub request_type: String,
    pub response_type: String,
    pub request_stream: bool,
    pub response_stream: bool,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Represents a protobuf service
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub methods: HashMap<String, Method>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

impl Service {
    pub fn new(name: String) -> Self {
        Service {
            name,
            methods: HashMap::new(),
            options: None,
        }
    }
}

/// Represents a protobuf enum value
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub id: i32,
}

/// Represents a protobuf enum
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    pub name: String,
    pub values: HashMap<String, i32>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            values: HashMap::new(),
            options: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_creation() {
        let t = Type::new("TestMessage".to_string());
        assert_eq!(t.name, "TestMessage");
        assert!(t.fields.is_empty());
    }

    #[test]
    fn test_service_creation() {
        let s = Service::new("TestService".to_string());
        assert_eq!(s.name, "TestService");
        assert!(s.methods.is_empty());
    }
}
