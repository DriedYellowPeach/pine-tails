#!/usr/bin/env bash
set -x
set -eo pipefail

# In pgadmin4, DB_NAME is the Name filed in general section
# Check if a custom database name has been set, otherwise default to 'newsletter'
DB_PORT="${DB_PORT:=5433}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=password}"
APP_USER="${APP_USER:=neil}"
APP_USER_PWD="${APP_USER_PWD:=password}"
APP_DB_NAME="${APP_DB_NAME:=pine_tails_dev}"

CONTAINER_NAME="postgres_$(date '+%s')"

print_error() {
	local RED='\033[0;31m'
	local NC='\033[0m'
	echo -e "${RED}Error: $1${NC}"
}

pre_flight_inspection() {
	if [[ -z "$(command -v docker)" ]]; then
		print_error 'docker is not installed' >&2
		exit 1
	fi

	if [[ -z "$(command -v psql)" ]]; then
		print_error 'psql is not installed' >&2
		exit 1
	fi

	if [[ -z "$(command -v sqlx)" ]]; then
		#detailed error message with instruction of how to install
		local err_msg="sqlx is not installed
Use:
    cargo install sqlx-cli
to install it"
		print_error "${err_msg}" >&2
		exit 1
	fi

}

run_docker() {
	# Launch postgres using Docker
	docker run \
		--env POSTGRES_USER="${SUPERUSER}" \
		--env POSTGRES_PASSWORD="${SUPERUSER_PWD}" \
		--health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
		--health-interval=1s \
		--health-timeout=5s \
		--health-retries=5 \
		--publish "${DB_PORT}":5432 \
		--detach \
		--name "${CONTAINER_NAME}" \
		postgres -N 1000
	#>--------------^ Increased maximum number of connections for testing purposes
}

wait_for_db_ready() {
	export PGPASSWORD="${APP_USER_PWD}"

	until [ \
		"$(docker inspect -f "{{.State.Health.Status}}" "${CONTAINER_NAME}")" == \
		"healthy" \
		]; do
		>&2 echo "Postgres is still unavailable - sleeping"
		sleep 1
	done

	# Create the application user
	CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
	docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

	# Grant create db privileges to the app user
	GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
	docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"

	>&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now!"
}

db_migration() {
	export DATABASE_URL="postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}"
	sqlx database create
	sqlx migrate run
}

main() {
	pre_flight_inspection

	if [[ -n "${SKIP_DOCKER}" ]]; then
		echo "Skipped Running postgre in Docker" >&2
	else
		run_docker
		wait_for_db_ready "${CONTAINER_NAME}"
	fi

	db_migration
	echo "Postgres has been migrated, ready to go! 🚀" >&2
}

main "$@"
