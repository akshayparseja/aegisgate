BINARY_NAME = aegis-proxy
DOCKER_IMAGE = aegisgate-proxy:latest
CONFIG_PATH = config/aegis_config.yaml

.PHONY: help build run test stress-test clean docker-build docker-run up up-full down logs

help:
	@echo "AegisGate Management:"
	@echo "  make build          - Build binary locally"
	@echo "  make run            - Run locally"
	@echo "  make docker-build   - Build Docker image"
	@echo "  make up             - Run proxy via Compose"
	@echo "  make up-full        - Run proxy + optional EMQX via Compose"
	@echo "  make down           - Stop all services"
	@echo "  make logs           - Stream proxy logs"

build:
	cargo build --release

run:
	cargo run --bin $(BINARY_NAME)

test:
	cargo test --workspace

stress-test:
	cargo run --example flood_test

clean:
	cargo clean

docker-build:
	docker build -t $(DOCKER_IMAGE) .

docker-run:
	docker run -it --rm \
		-p 8080:8080 \
		-p 9090:9090 \
		--add-host=host.docker.internal:host-gateway \
		-v $(PWD)/$(CONFIG_PATH):/app/$(CONFIG_PATH) \
		$(DOCKER_IMAGE)

up:
	docker-compose up -d aegis-proxy

up-full:
	docker-compose --profile debug-broker up -d

down:
	docker-compose down

logs:
	docker-compose logs -f aegis-proxy

.PHONY: verify
verify:
	@echo "Running integration tests..."
	cargo test --test proxy_test
	@echo "Running end-to-end MQTT flow test..."
	bash scripts/test_mqtt_flow.sh
