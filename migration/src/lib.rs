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
mod m20260505_215000_overtimes_and_phases;
mod m20260506_090000_update_reject_form;
mod m20260506_100000_geofencing;
mod m20260507_012900_rename_image_url;
mod m20260507_102900_update_new_part_forms_and_reject_forms;
mod m20260508_090000_refactor_image_links;
mod m20260508_100000_make_approver_nullable;
mod m20260507_122648_rename_signature_field;
mod m20260509_100000_refactor_state_history;


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
            Box::new(m20260505_215000_overtimes_and_phases::Migration),
            Box::new(m20260506_090000_update_reject_form::Migration),
            Box::new(m20260506_100000_geofencing::Migration),
            Box::new(m20260507_012900_rename_image_url::Migration),
            Box::new(m20260507_102900_update_new_part_forms_and_reject_forms::Migration),
            Box::new(m20260508_090000_refactor_image_links::Migration),
            Box::new(m20260508_100000_make_approver_nullable::Migration),
            Box::new(m20260507_122648_rename_signature_field::Migration),
            Box::new(m20260509_100000_refactor_state_history::Migration),
        ]
    }
}
