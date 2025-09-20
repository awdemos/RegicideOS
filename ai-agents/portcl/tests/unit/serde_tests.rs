//! Unit tests for serde utilities module

use portcl::utils::serde_utils::{
    to_json_string, to_json_bytes, from_json_string, from_json_bytes,
    to_toml_string, from_toml_string
};
use portcl::error::PortCLError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestStruct {
    name: String,
    value: i32,
    active: bool,
}

impl Default for TestStruct {
    fn default() -> Self {
        Self {
            name: "test".to_string(),
            value: 42,
            active: true,
        }
    }
}

#[test]
fn test_to_json_string_success() {
    let test_data = TestStruct::default();
    let result = to_json_string(&test_data);

    assert!(result.is_ok());
    let json_str = result.unwrap();
    assert!(json_str.contains("\"name\": \"test\""));
    assert!(json_str.contains("\"value\": 42"));
    assert!(json_str.contains("\"active\": true"));
}

#[test]
fn test_to_json_bytes_success() {
    let test_data = TestStruct::default();
    let result = to_json_bytes(&test_data);

    assert!(result.is_ok());
    let json_bytes = result.unwrap();
    let json_str = String::from_utf8(json_bytes).unwrap();
    assert!(json_str.contains("\"name\": \"test\""));
}

#[test]
fn test_from_json_string_success() {
    let json_str = r#"{"name": "test", "value": 42, "active": true}"#;
    let result: Result<TestStruct, PortCLError> = from_json_string(json_str);

    assert!(result.is_ok());
    let test_data = result.unwrap();
    assert_eq!(test_data.name, "test");
    assert_eq!(test_data.value, 42);
    assert!(test_data.active);
}

#[test]
fn test_from_json_bytes_success() {
    let json_bytes = br#"{"name": "test", "value": 42, "active": true}"#;
    let result: Result<TestStruct, PortCLError> = from_json_bytes(json_bytes);

    assert!(result.is_ok());
    let test_data = result.unwrap();
    assert_eq!(test_data.name, "test");
    assert_eq!(test_data.value, 42);
    assert!(test_data.active);
}

#[test]
fn test_to_toml_string_success() {
    let test_data = TestStruct::default();
    let result = to_toml_string(&test_data);

    assert!(result.is_ok());
    let toml_str = result.unwrap();
    assert!(toml_str.contains("name = \"test\""));
    assert!(toml_str.contains("value = 42"));
    assert!(toml_str.contains("active = true"));
}

#[test]
fn test_from_toml_string_success() {
    let toml_str = r#"
name = "test"
value = 42
active = true
"#;
    let result: Result<TestStruct, PortCLError> = from_toml_string(toml_str);

    assert!(result.is_ok());
    let test_data = result.unwrap();
    assert_eq!(test_data.name, "test");
    assert_eq!(test_data.value, 42);
    assert!(test_data.active);
}

#[test]
fn test_json_round_trip() {
    let original_data = TestStruct::default();

    // Serialize to JSON string
    let json_str = to_json_string(&original_data).unwrap();

    // Deserialize back to struct
    let deserialized_data: TestStruct = from_json_string(&json_str).unwrap();

    assert_eq!(original_data, deserialized_data);
}

#[test]
fn test_json_bytes_round_trip() {
    let original_data = TestStruct::default();

    // Serialize to JSON bytes
    let json_bytes = to_json_bytes(&original_data).unwrap();

    // Deserialize back to struct
    let deserialized_data: TestStruct = from_json_bytes(&json_bytes).unwrap();

    assert_eq!(original_data, deserialized_data);
}

#[test]
fn test_toml_round_trip() {
    let original_data = TestStruct::default();

    // Serialize to TOML string
    let toml_str = to_toml_string(&original_data).unwrap();

    // Deserialize back to struct
    let deserialized_data: TestStruct = from_toml_string(&toml_str).unwrap();

    assert_eq!(original_data, deserialized_data);
}

#[test]
fn test_invalid_json_string() {
    let invalid_json = r#"{"name": "test", "value": 42, "active":}"#;
    let result: Result<TestStruct, PortCLError> = from_json_string(invalid_json);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PortCLError::Json(_)));
}

#[test]
fn test_invalid_json_bytes() {
    let invalid_json = br#"{"name": "test", "value": 42, "active":}"#;
    let result: Result<TestStruct, PortCLError> = from_json_bytes(invalid_json);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PortCLError::Json(_)));
}

#[test]
fn test_invalid_toml_string() {
    let invalid_toml = r#"name = "test"
value = 42
active =
"#;
    let result: Result<TestStruct, PortCLError> = from_toml_string(invalid_toml);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PortCLError::TomlDeserialize(_)));
}

#[test]
fn test_complex_structures() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ComplexStruct {
        nested: TestStruct,
        list: Vec<String>,
        optional: Option<i32>,
        map: std::collections::HashMap<String, f64>,
    }

    let mut map = std::collections::HashMap::new();
    map.insert("pi".to_string(), 3.14159);
    map.insert("e".to_string(), 2.71828);

    let complex_data = ComplexStruct {
        nested: TestStruct::default(),
        list: vec!["item1".to_string(), "item2".to_string()],
        optional: Some(100),
        map,
    };

    // Test JSON serialization/deserialization
    let json_str = to_json_string(&complex_data).unwrap();
    let deserialized: ComplexStruct = from_json_string(&json_str).unwrap();
    assert_eq!(complex_data, deserialized);

    // Test TOML serialization/deserialization
    let toml_str = to_toml_string(&complex_data).unwrap();
    let deserialized_toml: ComplexStruct = from_toml_string(&toml_str).unwrap();
    assert_eq!(complex_data, deserialized_toml);
}

#[test]
fn test_enum_serialization() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestEnum {
        Variant1,
        Variant2(String),
        Variant3 { value: i32 },
    }

    let enum_value = TestEnum::Variant2("test".to_string());

    // Test JSON
    let json_str = to_json_string(&enum_value).unwrap();
    let deserialized: TestEnum = from_json_string(&json_str).unwrap();
    assert_eq!(enum_value, deserialized);

    // Test TOML
    let toml_str = to_toml_string(&enum_value).unwrap();
    let deserialized_toml: TestEnum = from_toml_string(&toml_str).unwrap();
    assert_eq!(enum_value, deserialized_toml);
}

#[test]
fn test_empty_struct() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct EmptyStruct {}

    let empty = EmptyStruct {};

    // Test JSON
    let json_str = to_json_string(&empty).unwrap();
    let deserialized: EmptyStruct = from_json_string(&json_str).unwrap();
    assert_eq!(empty, deserialized);

    // Test TOML
    let toml_str = to_toml_string(&empty).unwrap();
    let deserialized_toml: EmptyStruct = from_toml_string(&toml_str).unwrap();
    assert_eq!(empty, deserialized_toml);
}

#[test]
fn test_special_characters() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct SpecialChars {
        unicode: String,
        newlines: String,
        quotes: String,
    }

    let special = SpecialChars {
        unicode: "café résumé".to_string(),
        newlines: "line1\nline2\nline3".to_string(),
        quotes: "contains \"quotes\" and 'apostrophes'".to_string(),
    };

    // Test JSON (should handle special characters properly)
    let json_str = to_json_string(&special).unwrap();
    let deserialized: SpecialChars = from_json_string(&json_str).unwrap();
    assert_eq!(special, deserialized);

    // Test TOML (should handle special characters properly)
    let toml_str = to_toml_string(&special).unwrap();
    let deserialized_toml: SpecialChars = from_toml_string(&toml_str).unwrap();
    assert_eq!(special, deserialized_toml);
}

#[test]
fn test_large_data() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct LargeData {
        items: Vec<i32>,
        text: String,
    }

    let large_data = LargeData {
        items: (0..1000).collect(),
        text: "a".repeat(10000),
    };

    // Test JSON
    let json_str = to_json_string(&large_data).unwrap();
    let deserialized: LargeData = from_json_string(&json_str).unwrap();
    assert_eq!(large_data, deserialized);

    // Test TOML
    let toml_str = to_toml_string(&large_data).unwrap();
    let deserialized_toml: LargeData = from_toml_string(&toml_str).unwrap();
    assert_eq!(large_data, deserialized_toml);
}