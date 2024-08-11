use esp_idf_svc::{
    http::{
        server::{Configuration, EspHttpConnection, EspHttpServer, Request},
        Method,
    },
    io::{EspIOError, Write},
    sys::EspError,
    wifi::AccessPointInfo,
};
use std::sync::{mpsc::Sender, Arc, Condvar, Mutex};

use crate::{WifiCreds, WifiRunner};

pub fn setup_server(
    config: &Configuration,
    wifi_runner_tx: Sender<WifiRunner>,
    wifi_update_cond: Arc<(Mutex<bool>, Condvar)>,
    wifi_nets_store: Arc<Mutex<Vec<AccessPointInfo>>>,
) -> Result<EspHttpServer<'static>, EspError> {
    let mut server = EspHttpServer::new(config).expect("HTTP server init failed");

    let ssid_store = Arc::new(Mutex::new(String::new()));
    let wpa2_store = Arc::new(Mutex::new(String::new()));

    server.fn_handler("/style.css", Method::Get, handle_get_style)?;

    server.fn_handler("/script.js", Method::Get, handle_get_script)?;

    let ssid = ssid_store.clone();
    let wifi_nets = wifi_nets_store.clone();
    server.fn_handler("/", Method::Get, move |request| {
        handle_get_index(request, ssid.clone(), wifi_nets.clone())
    })?;

    let ssid = ssid_store.clone();
    let wpa2 = wpa2_store.clone();
    let wifi_runner = wifi_runner_tx.clone();
    server.fn_handler("/", Method::Post, move |request| {
        handle_post_index(request, ssid.clone(), wpa2.clone(), wifi_runner.clone())
    })?;

    let wifi_nets = wifi_nets_store.clone();
    let wifi_update_pair = Arc::clone(&wifi_update_cond);
    let wifi_runner = wifi_runner_tx.clone();
    server.fn_handler("/scan", Method::Get, move |request| {
        handle_get_scan(
            request,
            wifi_nets.clone(),
            Arc::clone(&wifi_update_pair),
            wifi_runner.clone(),
        )
    })?;

    Ok(server)
}

fn handle_get_style(request: Request<&mut EspHttpConnection>) -> Result<(), EspIOError> {
    request
        .into_response(200, None, &[("Content-Type", "text/css; charset=utf-8")])?
        .write_all(include_bytes!("web/style.css"))?;
    Ok(())
}

fn handle_get_script(request: Request<&mut EspHttpConnection>) -> Result<(), EspIOError> {
    request
        .into_response(
            200,
            None,
            &[("Content-Type", "text/javascript; charset=utf-8")],
        )?
        .write_all(include_bytes!("web/script.js"))?;
    Ok(())
}

fn handle_get_index(
    request: Request<&mut EspHttpConnection>,
    ssid: Arc<Mutex<String>>,
    wifi_nets: Arc<Mutex<Vec<AccessPointInfo>>>,
) -> Result<(), EspIOError> {
    let available_ssid_options = get_available_ssids(&wifi_nets.lock().unwrap());
    let page = format!(
        include_str!("web/index.html"),
        ssid = ssid.lock().unwrap(),
        wpa2 = "",
        available_ssids = available_ssid_options
    );
    request.into_ok_response()?.write_all(page.as_bytes())?;
    Ok(())
}

fn handle_post_index(
    mut request: Request<&mut EspHttpConnection>,
    ssid: Arc<Mutex<String>>,
    wpa2: Arc<Mutex<String>>,
    wifi_runner: Sender<WifiRunner>,
) -> Result<(), EspIOError> {
    let mut scratch = [0; 256];
    let len = request.read(&mut scratch)?;
    let req = std::str::from_utf8(&scratch[0..len]).unwrap();

    for part in req.split('&') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key {
            "ssid" => *ssid.lock().unwrap() = urlencoding::decode(value).unwrap().into_owned(),
            "wpa2" => *wpa2.lock().unwrap() = urlencoding::decode(value).unwrap().into_owned(),
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

    log::info!(
        "SSID: {}, WPA2: {}",
        ssid.lock().unwrap(),
        wpa2.lock().unwrap()
    );
    request.into_response(302, None, &[("Location", "/")])?;
    Ok(())
}

fn handle_get_scan(
    request: Request<&mut EspHttpConnection>,
    wifi_nets: Arc<Mutex<Vec<AccessPointInfo>>>,
    wifi_update_pair: Arc<(Mutex<bool>, Condvar)>,
    wifi_runner: Sender<WifiRunner>,
) -> Result<(), EspIOError> {
    let (lock, cvar) = &*wifi_update_pair;
    let mut wifi_update = lock.lock().unwrap();
    wifi_runner
        .send(WifiRunner::GetWifi)
        .expect("Failed to send Wi-Fi scan request");
    while !*wifi_update {
        // this will block this thread until the wifi_update is set to true
        wifi_update = cvar.wait(wifi_update).unwrap();
    }
    *wifi_update = false;
    let available_ssid_options = get_available_ssids(&wifi_nets.lock().unwrap());
    request
        .into_ok_response()?
        .write_all(available_ssid_options.as_bytes())?;
    Ok(())
}

fn get_available_ssids(ssids: &[AccessPointInfo]) -> String {
    let mut options = String::new();
    options.push_str(r#"<option id="ssid-sel-default" disabled hidden selected value="">-- Select SSID --</option>"#);
    ssids.iter().fold(options, |mut f, ssid| {
        f.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            ssid.ssid, ssid.ssid
        ));
        f
    })
}
