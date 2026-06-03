use anyhow::Result;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use uuid::Uuid;
use zent_be::entities::{chat_rooms, chat_room_members, users};

/// Seed chat rooms between admins and technicians (same province), and
/// between super-admins and all admins. Idempotent — safe to run multiple
/// times without creating duplicates.
pub async fn seed_admin_technician_chat_rooms(db: &DatabaseConnection) -> Result<u32> {
    let roles = zent_be::entities::roles::Entity::find().all(db).await?;
    let role_map: std::collections::HashMap<String, i32> =
        roles.iter().map(|r| (r.name.clone(), r.id)).collect();

    let admin_role_id = role_map.get("Admin").copied();
    let tech_role_id = role_map.get("Technician").copied();
    let sa_role_id = role_map.get("SuperAdmin").copied();

    if admin_role_id.is_none() {
        println!("  Skipping: Admin role not found");
        return Ok(0);
    }
    let admin_role_id = admin_role_id.unwrap();

    let all_users = users::Entity::find()
        .filter(users::Column::DeletedAt.is_null())
        .all(db)
        .await?;

    let admins: Vec<&users::Model> = all_users
        .iter()
        .filter(|u| u.role_id == admin_role_id)
        .collect();

    let technicians: Vec<&users::Model> = all_users
        .iter()
        .filter(|u| Some(u.role_id) == tech_role_id && u.province.is_some())
        .collect();

    let super_admins: Vec<&users::Model> = all_users
        .iter()
        .filter(|u| Some(u.role_id) == sa_role_id)
        .collect();

    // Pre-fetch all existing room memberships for idempotency
    let all_members = chat_room_members::Entity::find()
        .filter(chat_room_members::Column::DeletedAt.is_null())
        .all(db)
        .await?;

    let mut existing_pairs: std::collections::HashSet<(Uuid, Uuid)> =
        std::collections::HashSet::new();
    let mut room_members_map: std::collections::HashMap<Uuid, Vec<Uuid>> =
        std::collections::HashMap::new();
    for m in &all_members {
        room_members_map.entry(m.room_id).or_default().push(m.user_id);
    }
    for members in room_members_map.values() {
        if members.len() >= 2 {
            // For 2-member rooms, track the pair
            let mut sorted = members.clone();
            sorted.sort();
            for i in 0..sorted.len() {
                for j in (i + 1)..sorted.len() {
                    existing_pairs.insert((sorted[i], sorted[j]));
                }
            }
        }
    }

    let now = chrono::Utc::now();
    let mut rooms_created: u32 = 0;

    // 1. Admin ↔ Technician (same province)
    if !admins.is_empty() && !technicians.is_empty() {
        for admin in &admins {
            let admin_province = match admin.province.as_ref() {
                Some(p) => p,
                None => continue,
            };
            for tech in &technicians {
                let tech_province = tech.province.as_ref().unwrap();
                if admin_province != tech_province {
                    continue;
                }
                let pair_key = normalize_pair(admin.id, tech.id);
                if existing_pairs.contains(&pair_key) {
                    continue;
                }
                create_room(db, admin.id, tech.id, &format!("{} & {}", admin.full_name, tech.full_name), now).await?;
                existing_pairs.insert(pair_key);
                rooms_created += 1;
            }
        }
        println!("  Admin-Technician rooms created: {}", rooms_created);
    }

    // 2. SuperAdmin ↔ All admins (no province filter)
    let mut sa_rooms: u32 = 0;
    if !super_admins.is_empty() && !admins.is_empty() {
        for sa in &super_admins {
            for admin in &admins {
                if sa.id == admin.id {
                    continue;
                }
                let pair_key = normalize_pair(sa.id, admin.id);
                if existing_pairs.contains(&pair_key) {
                    continue;
                }
                create_room(db, sa.id, admin.id, &format!("{} & {}", sa.full_name, admin.full_name), now).await?;
                existing_pairs.insert(pair_key);
                sa_rooms += 1;
            }
        }
        println!("  SuperAdmin-Admin rooms created: {}", sa_rooms);
    }

    Ok(rooms_created + sa_rooms)
}

fn normalize_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b { (a, b) } else { (b, a) }
}

async fn create_room(
    db: &DatabaseConnection,
    user_a: Uuid,
    user_b: Uuid,
    room_name: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let room_id = Uuid::new_v4();

    let room = chat_rooms::ActiveModel {
        id: Set(room_id),
        room_name: Set(room_name.to_string()),
        created_by: Set(user_a),
        work_order_id: Set(None),
        created_at: Set(now),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
    };
    room.insert(db).await?;

    let member_a = chat_room_members::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(user_a),
        created_at: Set(now),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
    };
    member_a.insert(db).await?;

    let member_b = chat_room_members::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(user_b),
        created_at: Set(now),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
    };
    member_b.insert(db).await?;

    Ok(())
}
