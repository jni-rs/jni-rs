BINDGEN=bindgen
CARGO=cargo

BINDGEN_OPTS=--builtins --convert-macros

TARGETS=src/java8.rs

all: debug


debug: bindgen
	$(CARGO) build

release: bindgen
	$(CARGO) build --release

doc: bindgen
	$(CARGO) doc

src/java%.rs: headers/jni%.h
	$(BINDGEN) $(BINDGEN_OPTS) $< --output $@

.PHONY: bindgen
bindgen: $(TARGETS)

.PHONY: bindgen
clean:
	rm $(TARGETS)
