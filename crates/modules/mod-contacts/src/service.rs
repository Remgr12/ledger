use chrono::Utc;
use erp_kernel::ModuleContext;
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::models::{Contact, ContactType};

pub struct ContactsService;

impl ContactsService {
    pub fn create_contact(
        ctx: &ModuleContext,
        contact_type: ContactType,
        name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        tax_id: Option<&str>,
        address: Option<&str>,
    ) -> Result<Contact, erp_kernel::ErpKernelError> {
        let contact_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO contacts (id, contact_type, name, email, phone, tax_id, address, is_active, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)
                "#,
                params![
                    contact_id,
                    contact_type.as_str(),
                    name,
                    email,
                    phone,
                    tax_id,
                    address,
                    now.to_rfc3339()
                ],
            )?;
        }

        let contact = Contact {
            id: contact_id.clone(),
            contact_type,
            name: name.to_string(),
            email: email.map(String::from),
            phone: phone.map(String::from),
            tax_id: tax_id.map(String::from),
            address: address.map(String::from),
            is_active: true,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "ContactCreated",
            "Contact",
            &contact_id,
            serde_json::to_value(&contact).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(contact)
    }

    pub fn get_contact_by_id(conn: &Connection, id: &str) -> Result<Contact, rusqlite::Error> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, contact_type, name, email, phone, tax_id, address, is_active, created_at
            FROM contacts
            WHERE id = ?1
            "#,
        )?;
        stmt.query_row(params![id], Self::map_contact_row)
    }

    pub fn list_contacts(conn: &Connection) -> Result<Vec<Contact>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, contact_type, name, email, phone, tax_id, address, is_active, created_at
            FROM contacts
            WHERE is_active = 1
            ORDER BY name ASC
            "#,
        )?;
        let rows = stmt.query_map([], Self::map_contact_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    fn map_contact_row(row: &Row<'_>) -> Result<Contact, rusqlite::Error> {
        let id: String = row.get(0)?;
        let contact_type_str: String = row.get(1)?;
        let name: String = row.get(2)?;
        let email: Option<String> = row.get(3)?;
        let phone: Option<String> = row.get(4)?;
        let tax_id: Option<String> = row.get(5)?;
        let address: Option<String> = row.get(6)?;
        let is_active_int: i32 = row.get(7)?;
        let created_at_str: String = row.get(8)?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        use std::str::FromStr;
        Ok(Contact {
            id,
            contact_type: ContactType::from_str(&contact_type_str).unwrap_or(ContactType::Both),
            name,
            email,
            phone,
            tax_id,
            address,
            is_active: is_active_int != 0,
            created_at,
        })
    }
}
