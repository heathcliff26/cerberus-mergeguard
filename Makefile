SHELL := bash

REPOSITORY ?= localhost
CONTAINER_NAME ?= cerberus-mergeguard
TAG ?= latest

# Build the binary in release mode
release:
	cargo build --release

# Build the container image
image:
	podman build -t $(REPOSITORY)/$(CONTAINER_NAME):$(TAG) .

# Run cargo test
test:
	cargo test

# Run linter (clippy)
lint:
	cargo clippy -- --deny warnings

# Build the docs, fail on warnings
doc:
	RUSTDOCFLAGS='--deny warnings' cargo doc --no-deps

# Format the code
fmt:
	cargo fmt

# Validate that all generated files are up to date.
validate:
	hack/validate.sh

# Clean up generated files
clean:
	hack/clean.sh

# Show this help message
help:
	@echo "Available targets:"
	@echo ""
	@awk '/^#/{c=substr($$0,3);next}c&&/^[[:alpha:]][[:alnum:]_-]+:/{print substr($$1,1,index($$1,":")),c}1{c=0}' $(MAKEFILE_LIST) | column -s: -t
	@echo ""
	@echo "Run 'make <target>' to execute a specific target."

.PHONY: \
	release \
	image \
	test \
	lint \
	fmt \
	validate \
	clean \
	help \
	$(NULL)
