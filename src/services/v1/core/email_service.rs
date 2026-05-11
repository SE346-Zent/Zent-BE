use std::collections::HashMap;
use std::sync::Arc;
use crate::core::errors::AppError;
use lapin::Connection;
use crate::services::v1::core::helpers::mq::publish_email_task;

pub async fn send_verification_email(
    rabbitmq: &Arc<Connection>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    // Escape variables for secure HTML rendering
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_code = v_htmlescape::escape(code).to_string();

    // 1. Prepare email template from cache
    let email_body = if let Some(template_content) = templates.get("verification_email.html") {
        template_content
            .replace("{{code}}", &escaped_code)
            .replace("{{name}}", &escaped_name)
    } else {
        tracing::warn!("Template 'verification_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Welcome to Zent, {}!</h2><p>Your verification code is: <strong style='color:#007bff; font-size:24px;'>{}</strong></p></body></html>", 
            escaped_name, escaped_code
        )
    };

    // 2. Deliver async email task to RabbitMQ
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Zent Account Verification",
        "body": email_body
    });
    
    publish_email_task(rabbitmq, email_payload, "verification email").await
}

pub async fn send_forgot_password_email(
    rabbitmq: &Arc<Connection>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    code: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_code = v_htmlescape::escape(code).to_string();

    let email_body = if let Some(template_content) = templates.get("forgot_password_email.html") {
        template_content
            .replace("{{code}}", &escaped_code)
            .replace("{{name}}", &escaped_name)
    } else {
        tracing::warn!("Template 'forgot_password_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Reset Your Password, {}</h2><p>Your password reset code is: <strong style='color:#dc3545; font-size:24px;'>{}</strong></p></body></html>", 
            escaped_name, escaped_code
        )
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Zent Password Reset Request",
        "body": email_body
    });
    
    publish_email_task(rabbitmq, email_payload, "reset email").await
}

pub async fn send_welcome_email(
    rabbitmq: &Arc<Connection>,
    _templates: &HashMap<String, String>,
    to: &str,
    name: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let email_payload = serde_json::json!({
        "to": to,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Your account has been successfully created.", escaped_name)
    });
    
    publish_email_task(rabbitmq, email_payload, "welcome email").await
}

pub async fn send_work_order_created_email(
    rabbitmq: &Arc<Connection>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    work_order_number: &str,
    service_type: &str,
    appointment: &str,
    address: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();
    let escaped_service = v_htmlescape::escape(service_type).to_string();
    let escaped_appointment = v_htmlescape::escape(appointment).to_string();
    let escaped_address = v_htmlescape::escape(address).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_created_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
            .replace("{{service_type}}", &escaped_service)
            .replace("{{appointment}}", &escaped_appointment)
            .replace("{{address}}", &escaped_address)
    } else {
        tracing::warn!("Template 'work_order_created_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Created, {}!</h2><p>Your work order number is: <strong>{}</strong></p><p>Service: {}</p><p>Appointment: {}</p><p>Address: {}</p></body></html>",
            escaped_name, escaped_wo_number, escaped_service, escaped_appointment, escaped_address
        )
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": format!("Work Order Created: {}", work_order_number),
        "body": email_body
    });
    
    publish_email_task(rabbitmq, email_payload, "work order creation email").await
}

pub async fn send_work_order_refusal_denied_email(
    rabbitmq: &Arc<Connection>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    work_order_number: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_refusal_denied_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
    } else {
        tracing::warn!("Template 'work_order_refusal_denied_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Update, {}!</h2><p>We regret to inform you that the technician's refusal for work order <strong>{}</strong> has been declined. Your work order remains active and will be reassigned.</p><p>We apologize for any inconvenience.</p></body></html>",
            escaped_name, escaped_wo_number
        )
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": format!("Work Order Update: {}", work_order_number),
        "body": email_body
    });

    publish_email_task(rabbitmq, email_payload, "work order refusal denied email").await
}

pub async fn send_work_order_assigned_email(
    rabbitmq: &Arc<Connection>,
    templates: &HashMap<String, String>,
    to: &str,
    name: &str,
    work_order_number: &str,
    technician_name: &str,
    appointment: &str,
) -> Result<(), AppError> {
    let escaped_name = v_htmlescape::escape(name).to_string();
    let escaped_wo_number = v_htmlescape::escape(work_order_number).to_string();
    let escaped_tech_name = v_htmlescape::escape(technician_name).to_string();
    let escaped_appointment = v_htmlescape::escape(appointment).to_string();

    let email_body = if let Some(template_content) = templates.get("work_order_assigned_email.html") {
        template_content
            .replace("{{name}}", &escaped_name)
            .replace("{{work_order_number}}", &escaped_wo_number)
            .replace("{{technician_name}}", &escaped_tech_name)
            .replace("{{appointment}}", &escaped_appointment)
    } else {
        tracing::warn!("Template 'work_order_assigned_email.html' not found in cache! Using minimal HTML fallback.");
        format!(
            "<html><body><h2>Work Order Assigned, {}!</h2><p>Your work order number is: <strong>{}</strong></p><p>Technician: {}</p><p>Appointment: {}</p></body></html>",
            escaped_name, escaped_wo_number, escaped_tech_name, escaped_appointment
        )
    };

    let email_payload = serde_json::json!({
        "to": to,
        "subject": format!("Work Order Assigned: {}", work_order_number),
        "body": email_body
    });
    
    publish_email_task(rabbitmq, email_payload, "work order assigned email").await
}
