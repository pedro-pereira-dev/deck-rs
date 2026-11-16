use std::collections::HashMap;

use zbus::Connection;
use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{DynamicDeserialize, DynamicType, OwnedValue, Value};

type dbus_properties = HashMap<String, OwnedValue>;

pub(super) async fn call_dbus_method<B, R>(
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

pub(super) async fn set_dbus_property(
    service: &str,
    object_path: &str,
    interface: &str,
    name: &str,
    value: Value<'_>,
) -> Result<(), String> {
    call_dbus_method(service, object_path, "org.freedesktop.DBus.Properties", "Set", &(interface, name, value)).await
}

pub(super) async fn is_dbus_service_available(service: &str) -> bool {
    let Ok(connection) = bus().await else { return false };
    let Ok(proxy) = zbus::fdo::DBusProxy::new(&connection).await else { return false };
    let Ok(service) = service.try_into() else { return false };
    proxy.name_has_owner(service).await.unwrap_or(false)
}

pub(super) async fn managed_objects(service: &str) -> Option<zbus::fdo::ManagedObjects> {
    let connection = bus().await.ok()?;
    let proxy = zbus::fdo::ObjectManagerProxy::builder(&connection).destination(service).ok()?.path("/").ok()?;
    proxy.build().await.ok()?.get_managed_objects().await.ok()
}

pub(super) fn find_interface_properties<'a>(
    interfaces: &'a HashMap<OwnedInterfaceName, dbus_properties>,
    name: &str,
) -> Option<&'a dbus_properties> {
    interfaces.iter().find(|(interface, _)| interface.as_str() == name).map(|(_, properties)| properties)
}

pub(super) fn property_value<T: TryFrom<OwnedValue>>(properties: &dbus_properties, name: &str) -> Option<T> {
    T::try_from(properties.get(name)?.try_clone().ok()?).ok()
}

async fn bus() -> Result<Connection, String> {
    static connection: tokio::sync::OnceCell<Option<Connection>> = tokio::sync::OnceCell::const_new();
    connection.get_or_init(|| async { Connection::system().await.ok() }).await.clone().ok_or("no system bus".into())
}
