use async_trait::async_trait;
use anyhow::Result;
use crate::event::Event;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>>;
    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()>;
}
