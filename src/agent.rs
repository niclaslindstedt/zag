use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Agent {
    fn name(&self) -> &str;

    fn default_model() -> &'static str
    where
        Self: Sized;

    fn system_prompt(&self) -> &str;

    fn set_system_prompt(&mut self, prompt: String);

    fn set_model(&mut self, model: String);

    fn set_root(&mut self, root: String);

    fn set_skip_permissions(&mut self, skip: bool);

    async fn run(&self, prompt: Option<&str>) -> Result<()>;

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()>;

    async fn cleanup(&self) -> Result<()>;
}
