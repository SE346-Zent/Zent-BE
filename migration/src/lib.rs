pub use sea_orm_migration::prelude::*;

mod m20260305_081157_login_signup_migration;
mod m20260318_051423_work_orders;
mod m20260329_174031_device;
mod m20260330_104147_work_order_update;



pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260305_081157_login_signup_migration::Migration),
            Box::new(m20260318_051423_work_orders::Migration),
            Box::new(m20260329_174031_device::Migration),
            Box::new(m20260330_104147_work_order_update::Migration),
        ]
    }
}
