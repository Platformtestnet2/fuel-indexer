use crate::{
    executor::{ExecutorSource, NativeIndexExecutor, WasmIndexExecutor},
    Database, IndexerConfig, IndexerError, IndexerResult, Manifest,
};
use async_std::sync::{Arc, Mutex};
use fuel_indexer_database::{
    queries, types::IndexerAssetType, IndexerConnection, IndexerConnectionPool,
};
use fuel_indexer_lib::{defaults, utils::ServiceRequest};
use fuel_indexer_schema::db::manager::SchemaManager;
use fuel_indexer_types::fuel::BlockData;
use futures::{
    stream::{FuturesUnordered, StreamExt},
    Future,
};
use std::collections::HashMap;
use std::marker::Send;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{
    sync::mpsc::Receiver,
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{debug, error, info, warn};

pub struct IndexerService {
    config: IndexerConfig,
    pool: IndexerConnectionPool,
    manager: SchemaManager,
    handles: HashMap<String, JoinHandle<()>>,
    rx: Receiver<ServiceRequest>,
    killers: HashMap<String, Arc<AtomicBool>>,
}

impl IndexerService {
    pub async fn new(
        config: IndexerConfig,
        pool: IndexerConnectionPool,
        rx: Receiver<ServiceRequest>,
    ) -> IndexerResult<IndexerService> {
        let manager = SchemaManager::new(pool.clone());

        Ok(IndexerService {
            config,
            pool,
            manager,
            handles: HashMap::default(),
            killers: HashMap::default(),
            rx,
        })
    }

    pub async fn register_indexer_from_manifest(
        &mut self,
        mut manifest: Manifest,
    ) -> IndexerResult<()> {
        let mut conn = self.pool.acquire().await?;

        let indexer_exists = (queries::get_indexer_id(
            &mut conn,
            &manifest.namespace,
            &manifest.identifier,
        )
        .await)
            .is_ok();
        if indexer_exists {
            if !self.config.replace_indexer {
                return Err(IndexerError::Unknown(format!(
                    "Indexer({}.{}) already exists",
                    &manifest.namespace, &manifest.identifier
                )));
            } else if let Err(e) = queries::remove_indexer(
                &mut conn,
                &manifest.namespace,
                &manifest.identifier,
            )
            .await
            {
                error!(
                    "Failed to remove Indexer({}.{}): {e}",
                    &manifest.namespace, &manifest.identifier
                );
                queries::revert_transaction(&mut conn).await?;
                return Err(e.into());
            }
        }

        let index = queries::register_indexer(
            &mut conn,
            &manifest.namespace,
            &manifest.identifier,
            None,
        )
        .await?;

        let schema = manifest.graphql_schema()?;
        let schema_bytes = schema.as_bytes().to_vec();

        self.manager
            .new_schema(
                &manifest.namespace,
                &manifest.identifier,
                &schema,
                manifest.execution_source(),
                &mut conn,
            )
            .await?;

        let start_block = get_start_block(&mut conn, &manifest).await?;
        manifest.start_block = Some(start_block);
        let (handle, exec_source, killer) = WasmIndexExecutor::create(
            &self.config,
            &manifest,
            ExecutorSource::Manifest,
            self.pool.clone(),
        )
        .await?;

        let mut items = vec![
            (IndexerAssetType::Wasm, exec_source.to_vec()),
            (
                IndexerAssetType::Manifest,
                Manifest::try_into(manifest.clone())?,
            ),
            (IndexerAssetType::Schema, schema_bytes),
        ];

        while let Some((asset_type, bytes)) = items.pop() {
            info!(
                "Registering Asset({:?}) for Indexer({})",
                asset_type,
                index.uid()
            );

            {
                queries::register_indexer_asset(
                    &mut conn,
                    &manifest.namespace,
                    &manifest.identifier,
                    bytes,
                    asset_type,
                    None,
                )
                .await?;
            }
        }
        info!("Registered Indexer({})", &manifest.uid());
        self.handles.insert(manifest.uid(), handle);
        self.killers.insert(manifest.uid(), killer);

        Ok(())
    }

    pub async fn register_indexers_from_registry(&mut self) -> IndexerResult<()> {
        let mut conn = self.pool.acquire().await?;
        let indices = queries::all_registered_indexers(&mut conn).await?;
        for index in indices {
            let assets = queries::latest_assets_for_indexer(&mut conn, &index.id).await?;
            let mut manifest = Manifest::try_from(&assets.manifest.bytes)?;

            let start_block = get_start_block(&mut conn, &manifest).await.unwrap_or(1);
            manifest.start_block = Some(start_block);
            let (handle, _module_bytes, killer) = WasmIndexExecutor::create(
                &self.config,
                &manifest,
                ExecutorSource::Registry(assets.wasm.bytes),
                self.pool.clone(),
            )
            .await?;

            info!("Registered Indexer({})", manifest.uid());
            self.handles.insert(manifest.uid(), handle);
            self.killers.insert(manifest.uid(), killer);
        }

        Ok(())
    }

    pub async fn register_native_indexer<
        T: Future<Output = IndexerResult<()>> + Send + 'static,
    >(
        &mut self,
        mut manifest: Manifest,
        handle_events: fn(Vec<BlockData>, Arc<Mutex<Database>>) -> T,
    ) -> IndexerResult<()> {
        let mut conn = self.pool.acquire().await?;
        let _index = queries::register_indexer(
            &mut conn,
            &manifest.namespace,
            &manifest.identifier,
            None,
        )
        .await?;
        let schema = manifest.graphql_schema()?;
        let _schema_bytes = schema.as_bytes().to_vec();

        self.manager
            .new_schema(
                &manifest.namespace,
                &manifest.identifier,
                &schema,
                manifest.execution_source(),
                &mut conn,
            )
            .await?;

        let start_block = get_start_block(&mut conn, &manifest).await.unwrap_or(1);
        manifest.start_block = Some(start_block);
        let uid = manifest.uid();
        let (handle, _module_bytes, killer) = NativeIndexExecutor::<T>::create(
            &self.config,
            &manifest,
            self.pool.clone(),
            handle_events,
        )
        .await?;

        info!("Registered NativeIndex({})", uid);

        self.handles.insert(uid.clone(), handle);
        self.killers.insert(uid, killer);
        Ok(())
    }

    pub async fn run(self) {
        let IndexerService {
            handles,
            rx,
            pool,
            config,
            killers,
            ..
        } = self;

        let futs = Arc::new(Mutex::new(FuturesUnordered::from_iter(
            handles.into_values(),
        )));

        let _ = tokio::spawn(create_service_task(
            rx,
            config.clone(),
            pool.clone(),
            futs.clone(),
            killers,
        ))
        .await
        .unwrap();

        while let Some(fut) = futs.lock().await.next().await {
            info!("Retired a future {fut:?}");
        }
    }
}

async fn create_service_task(
    mut rx: Receiver<ServiceRequest>,
    config: IndexerConfig,
    pool: IndexerConnectionPool,
    futs: Arc<Mutex<FuturesUnordered<JoinHandle<()>>>>,
    mut killers: HashMap<String, Arc<AtomicBool>>,
) -> IndexerResult<()> {
    loop {
        let futs = futs.lock().await;
        match rx.try_recv() {
            Ok(service_request) => match service_request {
                ServiceRequest::Reload(request) => {
                    let mut conn = pool.acquire().await?;

                    match queries::get_indexer_id(
                        &mut conn,
                        &request.namespace,
                        &request.identifier,
                    )
                    .await
                    {
                        Ok(id) => {
                            let assets =
                                queries::latest_assets_for_indexer(&mut conn, &id)
                                    .await?;
                            let mut manifest =
                                Manifest::try_from(&assets.manifest.bytes)?;

                            let start_block =
                                get_start_block(&mut conn, &manifest).await?;
                            manifest.start_block = Some(start_block);
                            let (handle, _module_bytes, killer) =
                                WasmIndexExecutor::create(
                                    &config,
                                    &manifest,
                                    ExecutorSource::Registry(assets.wasm.bytes),
                                    pool.clone(),
                                )
                                .await?;

                            futs.push(handle);

                            if let Some(killer_for_prev_executor) =
                                killers.insert(manifest.uid(), killer)
                            {
                                let uid = manifest.uid();
                                info!("Indexer({uid}) was replaced. Stopping previous version of Indexer({uid}).");
                                killer_for_prev_executor.store(true, Ordering::SeqCst);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to find Indexer({}.{}): {}",
                                &request.namespace, &request.identifier, e
                            );

                            continue;
                        }
                    }
                }
                ServiceRequest::Stop(request) => {
                    let uid = format!("{}.{}", request.namespace, request.identifier);

                    if let Some(killer) = killers.remove(&uid) {
                        killer.store(true, Ordering::SeqCst);
                    } else {
                        warn!("Stop Indexer: No indexer with the name Indexer({uid})");
                    }
                }
            },
            Err(e) => {
                debug!("No service request to handle: {e:?}.");
                sleep(Duration::from_secs(defaults::IDLE_SERVICE_WAIT_SECS)).await;
            }
        }
    }
}

async fn get_start_block(
    conn: &mut IndexerConnection,
    manifest: &Manifest,
) -> Result<u64, IndexerError> {
    let Manifest {
        namespace,
        identifier,
        start_block,
        resumable,
        ..
    } = manifest;
    match &resumable {
        Some(resumable) => {
            let last =
                queries::last_block_height_for_indexer(conn, namespace, identifier)
                    .await?;
            let start = start_block.unwrap_or(last);
            let resume = if *resumable {
                std::cmp::max(start, last)
            } else {
                start
            };
            info!("Resuming index from block {resume}");
            Ok(resume)
        }
        None => Ok(start_block.unwrap_or(1)),
    }
}
