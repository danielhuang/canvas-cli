use std::sync::Mutex;

use futures::Future;
use generational_arena::Arena;
use indicatif::{ProgressBar, ProgressStyle};

pub struct Progress {
    bar: ProgressBar,
    messages: Mutex<Arena<String>>,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            bar: {
                let pb = ProgressBar::new(0).with_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] {wide_msg} ({len})")
                        .progress_chars("#>-")
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
                );
                pb.enable_steady_tick(200);
                pb
            },
            messages: Mutex::new(Arena::new()),
        }
    }

    fn update(&self) {
        let messages: Vec<_> = self
            .messages
            .lock()
            .unwrap()
            .iter()
            .map(|x| x.1)
            .cloned()
            .collect();
        self.bar.set_message(&messages.join(", "));
        self.bar.set_length(messages.len() as _);
    }

    pub async fn wrap<T>(&self, msg: &str, f: impl Future<Output = T>) -> T {
        let i = self.messages.lock().unwrap().insert(msg.into());
        self.update();
        let o = f.await;
        self.messages.lock().unwrap().remove(i);
        self.update();
        o
    }

    pub fn finish(self) {
        self.bar.finish_and_clear();
    }
}
