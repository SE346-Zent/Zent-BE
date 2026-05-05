pub use sea_orm_migration::prelude::*;

mod m20260305_081157_login_signup_migration;
mod m20260329_174031_device;
mod m20260330_104147_work_order_update;
mod m20260331_063024_part;
mod m20260408_113539_parts_update;
mod m20260426_050036_policy;
mod m20260504_100000_add_wo_number_index;
mod m20260504_110000_add_user_state;
mod m20260505_035658_update_field;


pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260305_081157_login_signup_migration::Migration),
            Box::new(m20260329_174031_device::Migration),
            Box::new(m20260330_104147_work_order_update::Migration),
            Box::new(m20260331_063024_part::Migration),
            Box::new(m20260408_113539_parts_update::Migration),
            Box::new(m20260426_050036_policy::Migration),
            Box::new(m20260504_100000_add_wo_number_index::Migration),
            Box::new(m20260504_110000_add_user_state::Migration),
            Box::new(m20260505_035658_update_field::Migration),
        ]
    }
}
