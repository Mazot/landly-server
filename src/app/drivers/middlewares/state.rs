use crate::utils::{db::DbPool, di::DiContainer, redis::establish_connection};

#[derive(Clone)]
pub struct AppState {
    pub di_container: DiContainer,
}

impl AppState {
    pub fn new(db_pool: DbPool) -> Self {
        let redis_pool = establish_connection().ok();

        Self {
            di_container: DiContainer::new(&db_pool, redis_pool),
        }
    }
}
