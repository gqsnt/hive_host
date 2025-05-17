#[cfg(feature = "ssr")]
pub mod refresh_server_csrf;
#[cfg(feature = "ssr")]
pub mod ssr {
    use async_trait::async_trait;
    use chrono::Timelike;
    use sqlx::types::chrono::Local;
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    use tokio::time::Duration;
    use tokio::time::Instant;

    #[async_trait]
    pub trait Task: Send {
        async fn execute(&self);

        fn next_execution(&self) -> Instant;

        fn update_schedule(&mut self);

        fn is_running(&self) -> bool;

        fn set_running(&self, running: bool);

        fn clone_box(&self) -> Box<dyn Task>;

        fn name(&self) -> &'static str;

        fn allow_concurrent(&self) -> bool;
    }

    impl Clone for Box<dyn Task> {
        fn clone(&self) -> Box<dyn Task> {
            self.clone_box()
        }
    }

    #[derive(Default)]
    pub struct TaskDirector {
        tasks: BinaryHeap<Reverse<ScheduledTask>>,
    }

    struct ScheduledTask {
        next_run: Instant,
        task: Box<dyn Task>,
    }

    impl PartialEq for ScheduledTask {
        fn eq(&self, other: &Self) -> bool {
            self.next_run.eq(&other.next_run)
        }
    }

    impl Eq for ScheduledTask {}

    impl PartialOrd for ScheduledTask {
        #[allow(clippy::non_canonical_partial_ord_impl)]
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.next_run.partial_cmp(&other.next_run)
        }
    }

    impl Ord for ScheduledTask {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.next_run.cmp(&self.next_run)
        }
    }

    impl TaskDirector {
        pub fn add_task<T: Task + 'static>(&mut self, task: T) {
            let next_run = task.next_execution();
            self.tasks.push(Reverse(ScheduledTask {
                next_run,
                task: Box::new(task),
            }));
        }

        pub async fn run(mut self) {
            loop {
                if let Some(Reverse(mut scheduled_task)) = self.tasks.pop() {
                    let now = Instant::now();
                    if scheduled_task.next_run <= now {
                        if !scheduled_task.task.is_running()
                            || scheduled_task.task.allow_concurrent()
                        {
                            scheduled_task.task.set_running(true);

                            let task_clone = scheduled_task.task.clone();
                            tokio::spawn(async move {
                                let _guard = RunningGuard::new(task_clone.clone());
                                task_clone.execute().await;
                            });
                        }

                        scheduled_task.task.update_schedule();
                        scheduled_task.next_run = scheduled_task.task.next_execution();
                        self.tasks.push(Reverse(scheduled_task));
                    } else {
                        let sleep_duration = scheduled_task.next_run - now;
                        tokio::time::sleep(sleep_duration).await;
                        self.tasks.push(Reverse(scheduled_task));
                    }
                } else {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    struct RunningGuard {
        task: Box<dyn Task>,
    }

    impl RunningGuard {
        fn new(task: Box<dyn Task>) -> Self {
            Self { task }
        }
    }

    impl Drop for RunningGuard {
        fn drop(&mut self) {
            self.task.set_running(false);
        }
    }

    pub fn calculate_next_run_to_fixed_start_hour(start_hour: u32) -> Instant {
        let now = Local::now();
        let target_time = if now.hour() >= start_hour {
            (now + Duration::from_secs(3600 * 24))
                .with_hour(start_hour)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
        } else {
            now.with_hour(start_hour)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
        };
        Instant::now() + (target_time - now).to_std().unwrap()
    }
}
