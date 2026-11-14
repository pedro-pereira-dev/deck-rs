use super::{call, interface, objects, set, value};
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
    pub path: String,
    pub state: state,
}

#[derive(Clone)]
pub struct bluetooth {
    pub adapter: String,
    pub devices: Vec<device>,
    pub powered: bool,
}

pub async fn query(backend: &str) -> Option<bluetooth> {
    if !super::available(backend, "bluez", service).await {
        return None;
    }
    let objects = objects(service).await?;
    let (path, properties) = objects.iter().find_map(|(path, items)| Some((path, interface(items, adapter)?)))?;
    let mut devices: Vec<device> = objects
        .iter()
        .filter_map(|(path, items)| {
            let properties = interface(items, device_interface)?;
            let state = match (value(properties, "Connected"), value(properties, "Paired")) {
                (Some(true), _) => state::connected,
                (_, Some(true)) => state::paired,
                _ => state::new,
            };
            Some(device {
                battery: interface(items, "org.bluez.Battery1").and_then(|battery| value(battery, "Percentage")),
                name: value(properties, "Alias").or_else(|| value(properties, "Address"))?,
                path: path.to_string(),
                state,
            })
        })
        .collect();
    devices.sort_by_key(|device| device.state);
    devices.truncate(10);
    Some(bluetooth { adapter: path.to_string(), devices, powered: value(properties, "Powered").unwrap_or(false) })
}

pub async fn action(method: &str, path: String) -> Result<(), String> {
    match method {
        "on" | "off" => set(service, &path, adapter, "Powered", (method == "on").into()).await,
        "scan" => {
            call::<_, ()>(service, &path, adapter, "StartDiscovery", &()).await?;
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            call(service, &path, adapter, "StopDiscovery", &()).await
        }
        "forget" => {
            let (parent, _) = path.rsplit_once('/').ok_or("invalid device")?;
            let object = OwnedObjectPath::try_from(path.as_str()).map_err(|error| error.to_string())?;
            call(service, parent, adapter, "RemoveDevice", &(object,)).await
        }
        method => call(service, &path, device_interface, method, &()).await,
    }
}
