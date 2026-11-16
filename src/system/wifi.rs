use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

use super::{
    call_dbus_method, find_interface_properties, is_dbus_service_available, managed_objects, property_value,
    set_dbus_property,
};
use zbus::zvariant::OwnedObjectPath;

const device: &str = "net.connman.iwd.Device";
const network_interface: &str = "net.connman.iwd.Network";
const agent_manager_interface: &str = "net.connman.iwd.AgentManager";
const service: &str = "net.connman.iwd";
const station: &str = "net.connman.iwd.Station";
const agent_path: &str = "/com/deck_rs/IwdAgent";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum agent_request {
    pending,
    requested,
    submitted,
    cancelled,
}

#[derive(Clone)]
pub struct credential_agent {
    state: Arc<AtomicU8>,
    passphrase: Arc<tokio::sync::Mutex<Option<String>>>,
    requested: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}

impl credential_agent {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(0)),
            passphrase: Arc::new(tokio::sync::Mutex::new(None)),
            requested: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    async fn await_passphrase(&self) -> Option<String> {
        self.requested.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        loop {
            let notified = self.notify.notified();
            let state = self.request_state();
            match state {
                agent_request::submitted => return self.passphrase.lock().await.take(),
                agent_request::cancelled => return None,
                agent_request::pending | agent_request::requested => notified.await,
            }
        }
    }

    pub async fn submit(self, passphrase: String) {
        *self.passphrase.lock().await = Some(passphrase);
        self.state.store(2, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub async fn decline(self) {
        self.state.store(3, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn request_state(&self) -> agent_request {
        match self.state.load(Ordering::SeqCst) {
            2 => agent_request::submitted,
            3 => agent_request::cancelled,
            0 if self.requested.load(Ordering::SeqCst) => agent_request::requested,
            _ => agent_request::pending,
        }
    }
}

#[zbus::interface(name = "net.connman.iwd.Agent")]
impl credential_agent {
    #[zbus(name = "Release")]
    async fn release(&self) {}

    async fn request_passphrase(&self, network: OwnedObjectPath) -> zbus::fdo::Result<String> {
        let _ = network;
        self.await_passphrase().await.ok_or_else(|| zbus::fdo::Error::Failed("request cancelled".into()))
    }

    #[zbus(name = "Cancel")]
    async fn cancel(&self, reason: String) {
        let _ = reason;
        self.clone().decline().await;
    }
}

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
    pub station_object_path: Option<String>,
    pub status: Option<String>,
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
        return Some(wifi {
            adapter_object_path: String::new(),
            is_powered: false,
            networks: Vec::new(),
            station_object_path: None,
            status: Some("iwd is not running or is not accessible on the system bus".into()),
        });
    }
    let Some(objects) = managed_objects(service).await else {
        return Some(wifi {
            adapter_object_path: String::new(),
            is_powered: false,
            networks: Vec::new(),
            station_object_path: None,
            status: Some("could not read devices from iwd".into()),
        });
    };
    let adapter = objects
        .iter()
        .find(|(_, interfaces)| {
            find_interface_properties(interfaces, device).is_some()
                && find_interface_properties(interfaces, station).is_some()
        })
        .or_else(|| objects.iter().find(|(_, interfaces)| find_interface_properties(interfaces, device).is_some()));
    let Some((object_path, interfaces)) = adapter else {
        return Some(wifi {
            adapter_object_path: String::new(),
            is_powered: false,
            networks: Vec::new(),
            station_object_path: None,
            status: Some("iwd has no Wi-Fi device".into()),
        });
    };
    let station_path = find_interface_properties(interfaces, station).map(|_| object_path.as_str());
    let station_properties = find_interface_properties(interfaces, station);
    let connected =
        station_properties.and_then(|properties| property_value::<OwnedObjectPath>(properties, "ConnectedNetwork"));
    let (ordered, mut status): (Vec<(OwnedObjectPath, i16)>, Option<String>) = match station_path {
        Some(path) => match call_dbus_method(service, path, station, "GetOrderedNetworks", &()).await {
            Ok(networks) => (networks, None),
            Err(error) => (Vec::new(), Some(format!("could not read iwd networks: {error}"))),
        },
        None => (Vec::new(), Some("iwd has no station interface for its Wi-Fi device".into())),
    };
    let networks = networks(ordered, connected, &objects);
    let powered =
        find_interface_properties(interfaces, device).and_then(|properties| property_value(properties, "Powered"));
    if powered.is_none() {
        status.get_or_insert_with(|| "could not read Wi-Fi power state from iwd".into());
    }
    Some(wifi {
        adapter_object_path: object_path.to_string(),
        is_powered: powered.unwrap_or(false),
        networks,
        station_object_path: station_path.map(str::to_owned),
        status,
    })
}

pub async fn perform_wifi_action(method: &str, object_path: String) -> Result<(), String> {
    match method {
        "on" | "off" => set_dbus_property(service, &object_path, device, "Powered", (method == "on").into()).await,
        "Connect" => call_dbus_method(service, &object_path, network_interface, method, &()).await,
        method => call_dbus_method(service, &object_path, station, method, &()).await,
    }
}

pub async fn connect_with_agent(network_path: String, agent: credential_agent) -> Result<(), String> {
    let connection = super::dbus::system_bus().await?;
    let object_server = connection.object_server();
    if !object_server.at(agent_path, agent.clone()).await.map_err(|error| error.to_string())? {
        return Err("an iwd credential agent is already registered in this app".into());
    }
    let path = OwnedObjectPath::try_from(agent_path).map_err(|error| error.to_string())?;
    let register = call_dbus_method::<_, ()>(
        service,
        "/net/connman/iwd",
        agent_manager_interface,
        "RegisterAgent",
        &(path.clone(),),
    )
    .await;
    if let Err(error) = register {
        let _ = object_server.remove::<credential_agent, _>(agent_path).await;
        return Err(error);
    }
    let result = call_dbus_method::<_, ()>(service, &network_path, network_interface, "Connect", &()).await;
    let unregister =
        call_dbus_method::<_, ()>(service, "/net/connman/iwd", agent_manager_interface, "UnregisterAgent", &(path,))
            .await;
    let _ = object_server.remove::<credential_agent, _>(agent_path).await;
    result?;
    unregister
}
