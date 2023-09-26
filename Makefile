include /usr/share/dpkg/pkg-info.mk
include /usr/share/dpkg/architecture.mk

PACKAGE=pve-xtermjs
CRATENAME=termproxy

BUILDDIR ?= $(DEB_SOURCE)-$(DEB_VERSION_UPSTREAM)
ORIG_SRC_TAR=$(DEB_SOURCE)_$(DEB_VERSION_UPSTREAM).orig.tar.gz

export VERSION=$(DEB_VERSION_UPSTREAM_REVISION)

XTERMJSVER=5.3.0
XTERMJSTGZ=xterm-$(XTERMJSVER).tgz

FITADDONVER=0.8.0
FITADDONTGZ=xterm-addon-fit-$(FITADDONVER).tgz

WEBGLADDONVER=0.16.0
WEBGLADDONTGZ=xterm-addon-webgl-$(WEBGLADDONVER).tgz

DEB=$(PACKAGE)_$(DEB_VERSION_UPSTREAM_REVISION)_$(DEB_HOST_ARCH).deb
DBG_DEB=$(PACKAGE)-dbgsym_$(DEB_VERSION_UPSTREAM_REVISION)_$(DEB_HOST_ARCH).deb
DSC=rust-$(CRATENAME)_$(DEB_VERSION_UPSTREAM_REVISION).dsc

CARGO ?= cargo
ifeq ($(BUILD_MODE), release)
CARGO_BUILD_ARGS += --release
COMPILEDIR := target/release
else
COMPILEDIR := target/debug
endif

PREFIX = /usr
BINDIR = $(PREFIX)/bin
TERMPROXY_BIN := $(addprefix $(COMPILEDIR)/,termproxy)

all:

install: $(TERMPROXY_BIN)
	install -dm755 $(DESTDIR)$(BINDIR)
	install -m755 $(TERMPROXY_BIN) $(DESTDIR)$(BINDIR)/

$(TERMPROXY_BIN): .do-cargo-build
.do-cargo-build:
	$(CARGO) build $(CARGO_BUILD_ARGS)
	touch .do-cargo-build


.PHONY: cargo-build
cargo-build: .do-cargo-build

update-dcontrol:
	rm -rf $(BUILDDIR)
	$(MAKE) $(BUILDDIR)
	cd $(BUILDDIR); debcargo package \
	  --config debian/debcargo.toml \
	  --changelog-ready \
	  --no-overlay-write-back \
	  --directory build \
	  $(CRATENAME) \
	  $(DEB_VERSION_UPSTREAM)
	cd $(BUILDDIR)/build; wrap-and-sort -tkn
	cp --remove-destination $(BUILDDIR)/build/debian/control debian/control

$(BUILDDIR):
	rm -rf $@ $@.tmp
	mkdir $@.tmp
	cp -a debian/ src/ Makefile Cargo.toml $@.tmp
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout $$(git rev-parse HEAD)" \
	    > $@.tmp/debian/SOURCE
	mv $@.tmp $@


$(ORIG_SRC_TAR): $(BUILDDIR)
	tar czf $(ORIG_SRC_TAR) --exclude="$(BUILDDIR)/debian" $(BUILDDIR)

.PHONY: deb
deb: $(DEB)
$(DBG_DEB): $(DEB)
$(DEB): $(BUILDDIR)
	cd $(BUILDDIR); dpkg-buildpackage -b -uc -us
	lintian $(DEB)
	@echo $(DEB)

.PHONY: dsc
dsc:
	rm -rf $(DSC) $(BUILDDIR)
	$(MAKE) $(DSC)
	lintian $(DSC)

$(DSC): $(BUILDDIR) $(ORIG_SRC_TAR)
	cd $(BUILDDIR); dpkg-buildpackage -S -us -uc -d

sbuild: $(DSC)
	sbuild $(DSC)

EXCLUDED_ADDONS=attach fullscreen search terminado webLinks zmodem
X_EXCLUSIONS=$(foreach ADDON,$(EXCLUDED_ADDONS),--exclude=addons/$(ADDON))

.PHONY: download
download:
	wget https://registry.npmjs.org/xterm/-/$(XTERMJSTGZ) -O $(XTERMJSTGZ).tmp
	wget https://registry.npmjs.org/xterm-addon-fit/-/$(FITADDONTGZ) -O $(FITADDONTGZ).tmp
	wget https://registry.npmjs.org/xterm-addon-webgl/-/$(WEBGLADDONTGZ) -O $(WEBGLADDONTGZ).tmp
	mv $(XTERMJSTGZ).tmp $(XTERMJSTGZ)
	mv $(FITADDONTGZ).tmp $(FITADDONTGZ)
	mv $(WEBGLADDONTGZ).tmp $(WEBGLADDONTGZ)
	tar -C src/www -xf $(XTERMJSTGZ) package/lib package/css --strip-components=2 $(X_EXCLUSIONS)
	tar -C src/www -xf $(FITADDONTGZ) package/lib --strip-components=2 $(X_EXCLUSIONS)
	tar -C src/www -xf $(WEBGLADDONTGZ) package/lib --strip-components=2 $(X_EXCLUSIONS)
	rm $(XTERMJSTGZ) $(FITADDONTGZ) $(WEBGLADDONTGZ)

.PHONY: upload
upload: UPLOAD_DIST ?= $(DEB_DISTRIBUTION)
upload: $(DEB) $(DBG_DEB)
	tar cf - $(DEB) $(DBG_DEB) |ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve,pbs --dist $(UPLOAD_DIST) --arch $(DEB_HOST_ARCH)

.PHONY: distclean
distclean: clean

.PHONY: clean
clean:
	$(CARGO) clean
	rm -rf $(DEB_SOURCE)-[0-9]*/ build/ *.deb *.changes *.dsc *.tar.* *.buildinfo *.build .do-cargo-build

.PHONY: dinstall
dinstall: deb
	dpkg -i $(DEB)
