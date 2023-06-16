use chrono::{Utc, DateTime};
use once_cell::sync::Lazy;

struct OldDependency {
  name: String,
  version: String,
  latest_release: DateTime<Utc>,
  locked_release: DateTime<Utc>
}

// Use same date for all
static DATE: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

