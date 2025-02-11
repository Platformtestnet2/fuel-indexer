#[allow(unused)]
pub(crate) use crate::commands::{
    auth::Command as AuthCommand, build::Command as BuildCommand,
    check::Command as CheckCommand, deploy::Command as DeployCommand,
    init::Command as InitCommand, kill::Command as KillCommand,
    new::Command as NewCommand, pull_abi::Command as PullAbiCommand,
    remove::Command as RemoveCommand, start::Command as StartCommand,
    status::Command as StatusCommand, welcome::Command as WelcomeCommand,
};
use clap::{Parser, Subcommand};
use forc_postgres::{
    cli::{ForcPostgres, Opt as ForcPostgresOpt},
    commands as pg_commands,
};
use forc_tracing::{init_tracing_subscriber, TracingSubscriberOptions};

#[derive(Debug, Parser)]
#[clap(name = "forc index", about = "Fuel Index Orchestrator", version)]
pub struct Opt {
    /// The command to run
    #[clap(subcommand)]
    pub command: ForcIndex,
}

#[derive(Subcommand, Debug)]
pub enum ForcIndex {
    Init(InitCommand),
    New(NewCommand),
    Deploy(DeployCommand),
    Start(Box<StartCommand>),
    Check(CheckCommand),
    Remove(RemoveCommand),
    Build(BuildCommand),
    Auth(AuthCommand),
    Postgres(ForcPostgresOpt),
    PullAbi(PullAbiCommand),
    Kill(KillCommand),
    Status(StatusCommand),
    //Welcome(WelcomeCommand),
}

pub async fn run_cli() -> Result<(), anyhow::Error> {
    let opt = Opt::parse();
    let tracing_options = TracingSubscriberOptions {
        ..Default::default()
    };
    init_tracing_subscriber(tracing_options);

    match opt.command {
        ForcIndex::Init(command) => crate::commands::init::exec(command),
        ForcIndex::New(command) => crate::commands::new::exec(command),
        ForcIndex::Deploy(command) => crate::commands::deploy::exec(command),
        ForcIndex::Start(command) => crate::commands::start::exec(command).await,
        ForcIndex::Check(command) => crate::commands::check::exec(command).await,
        ForcIndex::Remove(command) => crate::commands::remove::exec(command),
        ForcIndex::Build(command) => crate::commands::build::exec(command),
        ForcIndex::PullAbi(command) => crate::commands::pull_abi::exec(command).await,
        //ForcIndex::Welcome(command) => crate::commands::welcome::exec(command).await,
        ForcIndex::Auth(command) => crate::commands::auth::exec(command),
        ForcIndex::Postgres(opt) => match opt.command {
            ForcPostgres::Create(command) => pg_commands::create::exec(command).await,
            ForcPostgres::Stop(command) => pg_commands::stop::exec(command).await,
            ForcPostgres::Drop(command) => pg_commands::drop::exec(command).await,
            ForcPostgres::Start(command) => pg_commands::start::exec(command).await,
        },
        ForcIndex::Kill(command) => crate::commands::kill::exec(command),
        ForcIndex::Status(command) => crate::commands::status::exec(command).await,
    }
}
