include defines.mk

XTERMJSDIR=xtermjs
SRCDIR=src

ARCH:=$(shell dpkg-architecture -qDEB_BUILD_ARCH)
GITVERSION:=$(shell cat .git/refs/heads/master)

DEB=${PACKAGE}_${VERSION}_${ARCH}.deb

all: ${DEB}
	@echo ${DEB}

.PHONY: deb
deb: ${DEB}
${DEB}: | submodule
	rm -rf ${SRCDIR}.tmp
	cp -rpa ${SRCDIR} ${SRCDIR}.tmp
	cp -a debian ${SRCDIR}.tmp/
	cp -ar ${XTERMJSDIR}/dist/* ${SRCDIR}.tmp/www
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout ${GITVERSION}" > ${SRCDIR}.tmp/debian/SOURCE
	cd ${SRCDIR}.tmp; dpkg-buildpackage -rfakeroot -b -uc -us
	lintian ${DEB}
	@echo ${DEB}

.PHONY: submodule
submodule:
	test -f "${XTERMJSDIR}/README.md" || git submodule update --init

.PHONY: download
download ${SRCDIR}:
	git submodule foreach 'git pull --ff-only origin master'

.PHONY: upload
upload: ${DEB}
	tar cf - ${DEB}|ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve --dist stretch

.PHONY: distclean
distclean: clean

.PHONY: clean
clean:
	rm -rf *~ debian/*~ *_${ARCH}.deb ${SRCDIR}.tmp *_all.deb *.changes *.dsc *.buildinfo

.PHONY: dinstall
dinstall: deb
	dpkg -i ${DEB}
