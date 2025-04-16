use crate::ssr::CsrfServer;
use crate::tasks::ssr::{calculate_next_run_to_fixed_start_hour, Task};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::time::Instant;

pub struct RefreshServerCsrf {
    pub csrf: Arc<CsrfServer>,
    pub start_hour: u32,
    pub next_run: Instant,
    pub running: Arc<AtomicBool>,
}

impl RefreshServerCsrf {
    pub fn new(csrf: Arc<CsrfServer>, start_hour: u32, on_startup: bool) -> Self {
        let next_run = if on_startup {
            tokio::time::Instant::now()
        } else {
            calculate_next_run_to_fixed_start_hour(start_hour)
        };
        Self {
            csrf,
            start_hour,
            next_run,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}


#[async_trait]
impl Task for RefreshServerCsrf {
    async fn execute(&self){
        self.csrf.refresh();
    }


    fn next_execution(&self) -> Instant {
        self.next_run
    }

    fn update_schedule(&mut self) {
        self.next_run = calculate_next_run_to_fixed_start_hour(self.start_hour);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }

    fn clone_box(&self) -> Box<dyn Task> {
        Box::new(Self {
            csrf: self.csrf.clone(),
            start_hour: self.start_hour,
            next_run: self.next_run,
            running: self.running.clone(),
        })
    }

    fn name(&self) -> &'static str {
        "Refresh Server Csrf"
    }

    fn allow_concurrent(&self) -> bool {
        false
    }
}
