include defines.mk

XTERMJSVER=3.12.0
XTERMJSTGZ=xterm-${XTERMJSVER}.tgz
XTERMJSDIR=package
XTERMDATA = ${XTERMJSDIR}/dist/

SRCDIR=src

ARCH:=$(shell dpkg-architecture -qDEB_BUILD_ARCH)
GITVERSION:=$(shell cat .git/refs/heads/master)

DEB=${PACKAGE}_${VERSION}_${ARCH}.deb

all: ${DEB}
	@echo ${DEB}

.PHONY: deb
deb: ${DEB}
${DEB}: ${XTERMDATA}
	rm -rf ${SRCDIR}.tmp
	cp -rpa ${SRCDIR} ${SRCDIR}.tmp
	cp -a debian ${SRCDIR}.tmp/
	cp -ar ${XTERMJSDIR}/dist/* ${SRCDIR}.tmp/www
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout ${GITVERSION}" > ${SRCDIR}.tmp/debian/SOURCE
	cd ${SRCDIR}.tmp; dpkg-buildpackage -b -uc -us
	lintian ${DEB}
	@echo ${DEB}

${XTERMDATA}: ${XTERMJSTGZ}
	rm -rf ${XTTERMDIR}
	tar -xf ${XTERMJSTGZ}

.PHONY: download
download ${XTERMJSTGZ}:
	wget https://registry.npmjs.org/xterm/-/${XTERMJSTGZ} -O ${XTERMJSTGZ}.tmp
	mv ${XTERMJSTGZ}.tmp ${XTERMJSTGZ}

.PHONY: upload
upload: ${DEB}
	tar cf - ${DEB}|ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve --dist stretch

.PHONY: distclean
distclean: clean

.PHONY: clean
clean:
	rm -rf *~ debian/*~ *_${ARCH}.deb ${SRCDIR}.tmp ${XTERMJSDIR} *_all.deb *.changes *.dsc *.buildinfo

.PHONY: dinstall
dinstall: deb
	dpkg -i ${DEB}
