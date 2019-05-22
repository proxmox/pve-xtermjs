include /usr/share/dpkg/pkg-info.mk

PACKAGE=pve-xtermjs

export VERSION=${DEB_VERSION_UPSTREAM_REVISION}

XTERMJSVER=3.13.2
XTERMJSTGZ=xterm-${XTERMJSVER}.tgz

SRCDIR=src

GITVERSION:=$(shell cat .git/refs/heads/master)

DEB=${PACKAGE}_${VERSION}_all.deb

all: ${DEB}
	@echo ${DEB}

.PHONY: deb
deb: ${DEB}
${DEB}:
	rm -rf ${SRCDIR}.tmp
	cp -rpa ${SRCDIR} ${SRCDIR}.tmp
	cp -a debian ${SRCDIR}.tmp/
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout ${GITVERSION}" > ${SRCDIR}.tmp/debian/SOURCE
	cd ${SRCDIR}.tmp; dpkg-buildpackage -b -uc -us
	lintian ${DEB}
	@echo ${DEB}


X_EXCLUSIONS=--exclude=addons/attach --exclude=addons/fullscreen --exclude=addons/search \
  --exclude=addons/terminado --exclude=addons/webLinks --exclude=addons/zmodem
.PHONY: download
download:
	wget https://registry.npmjs.org/xterm/-/${XTERMJSTGZ} -O ${XTERMJSTGZ}.tmp
	mv ${XTERMJSTGZ}.tmp ${XTERMJSTGZ}
	tar -C $(SRCDIR)/www -xf ${XTERMJSTGZ} package/dist --strip-components=2 ${X_EXCLUSIONS}
	rm ${XTERMJSTGZ}

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
