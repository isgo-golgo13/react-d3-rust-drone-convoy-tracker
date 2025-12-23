# Drone Convoy Tracking System - Top Level Makefile
# Builds React.js frontend + Rust backend with OpenCV

.PHONY: all build clean test run dev docker-build docker-up docker-down \
        frontend backend lint format check help install-deps

# Configuration
DOCKER_COMPOSE := docker compose
CARGO := cargo
NPM := npm
FRONTEND_DIR := ../drone-convoy-sortie
BACKEND_DIR := .

# Default target
all: build

# ============================================================================
# BUILD TARGETS
# ============================================================================

build: backend frontend
	@echo "Full stack build complete"

backend:
	@echo "Building Rust backend..."
	cd $(BACKEND_DIR) && $(CARGO) build --workspace --release
	@echo "Rust backend built"

backend-debug:
	@echo "Building Rust backend (debug)..."
	cd $(BACKEND_DIR) && $(CARGO) build --workspace
	@echo "Rust backend built (debug)"

frontend:
	@echo "Building React frontend..."
	cd $(FRONTEND_DIR) && $(NPM) install && $(NPM) run build
	@echo "React frontend built"

# ============================================================================
# DEVELOPMENT TARGETS
# ============================================================================

dev: docker-deps
	@echo "🚀 Starting development environment..."
	@$(MAKE) -j2 dev-backend dev-frontend

dev-backend:
	@echo "Starting Rust backend in dev mode..."
	cd $(BACKEND_DIR) && RUST_LOG=debug $(CARGO) run --bin drone-api

dev-frontend:
	@echo "Starting React frontend in dev mode..."
	cd $(FRONTEND_DIR) && $(NPM) run dev

# ============================================================================
# DOCKER TARGETS
# ============================================================================

docker-build:
	@echo "Building Docker images..."
	$(DOCKER_COMPOSE) build

docker-up: docker-build
	@echo "Starting all services..."
	$(DOCKER_COMPOSE) up -d
	@echo "Waiting for ScyllaDB to be ready..."
	@sleep 30
	@echo "Initializing database schema..."
	$(DOCKER_COMPOSE) exec -T scylla-node1 cqlsh -f /docker-entrypoint-initdb.d/schema.cql || true
	@echo "All services running"
	@$(MAKE) docker-status

docker-down:
	@echo "Stopping all services..."
	$(DOCKER_COMPOSE) down

docker-clean:
	@echo "Removing all containers, volumes, and images..."
	$(DOCKER_COMPOSE) down -v --rmi all

docker-logs:
	$(DOCKER_COMPOSE) logs -f

docker-status:
	@echo "Service Status:"
	@echo "================================================"
	@$(DOCKER_COMPOSE) ps
	@echo ""
	@echo "📍 Service URLs:"
	@echo "  React Dashboard:  http://localhost:8080"
	@echo "  Rust API:         http://localhost:3000"
	@echo "  WebSocket:        ws://localhost:9090"
	@echo "  Grafana:          http://localhost:3001 (admin/admin)"
	@echo "  Prometheus:       http://localhost:9091"
	@echo "  Jaeger:           http://localhost:16686"
	@echo "  ScyllaDB CQL:     localhost:9042"
	@echo "================================================"

docker-deps:
	@echo "Starting infrastructure dependencies..."
	$(DOCKER_COMPOSE) up -d scylla-node1 scylla-node2 scylla-node3 redis prometheus grafana jaeger
	@echo "Waiting for ScyllaDB cluster to form..."
	@sleep 45
	@echo "Dependencies ready"

# ============================================================================
# TEST TARGETS
# ============================================================================

test: test-backend test-frontend
	@echo "All tests passed"

test-backend:
	@echo "Running Rust tests..."
	cd $(BACKEND_DIR) && $(CARGO) test --workspace

test-frontend:
	@echo "Running React tests..."
	cd $(FRONTEND_DIR) && $(NPM) test -- --passWithNoTests

bench:
	@echo "Running benchmarks..."
	cd $(BACKEND_DIR) && $(CARGO) bench --workspace

# ============================================================================
# CODE QUALITY TARGETS
# ============================================================================

lint: lint-backend lint-frontend
	@echo "Linting complete"

lint-backend:
	@echo "Linting Rust code..."
	cd $(BACKEND_DIR) && $(CARGO) clippy --workspace -- -D warnings

lint-frontend:
	@echo "🔍 Linting React code..."
	cd $(FRONTEND_DIR) && $(NPM) run lint

format: format-backend format-frontend
	@echo "Formatting complete"

format-backend:
	@echo "Formatting Rust code..."
	cd $(BACKEND_DIR) && $(CARGO) fmt --all

format-frontend:
	@echo "Formatting React code..."
	cd $(FRONTEND_DIR) && $(NPM) run format || true

check:
	@echo "Running all checks..."
	cd $(BACKEND_DIR) && $(CARGO) check --workspace
	cd $(BACKEND_DIR) && $(CARGO) clippy --workspace -- -D warnings
	cd $(BACKEND_DIR) && $(CARGO) fmt --all -- --check
	@echo "All checks passed"

# ============================================================================
# UTILITY TARGETS
# ============================================================================

clean:
	@echo "Cleaning build artifacts..."
	cd $(BACKEND_DIR) && $(CARGO) clean
	cd $(FRONTEND_DIR) && rm -rf node_modules dist
	@echo "Clean complete"

install-deps:
	@echo "Installing dependencies..."
	@echo "Installing Rust dependencies..."
	cd $(BACKEND_DIR) && $(CARGO) fetch
	@echo "Installing Node.js dependencies..."
	cd $(FRONTEND_DIR) && $(NPM) install
	@echo "Dependencies installed"

# Database utilities
db-shell:
	$(DOCKER_COMPOSE) exec scylla-node1 cqlsh

db-status:
	$(DOCKER_COMPOSE) exec scylla-node1 nodetool status

db-init:
	$(DOCKER_COMPOSE) exec -T scylla-node1 cqlsh -f /docker-entrypoint-initdb.d/schema.cql

# ============================================================================
# HELP
# ============================================================================

help:
	@echo "🚁 Drone Convoy Tracking System - Makefile Commands"
	@echo "===================================================="
	@echo ""
	@echo "Build Commands:"
	@echo "  make build          - Build both frontend and backend (release)"
	@echo "  make backend        - Build Rust backend only (release)"
	@echo "  make backend-debug  - Build Rust backend only (debug)"
	@echo "  make frontend       - Build React frontend only"
	@echo ""
	@echo "Development Commands:"
	@echo "  make dev            - Start dev environment (backend + frontend)"
	@echo "  make dev-backend    - Start Rust backend in dev mode"
	@echo "  make dev-frontend   - Start React frontend in dev mode"
	@echo ""
	@echo "Docker Commands:"
	@echo "  make docker-build   - Build Docker images"
	@echo "  make docker-up      - Start all services"
	@echo "  make docker-down    - Stop all services"
	@echo "  make docker-clean   - Remove all containers/volumes/images"
	@echo "  make docker-logs    - Follow container logs"
	@echo "  make docker-status  - Show service status and URLs"
	@echo "  make docker-deps    - Start infrastructure only"
	@echo ""
	@echo "Test Commands:"
	@echo "  make test           - Run all tests"
	@echo "  make test-backend   - Run Rust tests"
	@echo "  make test-frontend  - Run React tests"
	@echo "  make bench          - Run benchmarks"
	@echo ""
	@echo "Code Quality Commands:"
	@echo "  make lint           - Lint all code"
	@echo "  make format         - Format all code"
	@echo "  make check          - Run all checks (clippy, fmt, etc.)"
	@echo ""
	@echo "General Service Commands:"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make install-deps   - Install all dependencies"
	@echo "  make db-shell       - Open ScyllaDB CQL shell"
	@echo "  make db-status      - Show ScyllaDB cluster status"
	@echo "  make db-init        - Initialize database schema"
	@echo ""
