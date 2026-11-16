use super::{
    call_dbus_method, find_interface_properties, is_dbus_service_available, managed_objects, property_value,
    set_dbus_property,
};
use zbus::zvariant::OwnedObjectPath;

const adapter: &str = "org.bluez.Adapter1";
const device_interface: &str = "org.bluez.Device1";
const service: &str = "org.bluez";

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum state {
    connected,
    paired,
    new,
}

#[derive(Clone)]
pub struct device {
    pub battery: Option<u8>,
    pub name: String,
    pub object_path: String,
    pub state: state,
}

#[derive(Clone)]
pub struct bluetooth {
    pub adapter_object_path: String,
    pub devices: Vec<device>,
    pub is_powered: bool,
}

fn device_state(connected: Option<bool>, paired: Option<bool>) -> state {
    match (connected, paired) {
        (Some(true), _) => state::connected,
        (_, Some(true)) => state::paired,
        _ => state::new,
    }
}

fn device_name(alias: Option<String>, address: Option<String>) -> Option<String> {
    alias.or(address)
}

fn order_devices(devices: &mut Vec<device>) {
    devices.sort_by_key(|device| device.state);
    devices.truncate(10);
}

pub async fn query_bluetooth_state() -> Option<bluetooth> {
    if !is_dbus_service_available(service).await {
        return None;
    }
    let objects = managed_objects(service).await?;
    let (object_path, adapter_properties) = objects
        .iter()
        .find_map(|(object_path, interfaces)| Some((object_path, find_interface_properties(interfaces, adapter)?)))?;
    let mut devices: Vec<device> = objects
        .iter()
        .filter_map(|(object_path, interfaces)| {
            let properties = find_interface_properties(interfaces, device_interface)?;
            let state = device_state(property_value(properties, "Connected"), property_value(properties, "Paired"));
            Some(device {
                battery: find_interface_properties(interfaces, "org.bluez.Battery1")
                    .and_then(|battery| property_value(battery, "Percentage")),
                name: device_name(property_value(properties, "Alias"), property_value(properties, "Address"))?,
                object_path: object_path.to_string(),
                state,
            })
        })
        .collect();
    order_devices(&mut devices);
    Some(bluetooth {
        adapter_object_path: object_path.to_string(),
        devices,
        is_powered: property_value(adapter_properties, "Powered").unwrap_or(false),
    })
}

pub async fn perform_bluetooth_action(method: &str, object_path: String) -> Result<(), String> {
    match method {
        "on" | "off" => set_dbus_property(service, &object_path, adapter, "Powered", (method == "on").into()).await,
        "scan" => {
            call_dbus_method::<_, ()>(service, &object_path, adapter, "StartDiscovery", &()).await?;
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            call_dbus_method(service, &object_path, adapter, "StopDiscovery", &()).await
        }
        "forget" => {
            let (parent, _) = object_path.rsplit_once('/').ok_or("invalid device")?;
            let object = OwnedObjectPath::try_from(object_path.as_str()).map_err(|error| error.to_string())?;
            call_dbus_method(service, parent, adapter, "RemoveDevice", &(object,)).await
        }
        method => call_dbus_method(service, &object_path, device_interface, method, &()).await,
    }
}
