use super::{
    call_dbus_method, find_interface_properties, is_dbus_service_available, managed_objects, property_value,
    set_dbus_property,
};
use zbus::zvariant::OwnedObjectPath;

const device: &str = "net.connman.iwd.Device";
const network_interface: &str = "net.connman.iwd.Network";
const service: &str = "net.connman.iwd";
const station: &str = "net.connman.iwd.Station";

#[derive(Clone)]
pub struct network {
    pub is_active: bool,
    pub object_path: String,
    pub signal: usize,
    pub ssid: String,
}

#[derive(Clone)]
pub struct wifi {
    pub adapter_object_path: String,
    pub is_powered: bool,
    pub networks: Vec<network>,
}

// convert signal strength in centi-dbm into a small number of display bars.
fn signal(centi_dbm: i16) -> usize {
    ((centi_dbm as f32 / 100.0 + 100.0).clamp(0.0, 100.0) / 25.0).ceil() as usize
}

fn networks(
    entries: Vec<(OwnedObjectPath, i16)>,
    connected: Option<OwnedObjectPath>,
    objects: &zbus::fdo::ManagedObjects,
) -> Vec<network> {
    entries
        .into_iter()
        .take(8)
        .map(|(network, strength)| network {
            is_active: connected.as_ref() == Some(&network),
            object_path: network.to_string(),
            signal: signal(strength),
            ssid: objects
                .get(&network)
                .and_then(|interfaces| find_interface_properties(interfaces, network_interface))
                .and_then(|properties| property_value(properties, "Name"))
                .unwrap_or_else(|| "unknown".into()),
        })
        .collect()
}

pub async fn query_wifi_state() -> Option<wifi> {
    if !is_dbus_service_available(service).await {
        return None;
    }
    let objects = managed_objects(service).await?;
    let (object_path, interfaces) =
        objects.iter().find(|(_, interfaces)| find_interface_properties(interfaces, device).is_some())?;
    let station_properties = find_interface_properties(interfaces, station);
    let connected =
        station_properties.and_then(|properties| property_value::<OwnedObjectPath>(properties, "ConnectedNetwork"));
    let ordered: Vec<(OwnedObjectPath, i16)> = match station_properties {
        Some(_) => call_dbus_method(service, object_path, station, "GetOrderedNetworks", &()).await.unwrap_or_default(),
        None => Vec::new(),
    };
    let networks = networks(ordered, connected, &objects);
    let powered = find_interface_properties(interfaces, device)
        .and_then(|properties| property_value(properties, "Powered"))
        .unwrap_or(false);
    Some(wifi { adapter_object_path: object_path.to_string(), is_powered: powered, networks })
}

pub async fn perform_wifi_action(method: &str, object_path: String) -> Result<(), String> {
    match method {
        "on" | "off" => set_dbus_property(service, &object_path, device, "Powered", (method == "on").into()).await,
        "Connect" => call_dbus_method(service, &object_path, network_interface, method, &()).await,
        method => call_dbus_method(service, &object_path, station, method, &()).await,
    }
}
