# Starting the Fuel Indexer

## Using CLI options

```text
Standalone binary for the fuel indexer service.

USAGE:
    fuel-indexer run [OPTIONS]

OPTIONS:
        --auth-enabled
            Require users to authenticate for some operations.

        --auth-strategy <AUTH_STRATEGY>
            Authentication scheme used.

    -c, --config <FILE>
            Indexer service config file.

        --database <DATABASE>
            Database type. [default: postgres] [possible values: postgres]

        --embedded-database
            Automatically create and start database using provided options or defaults.

        --fuel-node-host <FUEL_NODE_HOST>
            Host of the running Fuel node. [default: localhost]

        --fuel-node-port <FUEL_NODE_PORT>
            Listening port of the running Fuel node. [default: 4000]

        --graphql-api-host <GRAPHQL_API_HOST>
            GraphQL API host. [default: localhost]

        --graphql-api-port <GRAPHQL_API_PORT>
            GraphQL API port. [default: 29987]

    -h, --help
            Print help information

        --indexer-handler-timeout <INDEXER_HANDLER_TIMEOUT>
            Maximum length of time (in seconds) that an indexer's event handler can run before timing out. [default: 2]

        --indexer-net-config
            Allow network configuration via indexer manifests.

        --jwt-expiry <JWT_EXPIRY>
            Amount of time (seconds) before expiring token (if JWT scheme is specified).

        --jwt-issuer <JWT_ISSUER>
            Issuer of JWT claims (if JWT scheme is specified).

        --jwt-secret <JWT_SECRET>
            Secret used for JWT scheme (if JWT scheme is specified).

        --local-fuel-node
            Start a local Fuel node.

        --log-level <LOG_LEVEL>
            Log level passed to the Fuel Indexer service. [default: info] [possible values: info,
            debug, error, warn]

    -m, --manifest <FILE>
            Indexer config file.

        --max-body-size <MAX_BODY_SIZE>
            Max body size for GraphQL API requests. [default: 5242880]

        --metrics
            Use Prometheus metrics reporting.

        --postgres-database <POSTGRES_DATABASE>
            Postgres database.

        --postgres-host <POSTGRES_HOST>
            Postgres host.

        --postgres-password <POSTGRES_PASSWORD>
            Postgres password.

        --postgres-port <POSTGRES_PORT>
            Postgres port.

        --postgres-user <POSTGRES_USER>
            Postgres username.

        --rate-limit
            Enable rate limiting.

        --rate-limit-rps <RATE_LIMIT_RPS>
            Maximum number of requests to allow over --rate-limit-window.

        --rate-limit-window <RATE_LIMIT_WINDOW_SIZE>
            Number of seconds over which to allow --rate-limit-rps.

        --run-migrations
            Run database migrations before starting service.

        --stop-idle-indexers
            Prevent indexers from running without handling any blocks.

    -v, --verbose
            Enable verbose logging.

    -V, --version
            Print version information
```

## Using a configuration file

```yaml
{{#include ../../../config.yaml}}
```
