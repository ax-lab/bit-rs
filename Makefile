.PHONY: all build test test-cargo test-script test-release test-go

all: build

build:
	@ go build ./go/bit-wrapper/bit.go
	@ cargo  build --all --manifest-path rust/Cargo.toml $(cargo)

test: test-cargo test-go test-script
	@echo
	@echo ======================================================
	@echo Tests passed!!!
	@echo ======================================================
	@echo
	
test-cargo:
	@echo
	@echo ------------------------------------------------------
	@echo :: Cargo tests
	@echo ------------------------------------------------------
	@echo
	@ ./cargo-test --all

test-script:
	@echo
	@echo ------------------------------------------------------
	@echo :: Script tests
	@echo ------------------------------------------------------
	@ go run ./go/bit-wrapper/bit.go test

test-release:
	@ ./cargo-test --release

test-go:
	@ go test ./go/*
