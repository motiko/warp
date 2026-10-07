pub mod model;
pub mod schema;
pub mod workspace_metadata;

#[cfg(feature = "local_fs")]
pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::embed_migrations!("migrations");
