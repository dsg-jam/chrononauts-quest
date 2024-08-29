use esp_idf_svc::{
    http::{
        server::{EspHttpConnection, Request},
        Method,
    },
    io::EspIOError,
    sys::EspError,
};
use std::net::Ipv4Addr;

use crate::http_server::ChrononautsHttpServer;

#[derive(Debug, thiserror::Error)]
pub enum CaptiveError {
    #[error(transparent)]
    Io(#[from] EspIOError),
    #[error(transparent)]
    EspError(#[from] EspError),
}

pub struct CaptivePortal;

impl CaptivePortal {
    pub fn attach(server: &mut ChrononautsHttpServer, addr: Ipv4Addr) -> Result<(), CaptiveError> {
        let redirect = move |request: Request<&'_ mut EspHttpConnection<'_>>| {
            request.into_response(302, None, &[("Location", &format!("http://{}", addr))])?;
            Ok(())
        };

        server.attach("/check_network_status.txt", Method::Get, redirect)?;
        server.attach("/connectivity-check.html", Method::Get, redirect)?;
        server.attach("/fwlink", Method::Get, redirect)?;
        server.attach("/gen_204", Method::Get, redirect)?;
        server.attach("/generate_204", Method::Get, redirect)?;
        server.attach("/hotspot-detect.html", Method::Get, redirect)?;
        server.attach("/library/test/success.html", Method::Get, redirect)?;
        server.attach("/ncsi.txt", Method::Get, redirect)?;

        Ok(())
    }
}
