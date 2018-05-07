console.log('xtermjs: starting');

var states = {
    start:         1,
    connecting:    2,
    connected:     3,
    disconnecting: 4,
    disconnected:  5,
    reconnecting:  6,
};

var term,
    protocol,
    socketURL,
    socket,
    ticket,
    resize,
    ping,
    state = states.start,
    starttime = new Date();

var type = getQueryParameter('console');
var vmid = getQueryParameter('vmid');
var vmname = getQueryParameter('vmname');
var nodename = getQueryParameter('node');

function updateState(newState, msg) {
    var timeout, severity, message;
    switch (newState) {
	case states.connecting:
	    message = "Connecting...";
	    timeout = 0;
	    severity = severities.warning;
	    break;
	case states.connected:
	    message = "Connected";
	    break;
	case states.disconnecting:
	    message = "Disconnecting...";
	    timeout = 0;
	    severity = severities.warning;
	    break;
	case states.reconnecting:
	    message = "Reconnecting...";
	    timeout = 0;
	    severity = severities.warning;
	    break;
	case states.disconnected:
	    switch (state) {
		case states.start:
		case states.connecting:
		case states.reconnecting:
		    message = "Connection failed";
		    timeout = 0;
		    severity = severities.error;
		    break;
		case states.connected:
		case states.disconnecting:
		    var time_since_started = new Date() - starttime;
		    timeout = 5000;
		    if (time_since_started > 5*1000 || type === 'shell') {
			message = "Connection closed";
		    } else {
			message = "Connection failed";
			severity = severities.error;
		    }
		    break;
		case states.disconnected:
		    // no state change
		    break;
		default:
		    throw "unknown state";
	    }
	    break;
	default:
	    throw "unknown state";
    }
    if (msg) {
	message += " (" + msg + ")";
    }
    state = newState;
    showMsg(message, timeout, severity);
}

var terminalContainer = document.getElementById('terminal-container');
document.getElementById('status_bar').addEventListener('click', hideMsg);
Terminal.applyAddon(fit);

createTerminal();

function createTerminal() {
    term = new Terminal(getTerminalSettings());

    term.on('resize', function (size) {
	if (state === states.connected) {
	    socket.send("1:" + size.cols + ":" + size.rows + ":");
	}
    });

    protocol = (location.protocol === 'https:') ? 'wss://' : 'ws://';

    var params = {};
    var url = '/nodes/' + nodename;
    switch (type) {
	case 'kvm':
	    url += '/qemu/' + vmid;
	    break;
	case 'lxc':
	    url += '/lxc/' + vmid;
	    break;
	case 'upgrade':
	    params.upgrade = 1;
	    break;
    }
    API2Request({
	method: 'POST',
	params: params,
	url: url + '/termproxy',
	success: function(result) {
	    var port = encodeURIComponent(result.data.port);
	    ticket = result.data.ticket;
	    socketURL = protocol + location.hostname + ((location.port) ? (':' + location.port) : '') + '/api2/json' + url + '/vncwebsocket?port=' + port + '&vncticket=' + encodeURIComponent(ticket);

	    term.open(terminalContainer, true);
	    socket = new WebSocket(socketURL, 'binary');
	    socket.binaryType = 'arraybuffer';
	    socket.onopen = runTerminal;
	    socket.onclose = tryReconnect;
	    socket.onerror = tryReconnect;
	    window.onbeforeunload = stopTerminal;
	    updateState(states.connecting);
	},
	failure: function(msg) {
	    updateState(states.disconnected,msg);
	}
    });

}

function runTerminal() {
    socket.onmessage = function(event) {
	var answer = Utf8ArrayToStr(event.data);
	if (state === states.connected) {
	    term.write(answer);
	} else if(state === states.connecting) {
	    if (answer.slice(0,2) === "OK") {
		updateState(states.connected);
		term.write(answer.slice(2));
	    } else {
		socket.close();
	    }
	}
    };

    term.on('data', function(data) {
	if (state === states.connected) {
	    socket.send("0:" + unescape(encodeURIComponent(data)).length.toString() + ":" +  data);
	}
    });

    ping = setInterval(function() {
	socket.send("2");
    }, 30*1000);

    window.addEventListener('resize', function() {
	clearTimeout(resize);
	resize = setTimeout(function() {
	    // done resizing
	    term.fit();
	}, 250);
    });

    socket.send(PVE.UserName + ':' + ticket + "\n");

    // initial focus and resize
    setTimeout(function() {
	term.focus();
	term.fit();
    }, 250);
}

function getLxcStatus(callback) {
    API2Request({
	method: 'GET',
	url: '/nodes/' + nodename + '/lxc/' + vmid + '/status/current',
	success: function(result) {
	    if (typeof callback === 'function') {
		callback(true, result);
	    }
	},
	failure: function(msg) {
	    if (typeof callback === 'function') {
		callback(false, msg);
	    }
	}
    });
}

function checkMigration() {
    var apitype = type;
    if (apitype === 'kvm') {
	apitype = 'qemu';
    }
    API2Request({
	method: 'GET',
	params: {
	    type: 'vm'
	},
	url: '/cluster/resources',
	success: function(result) {
	    // if not yet migrated , wait and try again
	    // if not migrating and stopped, cancel
	    // if started, connect
	    result.data.forEach(function(entity) {
		if (entity.id === (apitype + '/' + vmid)) {
		    var started = entity.status === 'running';
		    var migrated = entity.node !== nodename;
		    if (migrated) {
			if (started) {
			    // goto different node
			    location.href = '?console=' + type +
				'&xtermjs=1&vmid=' + vmid + '&vmname=' +
				vmname + '&node=' + entity.node;
			} else {
			    // wait again
			    updateState(states.reconnecting, 'waiting for migration to finish...');
			    setTimeout(checkMigration, 5000);
			}
		    } else {
			if (type === 'lxc') {
			    // we have to check the status of the
			    // container to know if it has the
			    // migration lock
			    getLxcStatus(function(success, result) {
				if (success) {
				    if (result.data.lock === 'migrate') {
					// still waiting
					updateState(states.reconnecting, 'waiting for migration to finish...');
					setTimeout(checkMigration, 5000);
				    } else if (started) {
					// container was rebooted
					location.reload();
				    } else {
					stopTerminal();
				    }
				} else {
				    // probably the status call failed because
				    // the ct is already somewhere else, so retry
				    setTimeout(checkMigration, 1000);
				}
			    });
			} else if (started) {
			    // this happens if we have old data in
			    // /cluster/resources, or the connection
			    // disconnected, so simply try to reload here
			    location.reload();
			} else if (type === 'kvm') {
			    // it seems the guest simply stopped
			    stopTerminal();
			}
		    }

		    return;
		}
	    });
	},
	failure: function(msg) {
	    errorTerminal({msg: msg});
	}
    });
}

function tryReconnect() {
    var time_since_started = new Date() - starttime;
    var type = getQueryParameter('console');
    if (time_since_started < 5*1000 || type === 'shell') { // 5 seconds
	stopTerminal();
	return;
    }

    updateState(states.disconnecting, 'Detecting migration...');
    setTimeout(checkMigration, 5000);
}

function stopTerminal(event) {
    event = event || {};
    term.off('resize');
    term.off('data');
    clearInterval(ping);
    socket.close();
    updateState(states.disconnected, event.msg + event.code);
}

function errorTerminal(event) {
    even = event || {};
    term.off('resize');
    term.off('data');
    clearInterval(ping);
    socket.close();
    term.destroy();
    updateState(states.disconnected, event.msg + event.code);
}
