pub mod audio;
pub mod bluetooth;
pub mod network;
pub mod power;

use std::collections::HashMap;
use zbus::Connection;
use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{DynamicDeserialize, DynamicType, OwnedValue, Value};

type properties = HashMap<String, OwnedValue>;

async fn bus() -> Result<Connection, String> {
    static connection: tokio::sync::OnceCell<Option<Connection>> = tokio::sync::OnceCell::const_new();
    connection.get_or_init(|| async { Connection::system().await.ok() }).await.clone().ok_or("no system bus".into())
}

async fn available(backend: &str, name: &str, service: &str) -> bool {
    if backend != "auto" && backend != name {
        return false;
    }
    let Ok(connection) = bus().await else { return false };
    let Ok(proxy) = zbus::fdo::DBusProxy::new(&connection).await else { return false };
    let Ok(service) = service.try_into() else { return false };
    proxy.name_has_owner(service).await.unwrap_or(false)
}

async fn objects(service: &str) -> Option<zbus::fdo::ManagedObjects> {
    let connection = bus().await.ok()?;
    let proxy = zbus::fdo::ObjectManagerProxy::builder(&connection).destination(service).ok()?.path("/").ok()?;
    proxy.build().await.ok()?.get_managed_objects().await.ok()
}

fn interface<'a>(interfaces: &'a HashMap<OwnedInterfaceName, properties>, name: &str) -> Option<&'a properties> {
    interfaces.iter().find(|(interface, _)| interface.as_str() == name).map(|(_, properties)| properties)
}

fn value<T: TryFrom<OwnedValue>>(properties: &properties, name: &str) -> Option<T> {
    T::try_from(properties.get(name)?.try_clone().ok()?).ok()
}

async fn call<B, R>(service: &str, path: &str, interface: &str, method: &str, body: &B) -> Result<R, String>
where
    B: serde::Serialize + DynamicType,
    R: for<'de> DynamicDeserialize<'de>,
{
    let connection = bus().await?;
    let reply = connection.call_method(Some(service), path, Some(interface), method, body).await;
    reply.map_err(|error| error.to_string())?.body().deserialize().map_err(|error| error.to_string())
}

async fn set(service: &str, path: &str, interface: &str, name: &str, value: Value<'_>) -> Result<(), String> {
    call(service, path, "org.freedesktop.DBus.Properties", "Set", &(interface, name, value)).await
}

pub async fn output(program: &str, args: &[&str]) -> String {
    let output = tokio::process::Command::new(program).args(args).output().await;
    output.map(|output| String::from_utf8_lossy(&output.stdout).into_owned()).unwrap_or_default()
}

pub fn which(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}
