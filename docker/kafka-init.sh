#!/bin/bash
set -e

echo "Creating Kafka topic movie-events..."
kafka-topics --create \
  --bootstrap-server "$KAFKA_BROKER" \
  --topic movie-events \
  --partitions 3 \
  --replication-factor 1 \
  --if-not-exists

echo "Topic created. Registering Avro schema..."

SCHEMA=$(cat /schemas/movie_event.avsc | tr -d '\n' | sed 's/"/\\"/g')

curl -s -X POST \
  -H "Content-Type: application/vnd.schemaregistry.v1+json" \
  --data "{\"schema\": \"$SCHEMA\"}" \
  "$SCHEMA_REGISTRY_URL/subjects/movie-events-value/versions"

echo "Schema registered."