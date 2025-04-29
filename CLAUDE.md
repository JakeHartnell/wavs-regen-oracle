# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands
- `make build`: Build both Solidity contracts and Rust WASM components
- `make wasi-build`: Build only WASM components
- `forge build`: Build Solidity contracts
- `cargo component build --release`: Build Rust components

## Test Commands
- `make test`: Run all Solidity tests
- `forge test --match-contract <ContractName>`: Run specific contract tests
- `forge test --match-test <testName>`: Run a single test
- `forge test -vvv`: Run tests with verbose output

## Lint Commands
- `make fmt`: Format all code
- `npm run lint:sol`: Run Solhint on Solidity files
- `npm run lint:natspec`: Check Solidity NatSpec comments

## Code Style Guidelines
- **Rust**: Follow rustfmt.toml settings with `use_small_heuristics = "Max"` and `use_field_init_shorthand = true`
- **Solidity**: Include SPDX License headers, use pragma solidity 0.8.22
- **Error Handling**: Use Result types in Rust, require/revert with clear messages in Solidity
- **Imports**: Group by external/internal, alphabetize
- **Naming**: PascalCase for contracts/types, snake_case for Rust functions/variables, camelCase for Solidity functions/variables