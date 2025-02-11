use crate::cli::StartCommand;
use std::process::Command;
use tracing::info;

pub async fn init(command: StartCommand) -> anyhow::Result<()> {
    let StartCommand {
        log_level,
        config,
        fuel_node_host,
        fuel_node_port,
        graphql_api_host,
        graphql_api_port,
        database,
        postgres_user,
        postgres_password,
        postgres_database,
        postgres_host,
        postgres_port,
        local_fuel_node,
        run_migrations,
        metrics,
        manifest,
        embedded_database,
        auth_enabled,
        auth_strategy,
        jwt_secret,
        jwt_issuer,
        jwt_expiry,
        verbose,
        ..
    } = command;

    let mut cmd = Command::new("fuel-indexer");
    cmd.arg("run");

    if let Some(m) = &manifest {
        cmd.arg("--manifest").arg(m);
    }

    if let Some(c) = &config {
        cmd.arg("--config").arg(c);
    } else {
        // Options that have default values
        cmd.arg("--fuel-node-host").arg(&fuel_node_host);
        cmd.arg("--fuel-node-port").arg(&fuel_node_port);
        cmd.arg("--graphql-api-host").arg(&graphql_api_host);
        cmd.arg("--graphql-api-port").arg(&graphql_api_port);
        cmd.arg("--log-level").arg(&log_level);

        // Bool options
        let options = vec![
            ("--run-migrations", run_migrations),
            ("--metrics", metrics),
            ("--auth-enabled", auth_enabled),
            ("--verbose", verbose),
            ("--local-fuel-node", local_fuel_node),
        ];
        for (opt, value) in options.iter() {
            if *value {
                cmd.arg(opt);
            }
        }

        // Nullable options
        let options = vec![
            ("--auth-strategy", auth_strategy),
            ("--jwt-secret", jwt_secret),
            ("--jwt-issuer", jwt_issuer),
            ("--jwt-expiry", jwt_expiry.map(|x| x.to_string())),
        ];
        for (opt, value) in options.iter() {
            if let Some(value) = value {
                cmd.arg(opt).arg(value);
            }
        }

        match database.as_ref() {
            "postgres" => {
                if embedded_database {
                    cmd.arg("--embedded-database");
                }

                // Postgres optional values
                let postgres_optionals = vec![
                    ("--postgres-user", postgres_user),
                    ("--postgres-password", postgres_password),
                    ("--postgres-host", postgres_host),
                    ("--postgres-port", postgres_port),
                    ("--postgres-database", postgres_database),
                ];

                for (flag, value) in postgres_optionals.iter() {
                    if let Some(v) = value {
                        cmd.arg(flag).arg(v);
                    }
                }
            }
            _ => unreachable!(
                "'postgres' is currently the only supported database option."
            ),
        }
    }

    if verbose {
        info!("{cmd:?}");
    }

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            info!("✅ Successfully started the indexer service at PID {pid}");
        }
        Err(e) => panic!("❌ Failed to spawn fuel-indexer child process: {e:?}."),
    }

    Ok(())
}
