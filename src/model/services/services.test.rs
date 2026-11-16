use std::collections::HashMap;
use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{OwnedValue, Value};

#[test]
fn finds_interface_by_exact_name() {
    let name = OwnedInterfaceName::try_from("org.example.Test").unwrap();
    let properties = HashMap::new();
    let interfaces = HashMap::from([(name, properties)]);
    assert!(super::find_interface_properties(&interfaces, "org.example.Test").is_some());
    assert!(super::find_interface_properties(&interfaces, "org.example.Other").is_none());
}

#[test]
fn converts_known_properties_and_handles_missing_or_wrong_types() {
    let properties = HashMap::from([("Powered".into(), OwnedValue::try_from(Value::from(true)).unwrap())]);
    assert_eq!(super::property_value::<bool>(&properties, "Powered"), Some(true));
    assert_eq!(super::property_value::<String>(&properties, "Powered"), None);
    assert_eq!(super::property_value::<bool>(&properties, "Missing"), None);
}

#[test]
fn path_lookup_checks_each_candidate_directory() {
    let root = std::env::temp_dir().join(format!("deck-rs-path-test-{}", std::process::id()));
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("deck-rs-test-bin"), "first").unwrap();

    assert!(super::is_executable_in_directories("deck-rs-test-bin", [second.clone(), first.clone()]));
    assert!(!super::is_executable_in_directories("deck-rs-missing-bin", [first, second]));
    std::fs::remove_dir_all(root).unwrap();
}
