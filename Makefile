.PHONY: all build test test-cargo test-bit test-release test-go

all: build

build:
	@ go build ./go/bit-wrapper/bit.go
	@ cargo  build --all --manifest-path rust/Cargo.toml $(cargo)

test: test-cargo test-bit
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

test-bit:
	@echo
	@echo ------------------------------------------------------
	@echo :: Script tests
	@echo ------------------------------------------------------
	@ go run ./go/bit-wrapper/bit.go test

test-main:
	@ ./cargo-test --bin bit

test-release:
	@ ./cargo-test --release

test-go:
	@ go test ./go/*
