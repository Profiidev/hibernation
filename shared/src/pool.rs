use tokio::task::{JoinError, JoinSet};

pub struct FuturePool<O, F>
where
  F: Future<Output = O> + Send + 'static,
  O: Send + 'static,
{
  futures: Vec<F>,
}

impl<O, F> FuturePool<O, F>
where
  F: Future<Output = O> + Send + 'static,
  O: Send + 'static,
{
  pub fn new(futures: Vec<F>) -> Self {
    FuturePool { futures }
  }

  pub async fn run(self) -> Vec<Result<O, JoinError>> {
    let mut left = self.futures;
    let mut running = JoinSet::new();

    for _ in 0..20 {
      if let Some(future) = left.pop() {
        running.spawn(future);
      }
    }

    let mut results = Vec::new();
    while let Some(result) = running.join_next().await {
      results.push(result);
      if let Some(future) = left.pop() {
        running.spawn(future);
      }
    }

    results
  }
}
