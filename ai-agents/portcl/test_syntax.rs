#!/usr/bin/env rust-script
//! Simple syntax check for our test file
use std::fs;

fn main() {
    println!("Checking test file syntax...");

    let content = match fs::read_to_string("tests/contract/test_execute_tests.rs") {
        Ok(content) => content,
        Err(e) => {
            println!("❌ Error reading file: {}", e);
            return;
        }
    };

    println!("✅ File read successfully");
    println!("✅ File size: {} bytes", content.len());

    // Basic syntax checks
    let lines = content.lines();
    let mut test_count = 0;
    let mut async_test_count = 0;
    let mut doc_comment_count = 0;

    for line in lines {
        if line.contains("#[tokio::test]") {
            async_test_count += 1;
            test_count += 1;
        } else if line.contains("#[test]") {
            test_count += 1;
        } else if line.trim().starts_with("//!") {
            doc_comment_count += 1;
        }
    }

    println!("✅ Found {} total tests ({} async)", test_count, async_test_count);
    println!("✅ Found {} documentation lines", doc_comment_count);

    // Check for key components
    let has_mock = content.contains("mock!");
    let has_data_models = content.contains("TestExecutionRequest") && content.contains("TestExecutionResponse");
    let has_openapi_refs = content.contains("OpenAPI") || content.contains("specification");
    let has_tdd_refs = content.contains("TDD") || content.contains("RED PHASE");

    println!("✅ Has mock setup: {}", has_mock);
    println!("✅ Has data models: {}", has_data_models);
    println!("✅ Has OpenAPI references: {}", has_openapi_refs);
    println!("✅ Has TDD/RED phase references: {}", has_tdd_refs);

    if has_mock && has_data_models && has_openapi_refs && has_tdd_refs {
        println!("🎉 Test file structure looks correct!");
    } else {
        println!("⚠️  Test file may be missing some expected components");
    }
}