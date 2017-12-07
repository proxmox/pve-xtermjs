console.log('xtermjs: starting');

var states = {
    start:         1,
    connecting:    2,
    connected:     3,
    disconnecting: 4,
    disconnected:  5,
};

var term,
    protocol,
    socketURL,
    socket,
    ticket,
    resize,
    ping,
    state = states.start;

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
	case states.disconnected:
	    switch (state) {
		case states.start:
		case states.connecting:
		    message = "Connection failed";
		    timeout = 0;
		    severity = severities.error;
		    break;
		case states.connected:
		case states.disconnecting:
		    message = "Connection closed";
		    timeout = 0;
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

createTerminal();

function createTerminal() {
    term = new Terminal();

    term.on('resize', function (size) {
	if (state === states.connected) {
	    socket.send("1:" + size.cols + ":" + size.rows + ":");
	}
    });

    protocol = (location.protocol === 'https:') ? 'wss://' : 'ws://';

    var params = {};
    var type = getQueryParameter('console');
    var vmid = getQueryParameter('vmid');
    var vmname = getQueryParameter('vmname');
    var nodename = getQueryParameter('node');
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
	    socket.onclose = stopTerminal;
	    socket.onerror = errorTerminal;
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

    setTimeout(function() {term.fit();}, 250);
}

function stopTerminal(event) {
    term.off('resize');
    term.off('data');
    clearInterval(ping);
    socket.close();
    updateState(states.disconnected, event.msg + event.code);
}

function errorTerminal(event) {
    term.off('resize');
    term.off('data');
    clearInterval(ping);
    socket.close();
    term.destroy();
    updateState(states.disconnected, event.msg + event.code);
}
