.DEFAULT_GOAL := help

NPM ?= npm
CARGO ?= cargo

.PHONY: help setup dev run build test test-js test-rust check fmt clean config

help: ## Show available targets
	@awk 'BEGIN {FS = ":.*## "; printf "\n  Alfred — GoodGuys raid assistant\n\n"} \
		/^[a-zA-Z0-9_-]+:.*?## / { printf "  %-12s %s\n", $$1, $$2 } \
		END { printf "\n" }' $(MAKEFILE_LIST)

setup: node_modules ## Install Node dependencies

node_modules: package.json package-lock.json
	$(NPM) install
	@touch node_modules

dev: node_modules ## Run Alfred in development mode
	$(NPM) run tauri dev

run: dev ## Same as make dev

build: node_modules ## Build a release app for this OS
	$(NPM) run tauri build

test: node_modules ## Run JavaScript and Rust tests
	$(NPM) test

test-js: node_modules ## Run UI helper tests
	$(NPM) run test:js

test-rust: ## Run Rust tests
	$(NPM) run test:rust

check: node_modules ## Typecheck TypeScript and compile-check Rust
	npx tsc --noEmit
	$(CARGO) check --manifest-path src-tauri/Cargo.toml

fmt: ## Format Rust sources
	$(CARGO) fmt --manifest-path src-tauri/Cargo.toml

clean: ## Remove build artifacts
	rm -rf dist src-tauri/target

config: ## Open the Alfred config directory (creates it if needed)
	@node -e "\
		const os=require('os'), path=require('path'), fs=require('fs'), {spawn}=require('child_process');\
		const home=os.homedir();\
		const dir=process.platform==='darwin'\
			? path.join(home,'Library','Application Support','Alfred')\
			: process.platform==='win32'\
				? path.join(process.env.APPDATA||path.join(home,'AppData','Roaming'),'Alfred')\
				: path.join(process.env.XDG_CONFIG_HOME||path.join(home,'.config'),'alfred');\
		fs.mkdirSync(dir,{recursive:true});\
		console.log(dir);\
		const cmd=process.platform==='darwin'?'open':process.platform==='win32'?'explorer':'xdg-open';\
		spawn(cmd,[dir],{stdio:'ignore',detached:true}).unref();\
	"
