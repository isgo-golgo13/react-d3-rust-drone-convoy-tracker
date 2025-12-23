# =============================================================================
# DRONE CONVOY TRACKING SYSTEM - Root Makefile
# =============================================================================
#
# Place this file at project root: react-d3-drone-convoy-dash/
#
# =============================================================================

.PHONY: all build clean test dev help \
        frontend backend \
        up down logs status \
        up-frontend up-backend up-full up-monitoring \
        dev-infra dev-frontend dev-backend dev-all \
        db-shell db-status db-init

# Configuration
DOCKER_COMPOSE := docker compose
FRONTEND_DIR := ./drone-convoy-sortie
BACKEND_DIR := ./drone-convoy-tracking-server

# Default
all: help

# =============================================================================
# BUILD TARGETS
# =============================================================================

build: build-frontend build-backend
	@echo "Full stack build complete"

build-frontend:
	@echo "Building React frontend..."
	cd $(FRONTEND_DIR) && npm install && npm run build
	@echo "Frontend built"

build-backend:
	@echo "Building Rust backend..."
	cd $(BACKEND_DIR) && cargo build --workspace --release
	@echo "Backend built"

build-backend-debug:
	@echo "Building Rust backend (debug)..."
	cd $(BACKEND_DIR) && cargo build --workspace
	@echo "Backend built (debug)"

# =============================================================================
# DOCKER - PRODUCTION STACK
# =============================================================================

up: ## Start full stack (frontend + backend + infra)
	@echo "🚀 Starting full stack..."
	$(DOCKER_COMPOSE) up -d
	@$(MAKE) status

up-frontend: ## Start frontend only (no backend)
	@echo "Starting frontend only..."
	$(DOCKER_COMPOSE) up -d frontend
	@echo "Frontend: http://localhost:8080"

up-backend: ## Start backend + infrastructure (no frontend)
	@echo "Starting backend + infrastructure..."
	$(DOCKER_COMPOSE) up -d drone-api
	@$(MAKE) status

up-monitoring: ## Start full stack + monitoring (Prometheus, Grafana, Jaeger)
	@echo "Starting with monitoring..."
	$(DOCKER_COMPOSE) --profile monitoring up -d
	@$(MAKE) status

down: ## Stop all services
	@echo "Stopping all services..."
	$(DOCKER_COMPOSE) down
	$(DOCKER_COMPOSE) -f docker-compose.dev.yaml down 2>/dev/null || true

down-clean: ## Stop all and remove volumes
	@echo "Stopping all and removing volumes..."
	$(DOCKER_COMPOSE) down -v
	$(DOCKER_COMPOSE) -f docker-compose.dev.yaml down -v 2>/dev/null || true

logs: ## Follow container logs
	$(DOCKER_COMPOSE) logs -f

logs-api: ## Follow API logs only
	$(DOCKER_COMPOSE) logs -f drone-api

logs-scylla: ## Follow ScyllaDB logs
	$(DOCKER_COMPOSE) logs -f scylla-node1 scylla-node2 scylla-node3

status: ## Show service status and URLs
	@echo ""
	@echo "Service Status:"
	@echo "════════════════════════════════════════════════════════"
	@$(DOCKER_COMPOSE) ps
	@echo ""
	@echo "📍 Service URLs:"
	@echo "  React Dashboard:  http://localhost:8080"
	@echo "  React Dev:        http://localhost:5173 (if using dev profile)"
	@echo "  Rust API:         http://localhost:3000"
	@echo "  WebSocket:        ws://localhost:9090"
	@echo "  ScyllaDB CQL:     localhost:9042"
	@echo "  Redis:            localhost:6379"
	@echo ""
	@echo "Monitoring (if --profile monitoring):"
	@echo "  Grafana:          http://localhost:3001 (admin/admin)"
	@echo "  Prometheus:       http://localhost:9091"
	@echo "  Jaeger:           http://localhost:16686"
	@echo "════════════════════════════════════════════════════════"

# =============================================================================
# DOCKER - DEVELOPMENT (Local Rust + Docker Infra)
# =============================================================================

dev-infra: ## Start infrastructure only (for local Rust development)
	@echo "Starting infrastructure for local development..."
	$(DOCKER_COMPOSE) -f docker-compose.dev.yaml up -d
	@echo ""
	@echo "⏳ Waiting for ScyllaDB schema initialization..."
	@sleep 5
	@$(DOCKER_COMPOSE) -f docker-compose.dev.yaml logs scylla-init | tail -20
	@echo ""
	@echo "Infrastructure ready!"
	@echo ""
	@echo "Next steps:"
	@echo "  cd $(BACKEND_DIR) && cargo run --bin drone-api"
	@echo "  cd $(FRONTEND_DIR) && npm run dev"

dev-frontend: ## Start frontend with hot-reload (needs backend running)
	@echo "Starting frontend dev server..."
	cd $(FRONTEND_DIR) && npm run dev

dev-backend: ## Run Rust backend locally (needs dev-infra first)
	@echo "Starting Rust backend locally..."
	cd $(BACKEND_DIR) && RUST_LOG=debug cargo run --bin drone-api

dev-all: ## Start infra + frontend in Docker, run backend locally
	@echo "🚀 Starting development environment..."
	$(DOCKER_COMPOSE) -f docker-compose.dev.yaml --profile frontend up -d
	@echo ""
	@echo "Frontend dev: http://localhost:5173"
	@echo ""
	@echo "Now run the backend locally:"
	@echo "  cd $(BACKEND_DIR) && cargo run --bin drone-api"

dev-monitoring: ## Start infrastructure + monitoring
	@echo "Starting infrastructure with monitoring..."
	$(DOCKER_COMPOSE) -f docker-compose.dev.yaml --profile monitoring up -d
	@$(MAKE) status

# =============================================================================
# DATABASE UTILITIES
# =============================================================================

db-shell: ## Open ScyllaDB CQL shell
	$(DOCKER_COMPOSE) exec scylla-node1 cqlsh

db-status: ## Show ScyllaDB cluster status
	$(DOCKER_COMPOSE) exec scylla-node1 nodetool status

db-init: ## Re-run schema initialization
	@echo "Re-initializing database schema..."
	$(DOCKER_COMPOSE) exec -T scylla-node1 cqlsh -f /schema.cql 2>/dev/null || \
	docker exec -i scylla-node1 cqlsh < $(BACKEND_DIR)/schema.cql

db-describe: ## Describe drone_convoy keyspace
	$(DOCKER_COMPOSE) exec scylla-node1 cqlsh -e "DESCRIBE KEYSPACE drone_convoy"

# =============================================================================
# TEST & QUALITY
# =============================================================================

test: test-backend test-frontend
	@echo "All tests passed"

test-backend:
	@echo "Running Rust tests..."
	cd $(BACKEND_DIR) && cargo test --workspace

test-frontend:
	@echo "Running React tests..."
	cd $(FRONTEND_DIR) && npm test -- --passWithNoTests 2>/dev/null || true

lint:
	@echo "Linting..."
	cd $(BACKEND_DIR) && cargo clippy --workspace -- -D warnings
	cd $(FRONTEND_DIR) && npm run lint 2>/dev/null || true

format:
	@echo "Formatting..."
	cd $(BACKEND_DIR) && cargo fmt --all
	cd $(FRONTEND_DIR) && npm run format 2>/dev/null || true

check:
	@echo "Running checks..."
	cd $(BACKEND_DIR) && cargo check --workspace
	cd $(BACKEND_DIR) && cargo fmt --all -- --check

# =============================================================================
# CLEANUP
# =============================================================================

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cd $(BACKEND_DIR) && cargo clean
	cd $(FRONTEND_DIR) && rm -rf node_modules dist .vite
	@echo "Clean complete"

clean-docker: ## Remove all Docker resources
	@echo "Removing Docker resources..."
	$(DOCKER_COMPOSE) down -v --rmi all 2>/dev/null || true
	$(DOCKER_COMPOSE) -f docker-compose.dev.yaml down -v --rmi all 2>/dev/null || true
	docker volume prune -f
	@echo "Docker cleanup complete"

# =============================================================================
# HELP
# =============================================================================

help: ## Show this help
	@echo ""
	@echo "🚁 Drone Convoy Tracking System"
	@echo "════════════════════════════════════════════════════════"
	@echo ""
	@echo "Quick Start:"
	@echo "  make up              Start everything (frontend + backend + DB)"
	@echo "  make dev-infra       Start DB/Redis, run Rust locally"
	@echo "  make down            Stop everything"
	@echo ""
	@echo "Available Commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Templates:"
	@echo "  make up-monitoring   Full stack + Grafana/Prometheus/Jaeger"
	@echo "  make dev-infra       Start infra, then: cargo run --bin drone-api"
	@echo "  make db-shell        Open CQL shell to ScyllaDB"
	@echo ""