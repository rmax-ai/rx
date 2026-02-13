use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::event::Event;

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn load(&self, goal_id: &str) -> Result<Vec<Event>>;
    async fn append_event(&self, goal_id: &str, event: Event) -> Result<()>;
    async fn list_goals(&self) -> Result<Vec<(String, String)>>;
}

pub struct InMemoryStateStore {
    events: Arc<Mutex<Vec<Event>>>,
}

impl InMemoryStateStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn load(&self, _goal_id: &str) -> Result<Vec<Event>> {
        let events = self.events.lock().await;
        Ok(events.clone())
    }

    async fn append_event(&self, _goal_id: &str, event: Event) -> Result<()> {
        let mut events = self.events.lock().await;
        events.push(event.clone());

        Ok(())
    }
    
    async fn list_goals(&self) -> Result<Vec<(String, String)>> {
        // InMemory version of list_goals
        Ok(Vec::new())
    }
}
