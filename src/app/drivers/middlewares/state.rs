use crate::utils::{
    db::DbPool,
    di::DiContainer,
    redis::establish_connection
};

#[derive(Clone)]
pub struct AppState {
    pub di_container: DiContainer,
}

impl AppState {
    pub fn new(db_pool: DbPool) -> Self {
        let redis_pool = match establish_connection() {
            Ok(pool) => Some(pool),
            Err(_) => None
        };

        Self {
            di_container: DiContainer::new(&db_pool, redis_pool),
        }
    }
}
