use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::work_order_symptoms;
use chrono::Utc;

pub const WORK_ORDER_SYMPTOMS: &[&str] = &[
    "Active Noise Cancelling(ANC)",
    "Backpack",
    "Bluetooth",
    "Case",
    "Charger",
    "External Hot Spot Issue",
    "External Keyboard",
    "External Mouse",
    "External Storage(USB/SSD/etc)",
    "Glasses",
    "Headset",
    "Kit(Mouse and Keyboard)",
    "MousePad",
    "Other",
    "PC Port not working properly",
    "Pen",
    "Printer",
    "Web Camera",
    "Audio",
    "Battery",
    "Boot issue",
    "Branding",
    "Camera",
    "Charging",
    "Covers",
    "Display",
    "Dock",
    "Drive (SSD / HDD)",
    "External Display",
    "Fan",
    "Fingerprint",
    "Keyboards",
    "Network",
    "No Post",
    "No Power",
    "Noise",
    "Non Technical",
    "Operating System (OS)",
    "Performance",
    "Physical Damage (CID)",
    "Physical Damage (Not CID)",
    "Pointing Devices",
    "Power Button",
    "Safety issue",
    "Smart card reader",
    "Smart Collab",
    "Software",
    "USB Port",
    "Other"
];

pub async fn seed_work_order_symptoms(db: &DatabaseConnection) -> Result<HashMap<String, i32>> 
{
    let mut map: HashMap<String, i32> = HashMap::new();
    let now = Utc::now();
    for &name in WORK_ORDER_SYMPTOMS
    {
        let existing = work_order_symptoms::Entity::find()
            .filter(work_order_symptoms::Column::SymptomNames.eq(name))
            .one(db)
            .await?;

        let id: i32 = match existing {
            Some(s) => {
                println!("Work order symptom '{}' already exists (id={})", name, s.id);
                s.id
            }
            None => {
                let inserted = work_order_symptoms::ActiveModel 
                {
                    symptom_names: Set(name.to_string()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("Created work order stymptoms '{}' (id={})", name, inserted.id);
                inserted.id 
            }
        };

        map.insert(name.to_string(), id);
    }
    Ok(map)
}