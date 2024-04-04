use super::kiwi::KiwiScraperStats;

#[derive(Eq, PartialEq, Clone)]
pub enum ScraperStatus {
    Running,
    Stopped,
}

#[async_trait::async_trait]
pub trait SDRScraper {
    async fn start(&mut self) -> anyhow::Result<()>;
    async fn stop(&mut self) -> anyhow::Result<()>;
    fn status(&self) -> ScraperStatus;
    fn name(&self) -> &str;
    fn get_stats(&self) -> KiwiScraperStats;
}
