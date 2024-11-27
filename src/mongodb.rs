use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
   pub user_id: i64,
   pub anime_id: i32,
   pub anime_name: String,
   pub last_episode: i32,
}