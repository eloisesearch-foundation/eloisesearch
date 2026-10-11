.DEFAULT_GOAL := help
.PHONY: help build run test fmt fmt-check lint check docker-build docker-up docker-down

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-13s %s\n", $$1, $$2}'

build: ## Build the server (debug)
	cargo build

run: ## Run the server locally on :7700 with ./data
	cargo run --bin eloisesearch

test: ## Run all tests
	cargo test --all-targets

fmt: ## Format the code
	cargo fmt --all

fmt-check: ## Check formatting
	cargo fmt --all -- --check

lint: ## Clippy with warnings as errors
	cargo clippy --all-targets -- -D warnings

check: fmt-check lint test ## Everything CI runs

docker-build: ## Build the container image
	docker build -t eloisesearch:dev .

docker-up: ## Start the service with Docker Compose (persistent volume)
	docker compose up --build -d

docker-down: ## Stop the Compose service (keeps the data volume)
	docker compose down
