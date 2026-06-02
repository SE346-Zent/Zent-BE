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
mod m20260510_100000_add_checklist;
mod m20260511_100000_drop_checklist_tables;
mod m20260512_100000_drop_overtimes;
mod m20260512_164300_add_fcm_token_and_outbox;
mod m20260513_100000_add_about_to_start_notified;
mod m20260514_000000_extend_outbox_records;
mod m20260515_000000_part_audit_log;
mod m20260514_153057_update_internet_time_image;
mod m20260516_100000_add_work_order_complaint;
mod m20260517_100000_add_escalation_level;
mod m20260518_100000_add_work_order_escalations;
mod m20260519_100000_add_work_order_appointment_changes;
mod m20260519_110000_add_work_order_pause;
mod m20260519_120000_drop_work_order_pause;
mod m20260520_100000_add_chat_tables;
mod m20260520_110000_add_avatar_and_chat_image_links;
mod m20260521_100000_add_wo_id_to_chat_rooms;
mod m20260522_100000_add_performance_indexes;
mod m20260523_100000_add_chat_room_id_to_work_orders;
mod m20260524_100000_add_ratings_and_part_wo_number;
mod m20260528_100000_decouple_inventory_fks;
mod m20260528_110000_add_warranty_status_lut;
mod m20260530_100000_add_login_audit_log;
mod m20260530_100001_backfill_new_part_form_status;
mod m20260530_100002_rename_part_audit_log;
mod m20260531_100000_add_registered_devices;
mod m20260531_110000_rename_city_to_ward;
mod m20260531_120000_drop_product_fk_from_registered_devices;
mod m20260601_100000_add_cancel_reason;
mod m20260602_100000_add_new_part_request_statuses;


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
            Box::new(m20260510_100000_add_checklist::Migration),
            Box::new(m20260511_100000_drop_checklist_tables::Migration),
            Box::new(m20260512_100000_drop_overtimes::Migration),
            Box::new(m20260512_164300_add_fcm_token_and_outbox::Migration),
            Box::new(m20260513_100000_add_about_to_start_notified::Migration),
            Box::new(m20260514_000000_extend_outbox_records::Migration),
            Box::new(m20260515_000000_part_audit_log::Migration),
            Box::new(m20260514_153057_update_internet_time_image::Migration),
            Box::new(m20260516_100000_add_work_order_complaint::Migration),
            Box::new(m20260517_100000_add_escalation_level::Migration),
            Box::new(m20260518_100000_add_work_order_escalations::Migration),
            Box::new(m20260519_100000_add_work_order_appointment_changes::Migration),
            Box::new(m20260519_110000_add_work_order_pause::Migration),
            Box::new(m20260519_120000_drop_work_order_pause::Migration),
            Box::new(m20260520_100000_add_chat_tables::Migration),
            Box::new(m20260520_110000_add_avatar_and_chat_image_links::Migration),
            Box::new(m20260521_100000_add_wo_id_to_chat_rooms::Migration),
            Box::new(m20260522_100000_add_performance_indexes::Migration),
            Box::new(m20260523_100000_add_chat_room_id_to_work_orders::Migration),
            Box::new(m20260524_100000_add_ratings_and_part_wo_number::Migration),
            Box::new(m20260528_100000_decouple_inventory_fks::Migration),
            Box::new(m20260528_110000_add_warranty_status_lut::Migration),
            Box::new(m20260530_100000_add_login_audit_log::Migration),
            Box::new(m20260530_100001_backfill_new_part_form_status::Migration),
            Box::new(m20260530_100002_rename_part_audit_log::Migration),
            Box::new(m20260531_100000_add_registered_devices::Migration),
            Box::new(m20260531_110000_rename_city_to_ward::Migration),
            Box::new(m20260531_120000_drop_product_fk_from_registered_devices::Migration),
            Box::new(m20260601_100000_add_cancel_reason::Migration),
            Box::new(m20260602_100000_add_new_part_request_statuses::Migration),
        ]
    }
}
