use esp_idf_svc::{
    http::{
        server::{EspHttpConnection, EspHttpServer},
        Method,
    },
    io::{EspIOError, Write},
    sys::EspError,
};
use std::{
    str::Utf8Error,
    sync::{mpsc::Sender, Arc, Mutex},
};

use crate::{
    wifi::{WifiCreds, WifiRunner},
    ChrononautsSSIDs,
};

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(transparent)]
    Io(#[from] EspIOError),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
    #[error(transparent)]
    EspError(#[from] EspError),
}

type SharedSsid = Arc<Mutex<heapless::String<32>>>;
type SharedWpa2 = Arc<Mutex<heapless::String<64>>>;
type Request<'a, 'b> = esp_idf_svc::http::server::Request<&'a mut EspHttpConnection<'b>>;

pub struct ChrononautsHttpServer {
    server: EspHttpServer<'static>,
    wifi_runner_tx: Sender<WifiRunner>,
    wifi_ssids: ChrononautsSSIDs,
    ssid: SharedSsid,
    wpa2: SharedWpa2,
}

impl ChrononautsHttpServer {
    pub fn new(wifi_runner_tx: Sender<WifiRunner>, wifi_ssids: ChrononautsSSIDs) -> Self {
        let server = EspHttpServer::new(&Default::default()).expect("HTTP server init failed");
        let ssid = SharedSsid::default();
        let wpa2 = SharedWpa2::default();
        Self {
            server,
            wifi_runner_tx,
            wifi_ssids,
            ssid,
            wpa2,
        }
    }

    pub fn setup(&mut self) -> Result<(), ServerError> {
        self.handle_get_style()?;
        self.handle_get_script()?;
        self.handle_get_index()?;
        self.handle_post_index()?;
        self.handle_get_ssids()?;
        self.handle_post_scan()?;
        Ok(())
    }

    fn handle_get_style(&mut self) -> Result<(), ServerError> {
        let handler = move |request: Request| {
            request
                .into_response(200, None, &[("Content-Type", "text/css; charset=utf-8")])?
                .write_all(include_bytes!("web/style.css"))?;
            Ok(())
        };
        self.attach("/style.css", Method::Get, handler)?;
        Ok(())
    }

    fn handle_get_script(&mut self) -> Result<(), ServerError> {
        let handler = move |request: Request| {
            request
                .into_response(
                    200,
                    None,
                    &[("Content-Type", "text/javascript; charset=utf-8")],
                )?
                .write_all(include_bytes!("web/script.js"))?;
            Ok(())
        };
        self.attach("/script.js", Method::Get, handler)?;
        Ok(())
    }

    fn handle_get_index(&mut self) -> Result<(), ServerError> {
        let ssid = self.ssid.clone();
        let wifi_ssids = self.wifi_ssids.clone();
        let handler = move |request: Request| {
            let available_ssid_options = get_available_ssids(&wifi_ssids)?;
            let page = format!(
                include_str!("web/index.html"),
                ssid = ssid.lock().unwrap(),
                wpa2 = "",
                available_ssids = available_ssid_options
            );
            request.into_ok_response()?.write_all(page.as_bytes())?;
            Ok(())
        };
        self.attach("/", Method::Get, handler)?;
        Ok(())
    }

    fn handle_post_index(&mut self) -> Result<(), ServerError> {
        let ssid = self.ssid.clone();
        let wpa2 = self.wpa2.clone();
        let wifi_runner = self.wifi_runner_tx.clone();
        let handler = move |mut request: Request| {
            let mut scratch = [0; 256];
            let len = request.read(&mut scratch)?;
            let req = std::str::from_utf8(&scratch[0..len])?;

            for part in req.split('&') {
                let Some((key, value)) = part.split_once('=') else {
                    continue;
                };
                match key {
                    "ssid" => {
                        *ssid.lock().unwrap() = urlencoding::decode(value)
                            .map_err(|err| err.utf8_error())?
                            .parse()
                            .unwrap()
                    }
                    "wpa2" => {
                        *wpa2.lock().unwrap() = urlencoding::decode(value)
                            .map_err(|err| err.utf8_error())?
                            .parse()
                            .unwrap()
                    }
                    _ => (),
                }
            }

            // Let's configure the Wi-Fi with the provided SSID and WPA2 password
            wifi_runner
                .send(WifiRunner::ChangeWifi(WifiCreds {
                    ssid: ssid.lock().unwrap().clone(),
                    wpa2: wpa2.lock().unwrap().clone(),
                }))
                .expect("Failed to send Wi-Fi credentials");
            request.into_response(302, None, &[("Location", "/")])?;
            Ok(())
        };
        self.attach("/", Method::Post, handler)?;
        Ok(())
    }

    fn handle_get_ssids(&mut self) -> Result<(), ServerError> {
        let wifi_ssids = self.wifi_ssids.clone();
        let handler = move |request: Request| {
            let ssids = get_available_ssids(&wifi_ssids)?;
            request.into_ok_response()?.write_all(ssids.as_bytes())?;
            Ok(())
        };
        self.attach("/ssids", Method::Get, handler)?;
        Ok(())
    }

    fn handle_post_scan(&mut self) -> Result<(), ServerError> {
        let wifi_runner = self.wifi_runner_tx.clone();
        let handler = move |request: Request| {
            wifi_runner
                .send(WifiRunner::GetWifi)
                .expect("Failed to send Wi-Fi scan request");
            request.into_ok_response()?;
            Ok(())
        };
        self.attach("/scan", Method::Post, handler)?;
        Ok(())
    }

    pub fn attach<F>(&mut self, uri: &str, method: Method, handler: F) -> Result<(), EspIOError>
    where
        F: for<'r> Fn(Request) -> Result<(), ServerError> + Send + 'static,
    {
        self.server.fn_handler(uri, method, handler)?;
        Ok(())
    }
}

fn get_available_ssids(ssids: &ChrononautsSSIDs) -> Result<String, ServerError> {
    let ssids = ssids.lock().unwrap();
    let mut options = String::new();
    options.push_str(r#"<option id="ssid-sel-default" disabled hidden selected value="">-- Select SSID --</option>"#);
    Ok(ssids.iter().fold(options, |mut f, ssid| {
        f.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            ssid.ssid, ssid.ssid
        ));
        f
    }))
}
