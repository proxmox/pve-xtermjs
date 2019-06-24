include /usr/share/dpkg/pkg-info.mk

PACKAGE=pve-xtermjs

export VERSION=${DEB_VERSION_UPSTREAM_REVISION}

XTERMJSVER=3.13.2
XTERMJSTGZ=xterm-${XTERMJSVER}.tgz

SRCDIR=src
BUILDDIR ?= ${PACKAGE}-${DEB_VERSION_UPSTREAM}
GITVERSION:=$(shell git rev-parse HEAD)

DEB=${PACKAGE}_${VERSION}_all.deb
DSC=${PACKAGE}_${VERSION}.dsc

all: ${DEB}
	@echo ${DEB}

${BUILDDIR}: ${SRCDIR} debian
	rm -rf ${BUILDDIR}
	rsync -a ${SRCDIR}/ debian ${BUILDDIR}
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout ${GITVERSION}" > ${BUILDDIR}/debian/SOURCE

.PHONY: deb
deb: ${DEB}
${DEB}: ${BUILDDIR}
	cd ${BUILDDIR}; dpkg-buildpackage -b -uc -us
	lintian ${DEB}
	@echo ${DEB}

.PHONY: dsc
dsc: ${DSC}
${DSC}: ${BUILDDIR}
	cd ${BUILDDIR}; dpkg-buildpackage -S -us -uc -d
	lintian ${DSC}

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
	tar cf - ${DEB}|ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve --dist buster

.PHONY: distclean
distclean: clean

.PHONY: clean
clean:
	rm -rf *~ debian/*~ ${PACKAGE}-*/ *.deb *.changes *.dsc *.tar.gz *.buildinfo

.PHONY: dinstall
dinstall: deb
	dpkg -i ${DEB}
