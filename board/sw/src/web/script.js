const ssid_select = document.getElementById("ssid");
var selected_ssid = "";

function request_scan() {
    var url = "/scan";
    var xhr = new XMLHttpRequest();
    xhr.open("GET", url, true);
    xhr.onreadystatechange = function() {
        if (xhr.readyState == 4 && xhr.status == 200) {
            ssid_select.innerHTML = xhr.responseText;
            ssid_select.value = selected_ssid;
            if (ssid_select.value && ssid_select.value.length > 0) {
                document.getElementById("ssid-sel-default").removeAttribute("selected");
            }
        }
    }
    xhr.send();
}

document.addEventListener("DOMContentLoaded", function(event) { 
    selected_ssid = ssid_select.getAttribute("value");
});
