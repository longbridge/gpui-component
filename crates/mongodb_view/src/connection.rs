//! MongoDB 连接实现

use crate::types::{MongoConnectionConfig, MongoError};
use async_trait::async_trait;
use futures_util::stream::TryStreamExt;
use mongodb::Client;
use mongodb::bson::{Bson, Document, doc};
use mongodb::options::FindOptions;
use rust_i18n::t;
use tracing::{error, info, warn};

/// MongoDB 连接 trait
#[async_trait]
pub trait MongoConnection: Send + Sync {
    fn config(&self) -> &MongoConnectionConfig;

    async fn connect(&mut self) -> Result<(), MongoError>;

    async fn disconnect(&mut self) -> Result<(), MongoError>;

    async fn ping(&self) -> Result<(), MongoError>;

    fn is_connected(&self) -> bool;

    async fn list_databases(&self) -> Result<Vec<String>, MongoError>;

    async fn list_collections(&self, database_name: &str) -> Result<Vec<String>, MongoError>;

    async fn create_collection(
        &self,
        database_name: &str,
        collection_name: &str,
    ) -> Result<(), MongoError>;

    async fn drop_database(&self, database_name: &str) -> Result<(), MongoError>;

    async fn aggregate_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        pipeline: Vec<Document>,
    ) -> Result<Vec<Document>, MongoError>;

    async fn list_indexes(
        &self,
        database_name: &str,
        collection_name: &str,
    ) -> Result<Vec<Document>, MongoError>;

    async fn create_index(
        &self,
        database_name: &str,
        collection_name: &str,
        keys: Document,
        name: Option<String>,
    ) -> Result<(), MongoError>;

    async fn drop_index(
        &self,
        database_name: &str,
        collection_name: &str,
        name: &str,
    ) -> Result<(), MongoError>;

    async fn get_collection_validation(
        &self,
        database_name: &str,
        collection_name: &str,
    ) -> Result<Option<Document>, MongoError>;

    async fn update_collection_validation(
        &self,
        database_name: &str,
        collection_name: &str,
        validator: Option<Document>,
    ) -> Result<(), MongoError>;

    async fn find_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
        options: FindOptions,
    ) -> Result<Vec<Document>, MongoError>;

    async fn count_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
    ) -> Result<i64, MongoError>;

    async fn insert_document(
        &self,
        database_name: &str,
        collection_name: &str,
        document: Document,
    ) -> Result<(), MongoError>;

    async fn replace_document(
        &self,
        database_name: &str,
        collection_name: &str,
        id: Bson,
        document: Document,
    ) -> Result<(), MongoError>;

    async fn delete_document(
        &self,
        database_name: &str,
        collection_name: &str,
        id: Bson,
    ) -> Result<(), MongoError>;

    async fn explain_find(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
        options: FindOptions,
    ) -> Result<Document, MongoError>;
}

/// MongoDB 连接实现
pub struct MongoConnectionImpl {
    config: MongoConnectionConfig,
    client: Option<Client>,
}

impl MongoConnectionImpl {
    pub fn new(config: MongoConnectionConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    fn client(&self) -> Result<&Client, MongoError> {
        self.client.as_ref().ok_or(MongoError::NotConnected)
    }

    fn has_auth_source(connection_string: &str) -> bool {
        connection_string
            .to_ascii_lowercase()
            .contains("authsource=")
    }

    fn has_credentials(connection_string: &str) -> bool {
        connection_string.contains('@')
    }

    fn is_authentication_failed(error: &mongodb::error::Error) -> bool {
        let message = error.to_string();
        message.contains("SCRAM failure")
            || message.contains("AuthenticationFailed")
            || message.contains("code 18")
    }

    fn append_admin_auth_source(connection_string: &str) -> String {
        if connection_string.contains('?') {
            if connection_string.ends_with('?') || connection_string.ends_with('&') {
                return format!("{}authSource=admin", connection_string);
            }
            return format!("{}&authSource=admin", connection_string);
        }

        let has_path = connection_string
            .split_once("://")
            .map(|(_, remaining)| remaining.contains('/'))
            .unwrap_or(false);
        if has_path {
            format!("{}?authSource=admin", connection_string)
        } else {
            format!("{}/?authSource=admin", connection_string)
        }
    }

    fn should_retry_with_admin_auth_source(
        connection_string: &str,
        error: &mongodb::error::Error,
    ) -> bool {
        !Self::has_auth_source(connection_string)
            && Self::has_credentials(connection_string)
            && Self::is_authentication_failed(error)
    }

    /// 通过 $listSessions 聚合管道查询 system.sessions 集合
    async fn find_system_sessions(
        &self,
        database_name: &str,
        filter: Option<Document>,
        options: &FindOptions,
    ) -> Result<Vec<Document>, MongoError> {
        info!("[find_system_sessions] 使用 $listSessions 聚合管道查询");
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>("system.sessions");

        let mut pipeline: Vec<Document> = Vec::new();

        // $listSessions: 尝试 allUsers，权限不足时回退到当前用户
        pipeline.push(doc! { "$listSessions": { "allUsers": true } });

        if let Some(ref f) = filter {
            if !f.is_empty() {
                pipeline.push(doc! { "$match": f });
            }
        }
        if let Some(skip) = options.skip {
            pipeline.push(doc! { "$skip": skip as i64 });
        }
        if let Some(limit) = options.limit {
            pipeline.push(doc! { "$limit": limit });
        }

        info!("[find_system_sessions] pipeline={pipeline:?}");
        let result = collection.aggregate(pipeline).await;

        match result {
            Ok(mut cursor) => {
                let mut documents = Vec::new();
                while let Some(document) = cursor.try_next().await.map_err(|e| {
                    error!("[find_system_sessions] 遍历失败: {e}");
                    MongoError::command_with_source(
                        t!("MongoConnection.find_documents_failed").to_string(),
                        e,
                    )
                })? {
                    documents.push(document);
                }
                info!(
                    "[find_system_sessions] allUsers=true 成功，共 {} 条文档",
                    documents.len()
                );
                Ok(documents)
            }
            Err(e) => {
                // allUsers 权限不足时，回退到当前用户会话
                warn!("[find_system_sessions] allUsers=true 失败: {e}，回退到当前用户");
                let mut pipeline: Vec<Document> = Vec::new();
                pipeline.push(doc! { "$listSessions": {} });

                if let Some(ref f) = filter {
                    if !f.is_empty() {
                        pipeline.push(doc! { "$match": f });
                    }
                }
                if let Some(skip) = options.skip {
                    pipeline.push(doc! { "$skip": skip as i64 });
                }
                if let Some(limit) = options.limit {
                    pipeline.push(doc! { "$limit": limit });
                }

                let mut cursor = collection.aggregate(pipeline).await.map_err(|e| {
                    error!("[find_system_sessions] 回退查询也失败: {e}");
                    MongoError::command_with_source(
                        t!("MongoConnection.find_documents_failed").to_string(),
                        e,
                    )
                })?;

                let mut documents = Vec::new();
                while let Some(document) = cursor.try_next().await.map_err(|e| {
                    error!("[find_system_sessions] 回退遍历失败: {e}");
                    MongoError::command_with_source(
                        t!("MongoConnection.find_documents_failed").to_string(),
                        e,
                    )
                })? {
                    documents.push(document);
                }
                info!(
                    "[find_system_sessions] 回退查询成功，共 {} 条文档",
                    documents.len()
                );
                Ok(documents)
            }
        }
    }
}

#[async_trait]
impl MongoConnection for MongoConnectionImpl {
    fn config(&self) -> &MongoConnectionConfig {
        &self.config
    }

    async fn connect(&mut self) -> Result<(), MongoError> {
        if self.client.is_some() {
            return Ok(());
        }

        let connection_string = self.config.connection_string.clone();
        let client = Client::with_uri_str(&connection_string)
            .await
            .map_err(|e| {
                MongoError::connection_with_source(
                    t!("MongoConnection.connect_failed").to_string(),
                    e,
                )
            })?;

        match client
            .database("admin")
            .run_command(doc! { "ping": 1 })
            .await
        {
            Ok(_) => {
                self.client = Some(client);
                Ok(())
            }
            Err(error) if Self::should_retry_with_admin_auth_source(&connection_string, &error) => {
                let retry_connection_string = Self::append_admin_auth_source(&connection_string);
                let retry_client = Client::with_uri_str(&retry_connection_string)
                    .await
                    .map_err(|e| {
                        MongoError::connection_with_source(
                            t!("MongoConnection.connect_failed").to_string(),
                            e,
                        )
                    })?;

                retry_client
                    .database("admin")
                    .run_command(doc! { "ping": 1 })
                    .await
                    .map_err(|e| {
                        MongoError::command_with_source(
                            t!("MongoConnection.ping_failed").to_string(),
                            e,
                        )
                    })?;

                self.config.connection_string = retry_connection_string;
                self.client = Some(retry_client);
                Ok(())
            }
            Err(error) => Err(MongoError::command_with_source(
                t!("MongoConnection.ping_failed").to_string(),
                error,
            )),
        }
    }

    async fn disconnect(&mut self) -> Result<(), MongoError> {
        self.client = None;
        Ok(())
    }

    async fn ping(&self) -> Result<(), MongoError> {
        let client = self.client()?;
        let database = client.database("admin");
        database
            .run_command(doc! { "ping": 1 })
            .await
            .map_err(|e| {
                MongoError::command_with_source(t!("MongoConnection.ping_failed").to_string(), e)
            })?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    async fn list_databases(&self) -> Result<Vec<String>, MongoError> {
        let client = self.client()?;
        client.list_database_names().await.map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.list_databases_failed").to_string(),
                e,
            )
        })
    }

    async fn list_collections(&self, database_name: &str) -> Result<Vec<String>, MongoError> {
        let client = self.client()?;
        client
            .database(database_name)
            .list_collection_names()
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.list_collections_failed").to_string(),
                    e,
                )
            })
    }

    async fn create_collection(
        &self,
        database_name: &str,
        collection_name: &str,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        client
            .database(database_name)
            .create_collection(collection_name)
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.create_collection_failed").to_string(),
                    e,
                )
            })?;
        Ok(())
    }

    async fn drop_database(&self, database_name: &str) -> Result<(), MongoError> {
        let client = self.client()?;
        client.database(database_name).drop().await.map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.drop_database_failed").to_string(),
                e,
            )
        })?;
        Ok(())
    }

    async fn aggregate_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        pipeline: Vec<Document>,
    ) -> Result<Vec<Document>, MongoError> {
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        let mut cursor = collection.aggregate(pipeline).await.map_err(|e| {
            MongoError::command_with_source(t!("MongoConnection.aggregate_failed").to_string(), e)
        })?;

        let mut documents = Vec::new();
        while let Some(document) = cursor.try_next().await.map_err(|e| {
            MongoError::command_with_source(t!("MongoConnection.aggregate_failed").to_string(), e)
        })? {
            documents.push(document);
        }
        Ok(documents)
    }

    async fn list_indexes(
        &self,
        database_name: &str,
        collection_name: &str,
    ) -> Result<Vec<Document>, MongoError> {
        let client = self.client()?;
        let result = client
            .database(database_name)
            .run_command(doc! { "listIndexes": collection_name })
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.list_indexes_failed").to_string(),
                    e,
                )
            })?;
        let cursor = result.get_document("cursor").map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.list_indexes_failed").to_string(),
                e,
            )
        })?;
        let first_batch = cursor.get_array("firstBatch").map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.list_indexes_failed").to_string(),
                e,
            )
        })?;
        let mut indexes = Vec::new();
        for item in first_batch {
            if let Bson::Document(document) = item {
                indexes.push(document.clone());
            }
        }
        Ok(indexes)
    }

    async fn create_index(
        &self,
        database_name: &str,
        collection_name: &str,
        keys: Document,
        name: Option<String>,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        let mut index_doc = doc! { "key": keys };
        if let Some(name) = name {
            index_doc.insert("name", name);
        }
        client
            .database(database_name)
            .run_command(doc! { "createIndexes": collection_name, "indexes": [index_doc] })
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.create_index_failed").to_string(),
                    e,
                )
            })?;
        Ok(())
    }

    async fn drop_index(
        &self,
        database_name: &str,
        collection_name: &str,
        name: &str,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        client
            .database(database_name)
            .run_command(doc! { "dropIndexes": collection_name, "index": name })
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.drop_index_failed").to_string(),
                    e,
                )
            })?;
        Ok(())
    }

    async fn get_collection_validation(
        &self,
        database_name: &str,
        collection_name: &str,
    ) -> Result<Option<Document>, MongoError> {
        let client = self.client()?;
        let result = client
            .database(database_name)
            .run_command(doc! {
                "listCollections": 1,
                "filter": { "name": collection_name },
            })
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.get_validation_failed").to_string(),
                    e,
                )
            })?;
        let cursor = result.get_document("cursor").map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.get_validation_failed").to_string(),
                e,
            )
        })?;
        let first_batch = cursor.get_array("firstBatch").map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.get_validation_failed").to_string(),
                e,
            )
        })?;
        let Some(Bson::Document(collection_doc)) = first_batch.first() else {
            return Ok(None);
        };
        let options = collection_doc.get_document("options").ok();
        let validator = options
            .and_then(|doc| doc.get_document("validator").ok())
            .cloned();
        Ok(validator)
    }

    async fn update_collection_validation(
        &self,
        database_name: &str,
        collection_name: &str,
        validator: Option<Document>,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        let validator = validator.unwrap_or_default();
        client
            .database(database_name)
            .run_command(doc! {
                "collMod": collection_name,
                "validator": validator,
                "validationLevel": "moderate",
                "validationAction": "error",
            })
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.update_validation_failed").to_string(),
                    e,
                )
            })?;
        Ok(())
    }

    async fn find_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
        options: FindOptions,
    ) -> Result<Vec<Document>, MongoError> {
        info!(
            "[find_documents] db={}, collection={}, filter={:?}, options.limit={:?}, options.skip={:?}",
            database_name, collection_name, filter, options.limit, options.skip
        );

        // system.sessions 不支持标准 find 命令，需要通过 $listSessions 聚合管道查询
        if collection_name == "system.sessions" {
            return self
                .find_system_sessions(database_name, filter, &options)
                .await;
        }

        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        let actual_filter = filter.unwrap_or_else(Document::new);
        info!("[find_documents] 执行 find, actual_filter={actual_filter:?}");
        let mut cursor = collection
            .find(actual_filter)
            .with_options(options)
            .await
            .map_err(|e| {
                error!("[find_documents] find() 游标创建失败: {e}");
                MongoError::command_with_source(
                    t!("MongoConnection.find_documents_failed").to_string(),
                    e,
                )
            })?;
        info!("[find_documents] 游标创建成功，开始遍历文档");

        let mut documents = Vec::new();
        while let Some(document) = cursor.try_next().await.map_err(|e| {
            error!("[find_documents] try_next() 遍历失败: {e}");
            MongoError::command_with_source(
                t!("MongoConnection.find_documents_failed").to_string(),
                e,
            )
        })? {
            documents.push(document);
        }
        info!("[find_documents] 遍历完成，共 {} 条文档", documents.len());
        Ok(documents)
    }

    async fn count_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
    ) -> Result<i64, MongoError> {
        info!(
            "[count_documents] db={}, collection={}, filter={:?}",
            database_name, collection_name, filter
        );
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        let actual_filter = filter.unwrap_or_else(Document::new);
        collection
            .count_documents(actual_filter)
            .await
            .map_err(|e| {
                error!("[count_documents] 失败: {e}");
                MongoError::command_with_source(
                    t!("MongoConnection.count_documents_failed").to_string(),
                    e,
                )
            })
            .map(|count| {
                info!("[count_documents] 成功, count={count}");
                count as i64
            })
    }

    async fn insert_document(
        &self,
        database_name: &str,
        collection_name: &str,
        document: Document,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        collection.insert_one(document).await.map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.insert_document_failed").to_string(),
                e,
            )
        })?;
        Ok(())
    }

    async fn replace_document(
        &self,
        database_name: &str,
        collection_name: &str,
        id: Bson,
        document: Document,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        collection
            .replace_one(doc! { "_id": id }, document)
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.replace_document_failed").to_string(),
                    e,
                )
            })?;
        Ok(())
    }

    async fn delete_document(
        &self,
        database_name: &str,
        collection_name: &str,
        id: Bson,
    ) -> Result<(), MongoError> {
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        collection
            .delete_one(doc! { "_id": id })
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.delete_document_failed").to_string(),
                    e,
                )
            })?;
        Ok(())
    }

    async fn explain_find(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
        options: FindOptions,
    ) -> Result<Document, MongoError> {
        let client = self.client()?;
        let mut find_doc = doc! {
            "find": collection_name,
            "filter": filter.unwrap_or_else(Document::new),
        };

        if let Some(sort) = options.sort {
            find_doc.insert("sort", sort);
        }
        if let Some(projection) = options.projection {
            find_doc.insert("projection", projection);
        }
        if let Some(skip) = options.skip {
            find_doc.insert("skip", skip as i64);
        }
        if let Some(limit) = options.limit {
            find_doc.insert("limit", limit);
        }

        client
            .database(database_name)
            .run_command(doc! { "explain": find_doc })
            .await
            .map_err(|e| {
                MongoError::command_with_source(t!("MongoConnection.explain_failed").to_string(), e)
            })
    }
}
