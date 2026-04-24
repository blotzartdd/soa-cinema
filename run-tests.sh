#!/bin/bash
set -e

docker-compose up -d zookeeper kafka schema-registry clickhouse postgres
docker-compose up --exit-code-from kafka-init kafka-init

cd movie-producer
KAFKA_BROKERS=localhost:9092 \
SCHEMA_REGISTRY_URL=http://localhost:8081 \
CLICKHOUSE_URL=http://localhost:8123 \
cargo test --test pipeline_integration_test -- --nocapture
