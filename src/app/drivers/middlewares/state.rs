use crate::utils::{ 
    db::DbPool, 
    di::DiContainer 
};

#[derive(Clone)]
pub struct AppState {
    pub di_container: DiContainer,
}

impl AppState {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            di_container: DiContainer::new(&db_pool),
        }
    }
}
