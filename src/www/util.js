function urlEncode(object) {
    var i,value, params = [];

    for (i in object) {
	if (object.hasOwnProperty(i)) {
	    value = object[i];
	    if (value === undefined) value = '';
	    params.push(encodeURIComponent(i) + '=' + encodeURIComponent(String(value)));
	}
    }

    return params.join('&');
}

var msgtimeout;
var severities = {
    normal:  1,
    warning: 2,
    error:   3,
};

function showMsg(message, timeout, severity) {
    var status_bar = document.getElementById('status_bar');
    clearTimeout(msgtimeout);

    status_bar.classList.remove('normal');
    status_bar.classList.remove('warning');
    status_bar.classList.remove('error');

    status_bar.textContent = message;

    severity = severity || severities.normal;

    switch (severity) {
	case severities.normal: 
	    status_bar.classList.add('normal');
	    break;
	case severities.warning: 
	    status_bar.classList.add('warning');
	    break;
	case severities.error: 
	    status_bar.classList.add('error');
	    break;
	default:
	    throw "unknown severity";
    }

    status_bar.classList.add('open');

    if (timeout !== 0) {
	msgtimeout = setTimeout(hideMsg, timeout || 1500);
    }
}

function hideMsg() {
    clearTimeout(msgtimeout);
    status_bar.classList.remove('open');
}

function getQueryParameter(name) {
    var params = location.search.slice(1).split('&');
    var result = "";
    params.forEach(function(param) {
	var components = param.split('=');
	if (components[0] === name) {
	    result = components.slice(1).join('=');
	}
    });
    return result;
}

var cur = 0;
var left = 0;

function Utf8ArrayToStr(arraybuffer) {
    var array = new Uint8Array(arraybuffer),
	i = 0,
	len = array.byteLength,
	out = "",
	c;

    while (i < len) {
	c = array[i++];
	if (!left && c < 0x80) {
		out += String.fromCharCode(c);
	} else if(!left) {
	    switch (c >> 4) {
		case 12: case 13:
		    // 110x xxxx 10xx xxxx
		    cur = (c & 0x1F) << 6;
		    left = 1;
		    break;
		case 14:
		    // 1110 xxxx 10xx xxxx 10xx xxxx
		    cur = (c & 0x0F) << 12;
		    left = 2;
		    break;
		case 15:
		    // 1111 0xxx 10xx xxxx 10xx xxxx 10xx xxxx
		    cur = (c & 0x07) << 18;
		    left = 3;
		    break;
		default:
		    cur = 0;
		    out += '\ufffd';
	    }
	} else if (c >= 0x80 && c <= 0xBF) {
	    cur = cur | ((c & 0x3F) << (--left * 6));
	    if (!left) {
		out += String.fromCharCode(cur);
		cur = 0;
	    }
	} else {
	    cur = 0;
	    left = 0;
	    out += '\ufffd';
	}
    }

    return out;
}

function API2Request(reqOpts) {
    var me = this;

    reqOpts.method = reqOpts.method || 'GET';

    var xhr = new XMLHttpRequest();

    xhr.onload = function() {
	var scope = reqOpts.scope || this;
	var result;
	var errmsg;

	if (xhr.readyState === 4) {
	    var ctype = xhr.getResponseHeader('Content-Type');
	    if (xhr.status === 200) {
		if (ctype.match(/application\/json;/)) {
		    result = JSON.parse(xhr.responseText);
		} else {
		    errmsg = 'got unexpected content type ' + ctype;
		}
	    } else {
		errmsg = 'Error ' + xhr.status + ': ' + xhr.statusText;
	    }
	} else {
	    errmsg = 'Connection error - server offline?';
	}

	if (errmsg !== undefined) {
	    if (reqOpts.failure) {
		reqOpts.failure.call(scope, errmsg);
	    }
	} else {
	    if (reqOpts.success) {
		reqOpts.success.call(scope, result);
	    }
	}
	if (reqOpts.callback) {
	    reqOpts.callback.call(scope, errmsg === undefined);
	}
    }

    var data = urlEncode(reqOpts.params || {});

    if (reqOpts.method === 'GET') {
	xhr.open(reqOpts.method, "/api2/json" + reqOpts.url + '?' + data);
    } else {
	xhr.open(reqOpts.method, "/api2/json" + reqOpts.url);
    }
    xhr.setRequestHeader('Cache-Control', 'no-cache');
    if (reqOpts.method === 'POST' || reqOpts.method === 'PUT') {
	xhr.setRequestHeader('Content-Type', 'application/x-www-form-urlencoded');
	xhr.setRequestHeader('CSRFPreventionToken', PVE.CSRFPreventionToken);
	xhr.send(data);
    } else if (reqOpts.method === 'GET') {
	xhr.send();
    } else {
	throw "unknown method";
    }
}

function getTerminalSettings() {
    var res = {};
    var settings = ['fontSize', 'fontFamily', 'letterSpacing', 'lineHeight'];
    if(localStorage) {
	settings.forEach(function(setting) {
	    var val = localStorage.getItem('pve-xterm-' + setting);
	    if (val !== undefined && val !== null) {
		res[setting] = val;
	    }
	});
    }
    return res;
}
