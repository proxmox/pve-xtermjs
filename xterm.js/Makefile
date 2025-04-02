include /usr/share/dpkg/default.mk

PACKAGE=pve-xtermjs

BUILDDIR ?= $(PACKAGE)-$(DEB_VERSION_UPSTREAM)
ORIG_SRC_TAR=$(PACKAGE)_$(DEB_VERSION_UPSTREAM).orig.tar.gz

DEB=$(PACKAGE)_$(DEB_VERSION)_all.deb
DSC=$(PACKAGE)_$(DEB_VERSION).dsc

# for downloading new version use `make XTERMJS_VERSION=X.Y.Z download`
XTERMJS_VERSION ?= $(DEB_VERSION_UPSTREAM)
FIT_ADDON_VERSION = 0.9.0
WEBGL_ADDON_VERSION = 0.17.0

all:

$(BUILDDIR):
	rm -rf $@ $@.tmp
	mkdir $@.tmp
	cp -a debian/ src/ Makefile $@.tmp
	echo "git clone git://git.proxmox.com/git/pve-xtermjs.git\\ngit checkout $$(git rev-parse HEAD)" \
	    > $@.tmp/debian/SOURCE
	mv $@.tmp $@


$(ORIG_SRC_TAR): $(BUILDDIR)
	tar czf $(ORIG_SRC_TAR) --exclude="$(BUILDDIR)/debian" $(BUILDDIR)

.PHONY: deb
deb: $(DEB)
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
download: XTERMJS_TGZ = xterm-$(XTERMJS_VERSION).tgz
download: FIT_ADDON_TGZ = addon-fit-$(FIT_ADDON_VERSION).tgz
download: WEBGL_ADDON_TGZ = addon-webgl-$(WEBGL_ADDON_VERSION).tgz
download:
	wget https://registry.npmjs.org/@xterm/xterm/-/$(XTERMJS_TGZ) -O $(XTERMJS_TGZ).tmp
	wget https://registry.npmjs.org/@xterm/addon-fit/-/$(FIT_ADDON_TGZ) -O $(FIT_ADDON_TGZ).tmp
	wget https://registry.npmjs.org/@xterm/addon-webgl/-/$(WEBGL_ADDON_TGZ) -O $(WEBGL_ADDON_TGZ).tmp
	mv $(XTERMJS_TGZ).tmp $(XTERMJS_TGZ)
	mv $(FIT_ADDON_TGZ).tmp $(FIT_ADDON_TGZ)
	mv $(WEBGL_ADDON_TGZ).tmp $(WEBGL_ADDON_TGZ)
	tar -C src -xf $(XTERMJS_TGZ) package/lib package/css --strip-components=2 $(X_EXCLUSIONS)
	tar -C src -xf $(FIT_ADDON_TGZ) package/lib --strip-components=2 $(X_EXCLUSIONS)
	tar -C src -xf $(WEBGL_ADDON_TGZ) package/lib --strip-components=2 $(X_EXCLUSIONS)
	rm $(XTERMJS_TGZ) $(FIT_ADDON_TGZ) $(WEBGL_ADDON_TGZ)

.PHONY: upload
upload: UPLOAD_DIST ?= $(DEB_DISTRIBUTION)
upload: $(DEB)
	tar cf - $(DEB) |ssh -X repoman@repo.proxmox.com -- upload --product pmg,pve,pbs,pdm --dist $(UPLOAD_DIST)

.PHONY: clean distclean
distclean: clean
clean:
	rm -rf $(PACKAGE)-[0-9]*/ build/
	rm -f *.deb *.changes *.dsc *.tar.* *.buildinfo *.build .do-cargo-build

.PHONY: dinstall
dinstall: deb
	dpkg -i $(DEB)
