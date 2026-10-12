# FLINT side of benchmark/FLINT_vs_symbolica.
# Build inside the flake's reference shell (pins FLINT 3.6.0 through flake.lock):
#   nix develop .#reference -c make -C benchmark/FLINT_vs_symbolica/flint
# Result: benchmark/FLINT_vs_symbolica/flint/build/flint_bench
CC ?= cc
CFLAGS ?= -O2 -g
PKG_CONFIG ?= pkg-config
BUILD ?= build

FLINT_CFLAGS := $(shell $(PKG_CONFIG) --cflags flint)
FLINT_LIBS := $(shell $(PKG_CONFIG) --libs flint)

$(BUILD)/flint_bench: flint_bench.c
	mkdir -p $(BUILD)
	$(CC) $(CFLAGS) $(FLINT_CFLAGS) -o $@ $< $(FLINT_LIBS)
	$(PKG_CONFIG) --modversion flint > $(BUILD)/flint.version

clean:
	rm -rf $(BUILD)

.PHONY: clean
