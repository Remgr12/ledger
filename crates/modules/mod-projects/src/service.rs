use chrono::Utc;
use erp_kernel::ModuleContext;
use mod_invoicing::InvoicingService;
use rusqlite::{params, Connection, Row};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::errors::ProjectsError;
use crate::models::{Project, ProjectStatus, ProjectTask, TimesheetEntry};

pub struct ProjectsService;

impl ProjectsService {
    /// Create a new project for a customer
    pub fn create_project(
        ctx: &ModuleContext,
        name: &str,
        code: &str,
        customer_id: Option<&str>,
        billing_rate_hourly: Decimal,
        estimated_hours: Decimal,
    ) -> Result<Project, ProjectsError> {
        let project_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO projects (
                    id, name, code, customer_id, status, billing_rate_hourly,
                    estimated_hours, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    project_id,
                    name,
                    code,
                    customer_id,
                    ProjectStatus::Open.as_str(),
                    billing_rate_hourly.to_string(),
                    estimated_hours.to_string(),
                    now.to_rfc3339()
                ],
            )?;
        }

        let project = Project {
            id: project_id.clone(),
            name: name.to_string(),
            code: code.to_string(),
            customer_id: customer_id.map(String::from),
            status: ProjectStatus::Open,
            billing_rate_hourly,
            estimated_hours,
            created_at: now,
        };

        let _ = ctx.record_and_publish_event(
            "ProjectCreated",
            "Project",
            &project_id,
            serde_json::to_value(&project).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(project)
    }

    /// Create a task under a project
    pub fn create_task(
        ctx: &ModuleContext,
        project_id: &str,
        title: &str,
        priority: &str,
        estimated_hours: Decimal,
    ) -> Result<ProjectTask, ProjectsError> {
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO project_tasks (
                    id, project_id, title, status, priority, estimated_hours, created_at
                )
                VALUES (?1, ?2, ?3, 'Todo', ?4, ?5, ?6)
                "#,
                params![
                    task_id,
                    project_id,
                    title,
                    priority,
                    estimated_hours.to_string(),
                    now.to_rfc3339()
                ],
            )?;
        }

        let task = ProjectTask {
            id: task_id,
            project_id: project_id.to_string(),
            title: title.to_string(),
            status: "Todo".to_string(),
            priority: priority.to_string(),
            estimated_hours,
            created_at: now,
        };

        Ok(task)
    }

    /// Log hours and activities against a project/task
    #[allow(clippy::too_many_arguments)]
    pub fn record_timesheet(
        ctx: &ModuleContext,
        project_id: &str,
        task_id: Option<&str>,
        employee_or_user: &str,
        activity_date: &str,
        hours: Decimal,
        hourly_rate_override: Option<Decimal>,
        is_billable: bool,
        description: &str,
    ) -> Result<TimesheetEntry, ProjectsError> {
        if hours <= Decimal::ZERO {
            return Err(ProjectsError::InvalidTransaction(
                "Timesheet hours must be greater than zero".into(),
            ));
        }

        let project = {
            let conn = ctx.db.conn();
            Self::get_project_by_id(&conn, project_id)?
        };

        let effective_rate = hourly_rate_override.unwrap_or(project.billing_rate_hourly);
        let billing_amount = if is_billable {
            hours * effective_rate
        } else {
            Decimal::ZERO
        };

        let ts_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        {
            let conn = ctx.db.conn();
            conn.execute(
                r#"
                INSERT INTO timesheets (
                    id, project_id, task_id, employee_or_user, activity_date,
                    hours, hourly_rate, billing_amount, is_billable, is_invoiced,
                    invoice_id, description, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, NULL, ?10, ?11)
                "#,
                params![
                    ts_id,
                    project_id,
                    task_id,
                    employee_or_user,
                    activity_date,
                    hours.to_string(),
                    effective_rate.to_string(),
                    billing_amount.to_string(),
                    if is_billable { 1 } else { 0 },
                    description,
                    now.to_rfc3339()
                ],
            )?;
        }

        let entry = TimesheetEntry {
            id: ts_id,
            project_id: project_id.to_string(),
            task_id: task_id.map(String::from),
            employee_or_user: employee_or_user.to_string(),
            activity_date: activity_date.to_string(),
            hours,
            hourly_rate: effective_rate,
            billing_amount,
            is_billable,
            is_invoiced: false,
            invoice_id: None,
            description: description.to_string(),
            created_at: now,
        };

        Ok(entry)
    }

    /// Convert unbilled timesheets into an official Sales Invoice in `mod-invoicing`
    pub fn invoice_unbilled_timesheets(
        ctx: &ModuleContext,
        project_id: &str,
        invoice_number: &str,
        tax_rate: Decimal,
    ) -> Result<mod_invoicing::Invoice, ProjectsError> {
        let (project, unbilled) = {
            let conn = ctx.db.conn();
            let p = Self::get_project_by_id(&conn, project_id)?;
            let u = Self::list_unbilled_timesheets(&conn, project_id)?;
            (p, u)
        };

        let customer_id = project.customer_id.ok_or_else(|| {
            ProjectsError::ProjectHasNoCustomer(project.name.clone())
        })?;

        if unbilled.is_empty() {
            return Err(ProjectsError::NoUnbilledTimesheets(project_id.to_string()));
        }

        let invoice_lines: Vec<(Option<String>, String, Decimal, Decimal, Decimal)> = unbilled
            .iter()
            .map(|ts| {
                let desc = format!("{} — {}", ts.activity_date, ts.description);
                (None, desc, ts.hours, ts.hourly_rate, tax_rate)
            })
            .collect();

        let invoice = InvoicingService::create_invoice(
            ctx,
            invoice_number,
            &customer_id,
            invoice_lines,
            Some(&format!("Project Billing: {}", project.name)),
        )?;

        // Mark timesheets as invoiced
        {
            let mut conn = ctx.db.conn();
            let tx = conn.transaction()?;
            for ts in &unbilled {
                tx.execute(
                    "UPDATE timesheets SET is_invoiced = 1, invoice_id = ?1 WHERE id = ?2",
                    params![invoice.id, ts.id],
                )?;
            }
            tx.commit()?;
        }

        Ok(invoice)
    }

    pub fn get_project_by_id(conn: &Connection, id: &str) -> Result<Project, ProjectsError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, code, customer_id, status, billing_rate_hourly,
                   estimated_hours, created_at
            FROM projects
            WHERE id = ?1
            "#,
        )?;

        stmt.query_row(params![id], Self::map_project_row)
            .map_err(|_| ProjectsError::ProjectNotFound(id.to_string()))
    }

    pub fn list_projects(conn: &Connection) -> Result<Vec<Project>, ProjectsError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, code, customer_id, status, billing_rate_hourly,
                   estimated_hours, created_at
            FROM projects
            ORDER BY name ASC
            "#,
        )?;

        let rows = stmt.query_map([], Self::map_project_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn list_unbilled_timesheets(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Vec<TimesheetEntry>, ProjectsError> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, task_id, employee_or_user, activity_date,
                   hours, hourly_rate, billing_amount, is_billable, is_invoiced,
                   invoice_id, description, created_at
            FROM timesheets
            WHERE project_id = ?1 AND is_billable = 1 AND is_invoiced = 0
            ORDER BY activity_date ASC
            "#,
        )?;

        let rows = stmt.query_map(params![project_id], Self::map_timesheet_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Update status of a project task (e.g. Backlog -> In Progress -> Review -> Done)
    pub fn update_task_status(
        ctx: &ModuleContext,
        task_id: &str,
        new_status: &str,
    ) -> Result<ProjectTask, ProjectsError> {
        let task = {
            let conn = ctx.db.conn();
            conn.execute(
                "UPDATE project_tasks SET status = ?1 WHERE id = ?2",
                params![new_status, task_id],
            )?;

            let mut stmt = conn.prepare(
                "SELECT id, project_id, title, status, priority, estimated_hours, created_at FROM project_tasks WHERE id = ?1"
            )?;

            stmt.query_row(params![task_id], |row| {
                let id: String = row.get(0)?;
                let project_id: String = row.get(1)?;
                let title: String = row.get(2)?;
                let status: String = row.get(3)?;
                let priority: String = row.get(4)?;
                let est_str: String = row.get(5)?;
                let created_at_str: String = row.get(6)?;

                let estimated_hours = Decimal::from_str(&est_str).unwrap_or(Decimal::ZERO);
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(ProjectTask {
                    id,
                    project_id,
                    title,
                    status,
                    priority,
                    estimated_hours,
                    created_at,
                })
            })?
        };

        let _ = ctx.record_and_publish_event(
            "ProjectTaskStatusUpdated",
            "ProjectTask",
            task_id,
            serde_json::to_value(&task).unwrap_or(serde_json::Value::Null),
            "system",
        )?;

        Ok(task)
    }

    /// List all project tasks, optionally filtered by project_id
    pub fn list_tasks(
        conn: &Connection,
        project_id: Option<&str>,
    ) -> Result<Vec<ProjectTask>, ProjectsError> {
        if let Some(pid) = project_id {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, title, status, priority, estimated_hours, created_at FROM project_tasks WHERE project_id = ?1 ORDER BY created_at ASC"
            )?;
            let rows = stmt.query_map(params![pid], Self::map_task_row)?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, project_id, title, status, priority, estimated_hours, created_at FROM project_tasks ORDER BY created_at ASC"
            )?;
            let rows = stmt.query_map([], Self::map_task_row)?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        }
    }

    /// Get unbilled hours summary: (total_hours, total_amount, entries_count)
    pub fn get_unbilled_summary(
        conn: &Connection,
        project_id: Option<&str>,
    ) -> Result<(Decimal, Decimal, u32), ProjectsError> {
        let mut total_hours = Decimal::ZERO;
        let mut total_amount = Decimal::ZERO;
        let mut count = 0;

        if let Some(pid) = project_id {
            let mut stmt = conn.prepare(
                "SELECT hours, billing_amount FROM timesheets WHERE project_id = ?1 AND is_billable = 1 AND is_invoiced = 0"
            )?;
            let rows = stmt.query_map(params![pid], |r| {
                let h: String = r.get(0)?;
                let a: String = r.get(1)?;
                Ok((h, a))
            })?;
            for r in rows {
                let (h, a) = r?;
                total_hours += Decimal::from_str(&h).unwrap_or(Decimal::ZERO);
                total_amount += Decimal::from_str(&a).unwrap_or(Decimal::ZERO);
                count += 1;
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT hours, billing_amount FROM timesheets WHERE is_billable = 1 AND is_invoiced = 0"
            )?;
            let rows = stmt.query_map([], |r| {
                let h: String = r.get(0)?;
                let a: String = r.get(1)?;
                Ok((h, a))
            })?;
            for r in rows {
                let (h, a) = r?;
                total_hours += Decimal::from_str(&h).unwrap_or(Decimal::ZERO);
                total_amount += Decimal::from_str(&a).unwrap_or(Decimal::ZERO);
                count += 1;
            }
        }

        Ok((total_hours, total_amount, count))
    }

    fn map_task_row(row: &Row<'_>) -> Result<ProjectTask, rusqlite::Error> {
        let id: String = row.get(0)?;
        let project_id: String = row.get(1)?;
        let title: String = row.get(2)?;
        let status: String = row.get(3)?;
        let priority: String = row.get(4)?;
        let est_str: String = row.get(5)?;
        let created_at_str: String = row.get(6)?;

        let estimated_hours = Decimal::from_str(&est_str).unwrap_or(Decimal::ZERO);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ProjectTask {
            id,
            project_id,
            title,
            status,
            priority,
            estimated_hours,
            created_at,
        })
    }

    fn map_project_row(row: &Row<'_>) -> Result<Project, rusqlite::Error> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let code: String = row.get(2)?;
        let customer_id: Option<String> = row.get(3)?;
        let status_str: String = row.get(4)?;
        let rate_str: String = row.get(5)?;
        let est_str: String = row.get(6)?;
        let created_at_str: String = row.get(7)?;

        let status = ProjectStatus::from_str(&status_str).unwrap_or(ProjectStatus::Open);
        let billing_rate_hourly = Decimal::from_str(&rate_str).unwrap_or(Decimal::ZERO);
        let estimated_hours = Decimal::from_str(&est_str).unwrap_or(Decimal::ZERO);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Project {
            id,
            name,
            code,
            customer_id,
            status,
            billing_rate_hourly,
            estimated_hours,
            created_at,
        })
    }

    fn map_timesheet_row(row: &Row<'_>) -> Result<TimesheetEntry, rusqlite::Error> {
        let id: String = row.get(0)?;
        let project_id: String = row.get(1)?;
        let task_id: Option<String> = row.get(2)?;
        let employee_or_user: String = row.get(3)?;
        let activity_date: String = row.get(4)?;
        let hours_str: String = row.get(5)?;
        let rate_str: String = row.get(6)?;
        let bill_amount_str: String = row.get(7)?;
        let is_billable_int: i32 = row.get(8)?;
        let is_invoiced_int: i32 = row.get(9)?;
        let invoice_id: Option<String> = row.get(10)?;
        let description: String = row.get(11)?;
        let created_at_str: String = row.get(12)?;

        let hours = Decimal::from_str(&hours_str).unwrap_or(Decimal::ZERO);
        let hourly_rate = Decimal::from_str(&rate_str).unwrap_or(Decimal::ZERO);
        let billing_amount = Decimal::from_str(&bill_amount_str).unwrap_or(Decimal::ZERO);
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(TimesheetEntry {
            id,
            project_id,
            task_id,
            employee_or_user,
            activity_date,
            hours,
            hourly_rate,
            billing_amount,
            is_billable: is_billable_int != 0,
            is_invoiced: is_invoiced_int != 0,
            invoice_id,
            description,
            created_at,
        })
    }
}
