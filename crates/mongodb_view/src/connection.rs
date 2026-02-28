//! MongoDB 连接实现

use crate::types::{MongoConnectionConfig, MongoError};
use async_trait::async_trait;
use futures_util::stream::TryStreamExt;
use mongodb::Client;
use mongodb::bson::{Bson, Document, doc};
use mongodb::options::FindOptions;
use rust_i18n::t;

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

        let client = Client::with_uri_str(&self.config.connection_string)
            .await
            .map_err(|e| {
                MongoError::connection_with_source(
                    t!("MongoConnection.connect_failed").to_string(),
                    e,
                )
            })?;
        self.client = Some(client);
        Ok(())
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
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        let mut cursor = collection
            .find(filter.unwrap_or_else(Document::new))
            .with_options(options)
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.find_documents_failed").to_string(),
                    e,
                )
            })?;

        let mut documents = Vec::new();
        while let Some(document) = cursor.try_next().await.map_err(|e| {
            MongoError::command_with_source(
                t!("MongoConnection.find_documents_failed").to_string(),
                e,
            )
        })? {
            documents.push(document);
        }
        Ok(documents)
    }

    async fn count_documents(
        &self,
        database_name: &str,
        collection_name: &str,
        filter: Option<Document>,
    ) -> Result<i64, MongoError> {
        let client = self.client()?;
        let collection = client
            .database(database_name)
            .collection::<Document>(collection_name);
        collection
            .count_documents(filter.unwrap_or_else(Document::new))
            .await
            .map_err(|e| {
                MongoError::command_with_source(
                    t!("MongoConnection.count_documents_failed").to_string(),
                    e,
                )
            })
            .map(|count| count as i64)
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
