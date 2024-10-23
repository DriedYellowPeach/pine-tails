#!/usr/bin/env bash
# set -x
set -eo pipefail

# In pgadmin4, DB_NAME is the Name filed in general section
# Check if a custom database name has been set, otherwise default to 'newsletter'
DB_NAME="${POSTGRES_DB:=pine_tails_dev}"

# Below are in the connection section
# Check if a custom user has been set, otherwise default to 'postgres'
DB_USER="${POSTGRES_USER:=neil}"
# Check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
# Check if a custom port has been set, otherwise default to '5432'
DB_PORT="${POSTGRES_PORT:=5433}"

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
		-e POSTGRES_USER="${DB_USER}" \
		-e POSTGRES_PASSWORD="${DB_PASSWORD}" \
		-e POSTGRES_DB="${DB_NAME}" \
		-p "${DB_PORT}":5432 \
		-d postgres \
		postgres -N 1000
	#>--------------^ Increased maximum number of connections for testing purposes
}

wait_for_db_ready() {
	export PGPASSWORD="${DB_PASSWORD}"
	# for safety, you can't type password in the command
	until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
		>&2 echo "Postgres is still unavailable - sleeping"
		sleep 1
	done
}

db_migration() {
	export DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}"
	sqlx database create
	sqlx migrate run
}

main() {
	pre_flight_inspection

	if [[ -n "${SKIP_DOCKER}" ]]; then
		echo "Skipped Running postgre in Docker" >&2
	else
		run_docker
	fi

	wait_for_db_ready
	db_migration
	echo "Postgres has been migrated, ready to go! 🚀" >&2
}

main "$@"
