pub mod wayland;

use std::collections::HashMap;
use std::{env, error::Error, os::unix::net::UnixStream};
use wayland::interface::{AnyEvent, Request, WlDisplayGetRegistryRequest, WlDisplaySyncRequest, WlRegistryBindRequest};
use wayland::object::{ObjectId, ObjectIdProvider};
use wayland::wire::{self, Message, MessageBuffer, MessageBuildError, MessageReader};

fn get_socket_path() -> Option<String> {
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").ok()?;
    let display_name = env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| String::from("wayland-0"));

    Some(format!("{xdg_runtime_dir}/{display_name}"))
}

#[derive(Clone, Default, Debug, PartialEq, Copy, Eq, PartialOrd, Ord, Hash)]
struct InterfaceDesc {
    object_name: ObjectId,
    version: u32,
}

fn get_registry(
    sock: &mut UnixStream,
    buf: &mut MessageBuffer,
) -> Result<HashMap<String, InterfaceDesc>, MessageBuildError> {
    WlDisplayGetRegistryRequest {
        registry: ObjectId::WL_REGISTRY,
    }
    .send(sock, buf)?;

    let mut registry = HashMap::<String, InterfaceDesc>::new();

    for _ in 0..53 {
        wire::read_message_into(sock, buf)?;

        let message = Message::from_u32_slice(buf.as_slice());
        let mut reader = MessageReader::new(&message);

        let object_name = ObjectId::new(reader.read_u32().unwrap());
        let interface_name = reader.read_str().unwrap();
        let version = reader.read_u32().unwrap();

        registry.insert(
            interface_name.to_owned(),
            InterfaceDesc {
                object_name,
                version,
            },
        );
    }

    Ok(registry)
}

fn main() -> Result<(), Box<dyn Error>> {
    let socket_path = get_socket_path().expect("failed to get wayland socket path");
    let mut sock = UnixStream::connect(socket_path)?;

    let mut id_provider = ObjectIdProvider::new();
    let mut buf = MessageBuffer::new();

    let registry = get_registry(&mut sock, &mut buf)?;

    WlDisplaySyncRequest {
        callback: ObjectId::WL_CALLBACK,
    }
    .send(&mut sock, &mut buf)?;

    let wl_compositor_name = registry["wl_compositor"];
    let wl_compositor_id = id_provider.next_id();

    WlRegistryBindRequest {
        name: wl_compositor_name.object_name,
        id: wl_compositor_id,
    }
    .send(&mut sock, &mut buf)?;

    loop {
        wire::read_message_into(&mut sock, &mut buf)?;
        let event = AnyEvent::from(buf.get_message());
        dbg!(event);
    }
}
