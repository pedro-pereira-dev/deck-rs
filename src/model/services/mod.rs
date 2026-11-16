pub mod audio;
pub mod bluetooth;
pub mod network;
pub mod power;

use std::collections::HashMap;
use zbus::Connection;
use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{DynamicDeserialize, DynamicType, OwnedValue, Value};

type dbus_properties = HashMap<String, OwnedValue>;

// reuse a system bus connection across module queries and actions.
async fn bus() -> Result<Connection, String> {
    static connection: tokio::sync::OnceCell<Option<Connection>> = tokio::sync::OnceCell::const_new();
    connection.get_or_init(|| async { Connection::system().await.ok() }).await.clone().ok_or("no system bus".into())
}

// check whether a system service is available on the bus.
async fn is_dbus_service_available(service: &str) -> bool {
    let Ok(connection) = bus().await else { return false };
    let Ok(proxy) = zbus::fdo::DBusProxy::new(&connection).await else { return false };
    let Ok(service) = service.try_into() else { return false };
    proxy.name_has_owner(service).await.unwrap_or(false)
}

// fetch all objects exposed by a dbus object manager service.
async fn managed_objects(service: &str) -> Option<zbus::fdo::ManagedObjects> {
    let connection = bus().await.ok()?;
    let proxy = zbus::fdo::ObjectManagerProxy::builder(&connection).destination(service).ok()?.path("/").ok()?;
    proxy.build().await.ok()?.get_managed_objects().await.ok()
}

// look up one named interface in an object's interface map.
fn find_interface_properties<'a>(
    interfaces: &'a HashMap<OwnedInterfaceName, dbus_properties>,
    name: &str,
) -> Option<&'a dbus_properties> {
    interfaces.iter().find(|(interface, _)| interface.as_str() == name).map(|(_, properties)| properties)
}

// clone and convert a dbus property into the requested rust type.
fn property_value<T: TryFrom<OwnedValue>>(properties: &dbus_properties, name: &str) -> Option<T> {
    T::try_from(properties.get(name)?.try_clone().ok()?).ok()
}

// call a dbus method and deserialize its response body.
async fn call_dbus_method<B, R>(
    service: &str,
    object_path: &str,
    interface: &str,
    method: &str,
    body: &B,
) -> Result<R, String>
where
    B: serde::Serialize + DynamicType,
    R: for<'de> DynamicDeserialize<'de>,
{
    let connection = bus().await?;
    let reply = connection.call_method(Some(service), object_path, Some(interface), method, body).await;
    reply.map_err(|error| error.to_string())?.body().deserialize().map_err(|error| error.to_string())
}

// set a named dbus property through the standard properties interface.
async fn set_dbus_property(
    service: &str,
    object_path: &str,
    interface: &str,
    name: &str,
    value: Value<'_>,
) -> Result<(), String> {
    call_dbus_method(service, object_path, "org.freedesktop.DBus.Properties", "Set", &(interface, name, value)).await
}

// run a command asynchronously and return stdout, or an empty string on failure.
pub async fn run_command_and_capture_stdout(program: &str, args: &[&str]) -> String {
    let output = tokio::process::Command::new(program).args(args).output().await;
    output.map(|output| String::from_utf8_lossy(&output.stdout).into_owned()).unwrap_or_default()
}

// check whether a program exists in one of the directories on path.
pub fn is_executable_on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| is_executable_in_directories(program, std::env::split_paths(&path)))
}

fn is_executable_in_directories(program: &str, paths: impl IntoIterator<Item = std::path::PathBuf>) -> bool {
    paths.into_iter().any(|dir| dir.join(program).is_file())
}

#[cfg(test)]
#[path = "services.test.rs"]
mod tests;
