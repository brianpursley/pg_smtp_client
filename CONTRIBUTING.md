# pg_smtp_client

## Setup

```shell
make init
```

## Running the Extension Locally

```shell
make run
```

## Running Unit Tests

```shell
make test
```

## Testing the Installation

Use trunk to build an installation package.
```shell
make build
```

Start a tembo-local instance and exec into it.
```shell
docker run -d -it --name local-tembo -p 5432:5432 -v .:/pg_smtp_client --rm quay.io/tembo/tembo-local
```
```shell
docker exec -it local-tembo /bin/bash
```

Use trunk to install the extension
```shell
trunk install -f /pg_smtp_client/.trunk/*.tar.gz pg_smtp_client
```

Connect using psql and enable the extension
```shell
psql postgres://postgres:postgres@localhost:5432
```
```sql
CREATE EXTENSION IF NOT EXISTS pg_smtp_client CASCADE;
```