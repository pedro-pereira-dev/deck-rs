use super::{call, interface, objects, set, value};
use zbus::zvariant::OwnedObjectPath;

const device: &str = "net.connman.iwd.Device";
const network_interface: &str = "net.connman.iwd.Network";
const service: &str = "net.connman.iwd";
const station: &str = "net.connman.iwd.Station";

#[derive(Clone)]
pub struct network {
    pub active: bool,
    pub path: String,
    pub signal: usize,
    pub ssid: String,
}

#[derive(Clone)]
pub struct wifi {
    pub networks: Vec<network>,
    pub path: String,
    pub powered: bool,
}

pub async fn query(backend: &str) -> Option<wifi> {
    if !super::available(backend, "iwd", service).await {
        return None;
    }
    let objects = objects(service).await?;
    let (path, items) = objects.iter().find(|(_, items)| interface(items, device).is_some())?;
    let connected =
        interface(items, station).and_then(|properties| value::<OwnedObjectPath>(properties, "ConnectedNetwork"));
    let ordered: Vec<(OwnedObjectPath, i16)> = match interface(items, station) {
        Some(_) => call(service, path, station, "GetOrderedNetworks", &()).await.unwrap_or_default(),
        None => Vec::new(),
    };
    let networks = ordered
        .into_iter()
        .take(8)
        .map(|(network, strength)| network {
            active: connected.as_ref() == Some(&network),
            signal: signal(strength),
            ssid: objects
                .get(&network)
                .and_then(|items| interface(items, network_interface))
                .and_then(|properties| value(properties, "Name"))
                .unwrap_or_else(|| "unknown".into()),
            path: network.to_string(),
        })
        .collect();
    let powered = interface(items, device).and_then(|properties| value(properties, "Powered")).unwrap_or(false);
    Some(wifi { networks, path: path.to_string(), powered })
}

pub async fn action(method: &str, path: String) -> Result<(), String> {
    match method {
        "on" | "off" => set(service, &path, device, "Powered", (method == "on").into()).await,
        "Connect" => call(service, &path, network_interface, method, &()).await,
        method => call(service, &path, station, method, &()).await,
    }
}

fn signal(centi_dbm: i16) -> usize {
    ((centi_dbm as f32 / 100.0 + 100.0).clamp(0.0, 100.0) / 25.0).ceil() as usize
}

#[cfg(test)]
mod tests {
    #[test]
    fn converts_signal_to_bars() {
        assert_eq!(super::signal(-3000), 3);
        assert_eq!(super::signal(-6000), 2);
        assert_eq!(super::signal(-9500), 1);
    }
}
