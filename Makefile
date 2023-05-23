include /usr/share/dpkg/pkg-info.mk
include /usr/share/dpkg/architecture.mk

PACKAGE=pve-xtermjs
CRATENAME=termproxy

export VERSION=$(DEB_VERSION_UPSTREAM_REVISION)

XTERMJSVER=4.16.0
XTERMJSTGZ=xterm-$(XTERMJSVER).tgz

FITADDONVER=0.5.0
FITADDONTGZ=xterm-addon-fit-$(FITADDONVER).tgz

DEB=$(PACKAGE)_$(DEB_VERSION_UPSTREAM_REVISION)_$(DEB_HOST_ARCH).deb
DBG_DEB=$(PACKAGE)-dbgsym_$(DEB_VERSION_UPSTREAM_REVISION)_$(DEB_HOST_ARCH).deb
DSC=rust-$(CRATENAME)_$(DEB_VERSION_UPSTREAM_REVISION).dsc

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
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout $$(git rev-parse HEAD)" \
	    > $@.tmp/debian/SOURCE

.PHONY: deb
deb: $(DEB)
$(DEB) $(DBG_DEB): build
	cd build; dpkg-buildpackage -b -uc -us --no-pre-clean
	lintian $(DEB)
	@echo $(DEB)

.PHONY: dsc
dsc: $(DSC)
$(DSC): build
	cd build; dpkg-buildpackage -S -us -uc -d -nc
	lintian $(DSC)

EXCLUDED_ADDONS=attach fullscreen search terminado webLinks zmodem
X_EXCLUSIONS=$(foreach ADDON,$(EXCLUDED_ADDONS),--exclude=addons/$(ADDON))

.PHONY: download
download:
	wget https://registry.npmjs.org/xterm/-/$(XTERMJSTGZ) -O $(XTERMJSTGZ).tmp
	wget https://registry.npmjs.org/xterm-addon-fit/-/$(FITADDONTGZ) -O $(FITADDONTGZ).tmp
	mv $(XTERMJSTGZ).tmp $(XTERMJSTGZ)
	mv $(FITADDONTGZ).tmp $(FITADDONTGZ)
	tar -C src/www -xf $(XTERMJSTGZ) package/lib package/css --strip-components=2 $(X_EXCLUSIONS)
	tar -C src/www -xf $(FITADDONTGZ) package/lib --strip-components=2 $(X_EXCLUSIONS)
	rm $(XTERMJSTGZ) $(FITADDONTGZ)

.PHONY: upload
upload: $(DEB) $(DBG_DEB)
	tar cf - $(DEB) $(DBG_DEB) |ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve,pbs --dist bullseye

.PHONY: distclean
distclean: clean

.PHONY: clean
clean:
	rm -rf *~ debian/*~ $(PACKAGE)-*/ build/ *.deb *.changes *.dsc *.tar.?z *.buildinfo

.PHONY: dinstall
dinstall: deb
	dpkg -i $(DEB)
