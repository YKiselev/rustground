
use rg_net::Ping;

use crate::server::{sv_clients::ClientId, sv_guests::Guests};

pub(crate) fn on_ping(client_id: &ClientId, ping: &Ping, guests: &mut Guests) {
    let guest = guests.get_or_create(*client_id);
    guest.send_pong(&ping);
}
