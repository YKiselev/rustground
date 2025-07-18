use rg_net::{Hello, RejectionReason, PROTOCOL_VERSION};

use crate::server::{sv_clients::ClientId, sv_guests::Guests};

pub(crate) fn on_hello(client_id: &ClientId, hello: &Hello, guests: &mut Guests, key: &[u8]) {
    let guest = guests.get_or_create(*client_id);
    if hello.version.0 <= PROTOCOL_VERSION.0 && hello.version.1 <= PROTOCOL_VERSION.1 {
        guest.send_server_info(key);
    } else {
        guest.send_rejected(RejectionReason::UnsupportedVersion);
    }
}
