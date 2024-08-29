const ssid_select = document.getElementById("ssid");
var selected_ssid = "";

function request_scan() {
    var url = "/scan";
    var xhr = new XMLHttpRequest();
    xhr.open("POST", url, true);
    xhr.onreadystatechange = function() {
        if (xhr.readyState == 4 && xhr.status == 200) {
            console.log("Scan requested");
        } else {
            console.log("Scan request failed");
        }
    }
    xhr.send();
}

function fetch_ssids() {
    var url = "/ssids";
    var xhr = new XMLHttpRequest();
    xhr.open("GET", url, true);
    xhr.onreadystatechange = function() {
        if (xhr.readyState == 4 && xhr.status == 200) {
            console.log(selected_ssid);
            ssid_select.innerHTML = xhr.responseText;
            if (selected_ssid.value && selected_ssid.value.length > 0) {
                document.getElementById("ssid-sel-default").removeAttribute("selected");
            } else {
                ssid_select.value = selected_ssid;
            }
        }
    }
    xhr.send();
}

function ssid_selected() {
    selected_ssid = ssid_select.value;
}

document.addEventListener("DOMContentLoaded", function(event) { 
    selected_ssid = ssid_select.getAttribute("value");
    const _refresh_ssids_interval = setInterval(fetch_ssids, 2000);
});
