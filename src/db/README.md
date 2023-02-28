# Shovel PostgreSQL module

Module for database interactions using PG.

## Migrations cheatsheet
```bash
# Create database
psql < src/db/postgres/migrations/__create.sql

# Delete database
psql < src/db/postgres/migrations/__delete.sql

# Run migrations
refinery migrate -p src/db/postgres/migrations
```

## Migrations: Adding new changes
1. Create a new file name `V{0}__{1}.sql` under `db/postgres/migrations` where
{0} is the number succeeding latest migration number and {1} is a brief summary
of the operation.
2. Run 
```bash
refinery migrate -p src/db/postgres/migrations
```
to apply changes

## Migrations: Updating an already existing migration

1. Update the migration file
2. Reset migration with
```bash
psql < src/db/postgres/migrations/__delete.sql &&\
psql < src/db/postgres/migrations/__create.sql
```
- Run 
```bash
refinery migrate -p src/db/postgres/migrations
```
to apply changes

## Migrations: ERROR: database "shovel" is being accessed by other users

`sqlx` accesses to database, causing migrations to fail. Close the process that
runs rust_analyzer (most likely the code editor) and re-run.
