include /usr/share/dpkg/pkg-info.mk
include /usr/share/dpkg/architecture.mk

PACKAGE=pve-xtermjs
CRATENAME=termproxy

export VERSION=${DEB_VERSION_UPSTREAM_REVISION}

XTERMJSVER=4.12.0
XTERMJSTGZ=xterm-${XTERMJSVER}.tgz

FITADDONVER=0.5.0
FITADDONTGZ=xterm-addon-fit-${FITADDONVER}.tgz

SRCDIR=src
GITVERSION:=$(shell git rev-parse HEAD)

DEB=${PACKAGE}_${DEB_VERSION_UPSTREAM_REVISION}_${DEB_BUILD_ARCH}.deb
DSC=rust-${CRATENAME}_${DEB_VERSION_UPSTREAM_REVISION}.dsc

ifeq ($(BUILD_MODE), release)
CARGO_BUILD_ARGS += --release
COMPILEDIR := target/release
else
COMPILEDIR := target/debug
endif

all: cargo-build $(SRCIDR)

.PHONY: $(SUBDIRS)
$(SUBDIRS):
	make -C $@

.PHONY: cargo-build
cargo-build:
	cargo build $(CARGO_BUILD_ARGS)

.PHONY: build
build:
	rm -rf build
	rm -f debian/control
	debcargo package \
	  --config debian/debcargo.toml \
	  --changelog-ready \
	  --no-overlay-write-back \
	  --directory build \
	  $(CRATENAME) \
	  $(shell dpkg-parsechangelog -l debian/changelog -SVersion | sed -e 's/-.*//')
	rm build/Cargo.lock
	find build/debian -name "*.hint" -delete
	cp build/debian/control debian/control
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout ${GITVERSION}" > build/debian/SOURCE

.PHONY: deb
deb: ${DEB}
$(DEB): build
	cd build; dpkg-buildpackage -b -uc -us --no-pre-clean
	lintian ${DEB}
	@echo ${DEB}

.PHONY: dsc
dsc: ${DSC}
$(DSC): build
	cd build; dpkg-buildpackage -S -us -uc -d -nc
	lintian ${DSC}

X_EXCLUSIONS=--exclude=addons/attach --exclude=addons/fullscreen --exclude=addons/search \
  --exclude=addons/terminado --exclude=addons/webLinks --exclude=addons/zmodem
.PHONY: download
download:
	wget https://registry.npmjs.org/xterm/-/${XTERMJSTGZ} -O ${XTERMJSTGZ}.tmp
	wget https://registry.npmjs.org/xterm-addon-fit/-/${FITADDONTGZ} -O ${FITADDONTGZ}.tmp
	mv ${XTERMJSTGZ}.tmp ${XTERMJSTGZ}
	mv ${FITADDONTGZ}.tmp ${FITADDONTGZ}
	tar -C $(SRCDIR)/www -xf ${XTERMJSTGZ} package/lib package/css --strip-components=2 ${X_EXCLUSIONS}
	tar -C $(SRCDIR)/www -xf ${FITADDONTGZ} package/lib --strip-components=2 ${X_EXCLUSIONS}
	rm ${XTERMJSTGZ} ${FITADDONTGZ}

.PHONY: upload
upload: ${DEB}
	tar cf - ${DEB}|ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve --dist buster

.PHONY: distclean
distclean: clean

.PHONY: clean
clean:
	rm -rf *~ debian/*~ ${PACKAGE}-*/ build/ *.deb *.changes *.dsc *.tar.?z *.buildinfo

.PHONY: dinstall
dinstall: deb
	dpkg -i ${DEB}
