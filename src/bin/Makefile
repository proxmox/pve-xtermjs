include	 ../../defines.mk

.PHONY: install
install: termproxy
	perl -I.. -T -e "use PVE::CLI::termproxy; PVE::CLI::termproxy->verify_api();"
	install -d ${BINDIR}
	install -m 0755 termproxy ${BINDIR}
