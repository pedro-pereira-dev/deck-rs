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
