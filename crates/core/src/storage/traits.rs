use anyhow::Result;
use gpui::SharedString;

pub trait Entity: Send + Sync {
    fn id(&self) -> Option<i64>;

    fn created_at(&self) -> i64;

    fn updated_at(&self) -> i64;
}

pub trait Repository: Send + Sync {
    type Entity: Entity;

    fn entity_type(&self) -> SharedString;

    fn insert(&self, item: &mut Self::Entity) -> Result<i64>;

    fn update(&self, item: &Self::Entity) -> Result<()>;

    fn delete(&self, id: i64) -> Result<()>;

    fn get(&self, id: i64) -> Result<Option<Self::Entity>>;

    fn list(&self) -> Result<Vec<Self::Entity>>;

    fn count(&self) -> Result<i64>;

    fn exists(&self, id: i64) -> Result<bool>;
}
