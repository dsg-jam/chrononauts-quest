use esp_idf_svc::{
    http::{
        server::{EspHttpConnection, EspHttpServer, Request},
        Method,
    },
    io::EspIOError,
};
use std::net::Ipv4Addr;

pub struct CaptivePortal;

impl CaptivePortal {
    pub fn attach(server: &mut EspHttpServer, addr: Ipv4Addr) -> Result<(), EspIOError> {
        let redirect = move |request: Request<&'_ mut EspHttpConnection<'_>>| {
            request.into_response(302, None, &[("Location", &format!("http://{}", addr))])?;
            Ok::<(), EspIOError>(())
        };

        server.fn_handler("/check_network_status.txt", Method::Get, redirect)?;
        server.fn_handler("/connectivity-check.html", Method::Get, redirect)?;
        server.fn_handler("/fwlink", Method::Get, redirect)?;
        server.fn_handler("/gen_204", Method::Get, redirect)?;
        server.fn_handler("/generate_204", Method::Get, redirect)?;
        server.fn_handler("/hotspot-detect.html", Method::Get, redirect)?;
        server.fn_handler("/library/test/success.html", Method::Get, redirect)?;
        server.fn_handler("/ncsi.txt", Method::Get, redirect)?;

        Ok(())
    }
}
