use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;
use zent_be::entities::checklist_items;
use chrono::Utc;

pub const CHECKLIST_ITEM_NAMES: &[&str] = &[
    "Lan Port/Wifi/WWAN/Bluetooth",
    "LCD Lid open/close degree check no flickering",
    "Speaker/Audio jack/Webcam/Microphone",
    "Update MTM/SN/UUID/Product Name",
    "AC adapter/battery charging",
    "LCD touch/rotate/flip test",
    "No part Replacement",
    "Update latest BIOS/FW/Driver",
    "USB & I/O Ports/SD Slot/Sim Slot",
];

pub async fn seed_checklist_items(db: &DatabaseConnection) -> Result<HashMap<String, i32>> {
    let mut map: HashMap<String, i32> = HashMap::new();
    let now = Utc::now();

    for &name in CHECKLIST_ITEM_NAMES {
        let existing = checklist_items::Entity::find()
            .filter(checklist_items::Column::Name.eq(name))
            .one(db)
            .await?;

        let id: i32 = match existing {
            Some(item) => {
                println!("  Checklist item '{}' already exists (id={})", name, item.id);
                item.id
            }
            None => {
                let inserted = checklist_items::ActiveModel {
                    name: Set(name.to_string()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                println!("  Created checklist item '{}' (id={})", name, inserted.id);
                inserted.id
            }
        };

        map.insert(name.to_string(), id);
    }

    Ok(map)
}
