WWWBASEDIR=${DESTDIR}/usr/share/pve-xtermjs

SOURCE = \
	xterm-addon-fit.js \
	xterm-addon-fit.js.map \
	index.html.tpl \
	main.js \
	style.css \
	util.js \
	xterm.css \
	xterm.js \
	xterm.js.map

index.html.tpl: index.html.tpl.in
	sed -e 's/@VERSION@/${VERSION}/' $< >$@.tmp
	mv $@.tmp $@

.PHONY: install
install: ${SOURCE}
	install -d ${WWWBASEDIR}
	set -e && for i in ${SOURCE}; do install -m 0644 $$i ${WWWBASEDIR}; done
