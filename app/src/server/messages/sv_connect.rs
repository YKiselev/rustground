use log::info;
use rg_net::{Connect, RejectionReason};

use crate::{
    error::AppError,
    server::{
        sv_clients::{ClientId, Clients},
        sv_guests::Guests, sv_security::ServerSecurity,
    },
};

pub(crate) fn on_connect(
    client_id: &ClientId,
    connect: &Connect,
    guests: &mut Guests,
    clients: &mut Clients,
    security: &ServerSecurity,
) -> Result<(), AppError> {
    let decoded = security.decode(connect.password)?;
    let guest = guests.get_or_create(*client_id);
    if !security.is_password_ok(&decoded) {
        info!("Wrong password from client {:?}!", client_id);
        guest.send_rejected(RejectionReason::Unauthorized);
    } else {
        guest.send_accepted();
        clients.add(*client_id, connect.name);
    }
    Ok(())
}
